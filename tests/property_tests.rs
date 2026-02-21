use clearing_engine::core::currency::CurrencyCode;
use clearing_engine::core::obligation::{Obligation, ObligationSet};
use clearing_engine::core::party::PartyId;
use clearing_engine::graph::cycle_detection::find_cycles;
use clearing_engine::graph::payment_graph::PaymentGraph;
use clearing_engine::optimization::liquidity::LiquidityAnalysis;
use clearing_engine::optimization::netting::NettingEngine;
use proptest::prelude::*;
use rust_decimal::Decimal;

/// Generate a random party ID from a small pool (to increase cycle probability).
fn arb_party() -> impl Strategy<Value = PartyId> {
    prop::sample::select(vec![
        PartyId::new("A"),
        PartyId::new("B"),
        PartyId::new("C"),
        PartyId::new("D"),
        PartyId::new("E"),
        PartyId::new("F"),
    ])
}

/// Generate a random currency from a small pool.
fn arb_currency() -> impl Strategy<Value = CurrencyCode> {
    prop::sample::select(vec![
        CurrencyCode::new("USD"),
        CurrencyCode::new("BRL"),
        CurrencyCode::new("INR"),
    ])
}

/// Generate a random positive amount (1 to 10,000,000).
fn arb_amount() -> impl Strategy<Value = Decimal> {
    (1u64..10_000_000u64).prop_map(Decimal::from)
}

/// Generate a random obligation (ensuring debtor != creditor).
fn arb_obligation() -> impl Strategy<Value = Obligation> {
    (arb_party(), arb_party(), arb_amount(), arb_currency()).prop_filter_map(
        "debtor must differ from creditor",
        |(debtor, creditor, amount, currency)| {
            if debtor == creditor {
                None
            } else {
                Some(Obligation::new(debtor, creditor, amount, currency))
            }
        },
    )
}

/// Generate a random obligation set of 1..50 obligations.
fn arb_obligation_set() -> impl Strategy<Value = ObligationSet> {
    prop::collection::vec(arb_obligation(), 1..50)
        .prop_map(|obs| obs.into_iter().collect::<ObligationSet>())
}

proptest! {
    // ===================================================================
    // INVARIANT 1: Ledger always balances to zero per currency.
    //
    // For any set of obligations, the sum of all net positions in each
    // currency must be exactly zero. Credits and debits are conserved.
    // ===================================================================
    #[test]
    fn ledger_always_balances(set in arb_obligation_set()) {
        let result = NettingEngine::multilateral_net(&set);
        prop_assert!(
            result.is_valid(),
            "Ledger must be balanced: every credit has a matching debit"
        );
    }

    // ===================================================================
    // INVARIANT 2: Net settlement ≤ gross settlement. Always.
    //
    // Netting can only reduce (or maintain) the total settlement amount.
    // It can never increase it.
    // ===================================================================
    #[test]
    fn net_never_exceeds_gross(set in arb_obligation_set()) {
        let result = NettingEngine::multilateral_net(&set);
        prop_assert!(
            result.net_total() <= result.gross_total(),
            "Net {} must be ≤ gross {}",
            result.net_total(),
            result.gross_total()
        );
    }

    // ===================================================================
    // INVARIANT 3: Savings percentage is always 0–100%.
    //
    // Cannot save more than 100% and cannot have negative savings.
    // ===================================================================
    #[test]
    fn savings_in_valid_range(set in arb_obligation_set()) {
        let result = NettingEngine::multilateral_net(&set);
        let pct = result.savings_percent();
        prop_assert!(
            pct >= 0.0 && pct <= 100.0,
            "Savings percent {} must be in [0, 100]",
            pct
        );
    }

    // ===================================================================
    // INVARIANT 4: Gross total equals sum of obligation amounts.
    //
    // The gross total reported by netting must exactly match the sum
    // of all individual obligation amounts.
    // ===================================================================
    #[test]
    fn gross_matches_obligation_sum(set in arb_obligation_set()) {
        let result = NettingEngine::multilateral_net(&set);
        let manual_sum: Decimal = set.obligations().iter().map(|o| o.amount()).sum();
        prop_assert_eq!(
            result.gross_total(),
            manual_sum,
            "Gross total must equal sum of obligations"
        );
    }

    // ===================================================================
    // INVARIANT 5: Netting is deterministic.
    //
    // Running the same obligations through netting twice must produce
    // identical results. No randomness, no hidden state.
    // ===================================================================
    #[test]
    fn netting_is_deterministic(set in arb_obligation_set()) {
        let result1 = NettingEngine::multilateral_net(&set);
        let result2 = NettingEngine::multilateral_net(&set);
        prop_assert_eq!(result1.gross_total(), result2.gross_total());
        prop_assert_eq!(result1.net_total(), result2.net_total());
    }

    // ===================================================================
    // INVARIANT 6: Liquidity requirement equals net settlement.
    //
    // The total liquidity needed by all debtors must equal the net
    // settlement total (which equals total owed to creditors).
    // ===================================================================
    #[test]
    fn liquidity_matches_net(set in arb_obligation_set()) {
        let result = NettingEngine::multilateral_net(&set);
        let liquidity = LiquidityAnalysis::from_netting_result(&result);
        prop_assert_eq!(
            liquidity.net_requirement,
            result.net_total(),
            "Liquidity net requirement must equal netting net total"
        );
    }

    // ===================================================================
    // INVARIANT 7: Per-currency breakdown sums to totals.
    //
    // The sum of gross totals across all currency breakdowns must
    // equal the overall gross total.
    // ===================================================================
    #[test]
    fn currency_breakdown_sums_correctly(set in arb_obligation_set()) {
        let result = NettingEngine::multilateral_net(&set);
        let breakdown_gross: Decimal = result
            .currency_breakdown()
            .values()
            .map(|c| c.gross_total)
            .sum();
        prop_assert_eq!(
            breakdown_gross,
            result.gross_total(),
            "Currency breakdown gross must sum to total gross"
        );
    }

    // ===================================================================
    // INVARIANT 8: Cycle bottleneck ≤ minimum edge in cycle.
    //
    // The bottleneck of any detected cycle cannot exceed the smallest
    // edge weight in that cycle.
    // ===================================================================
    #[test]
    fn cycle_bottleneck_valid(set in arb_obligation_set()) {
        let mut graph = PaymentGraph::new();
        for ob in set.obligations() {
            graph.add_obligation(ob.clone());
        }

        for currency in graph.currencies() {
            let cycles = find_cycles(&graph, currency);
            for cycle in &cycles {
                // Verify bottleneck is ≤ every edge in the cycle
                for i in 0..cycle.parties.len() {
                    let from = &cycle.parties[i];
                    let to = &cycle.parties[(i + 1) % cycle.parties.len()];
                    let edge_amount = graph.edge_amount(from, to, currency);
                    prop_assert!(
                        cycle.bottleneck <= edge_amount,
                        "Bottleneck {} must be ≤ edge {} → {} amount {}",
                        cycle.bottleneck,
                        from,
                        to,
                        edge_amount
                    );
                }
            }
        }
    }

    // ===================================================================
    // INVARIANT 9: Perfect bilateral pair nets to difference.
    //
    // When only two parties trade in one currency, the net settlement
    // must equal |A_owes_B - B_owes_A|.
    // ===================================================================
    #[test]
    fn bilateral_nets_to_difference(
        a_to_b in 1u64..10_000_000u64,
        b_to_a in 1u64..10_000_000u64,
    ) {
        let mut set = ObligationSet::new();
        let usd = CurrencyCode::new("USD");
        set.add(Obligation::new(
            PartyId::new("A"), PartyId::new("B"),
            Decimal::from(a_to_b), usd.clone(),
        ));
        set.add(Obligation::new(
            PartyId::new("B"), PartyId::new("A"),
            Decimal::from(b_to_a), usd,
        ));
        let result = NettingEngine::multilateral_net(&set);
        let expected_net = (Decimal::from(a_to_b) - Decimal::from(b_to_a)).abs();
        prop_assert_eq!(
            result.net_total(),
            expected_net,
            "Bilateral net must equal |{} - {}| = {}",
            a_to_b, b_to_a, expected_net
        );
    }
}
