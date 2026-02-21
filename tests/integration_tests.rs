use clearing_engine::core::currency::CurrencyCode;
use clearing_engine::core::obligation::{Obligation, ObligationSet};
use clearing_engine::core::party::PartyId;
use clearing_engine::graph::cycle_detection::find_cycles;
use clearing_engine::graph::payment_graph::PaymentGraph;
use clearing_engine::graph::scc::find_sccs;
use clearing_engine::optimization::liquidity::LiquidityAnalysis;
use clearing_engine::optimization::netting::NettingEngine;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Full pipeline test: obligations → graph → cycles → netting → liquidity.
#[test]
fn full_pipeline_brics_scenario() {
    let mut set = ObligationSet::new();
    let usd = CurrencyCode::new("USD");

    let brazil = PartyId::new("BR-TREASURY");
    let india = PartyId::new("IN-RBI");
    let china = PartyId::new("CN-PBOC");
    let russia = PartyId::new("RU-CBR");
    let south_africa = PartyId::new("ZA-SARB");

    set.add(Obligation::new(brazil.clone(), india.clone(), dec!(100_000_000), usd.clone()));
    set.add(Obligation::new(india.clone(), china.clone(), dec!(80_000_000), usd.clone()));
    set.add(Obligation::new(china.clone(), russia.clone(), dec!(120_000_000), usd.clone()));
    set.add(Obligation::new(russia.clone(), brazil.clone(), dec!(90_000_000), usd.clone()));
    set.add(Obligation::new(south_africa.clone(), india.clone(), dec!(40_000_000), usd.clone()));
    set.add(Obligation::new(china.clone(), brazil.clone(), dec!(70_000_000), usd.clone()));
    set.add(Obligation::new(india.clone(), russia.clone(), dec!(30_000_000), usd.clone()));
    set.add(Obligation::new(russia.clone(), south_africa.clone(), dec!(25_000_000), usd.clone()));

    // Verify obligation set
    assert_eq!(set.len(), 8);
    assert_eq!(set.gross_total(), dec!(555_000_000));

    // Build graph
    let mut graph = PaymentGraph::new();
    for ob in set.obligations() {
        graph.add_obligation(ob.clone());
    }
    assert_eq!(graph.party_count(), 5);
    assert_eq!(graph.currency_count(), 1);

    // Find SCCs
    let sccs = find_sccs(&graph, &usd);
    let nettable: Vec<_> = sccs.iter().filter(|s| s.is_nettable()).collect();
    assert!(!nettable.is_empty(), "Should find at least one nettable SCC");

    // Find cycles
    let cycles = find_cycles(&graph, &usd);
    assert!(!cycles.is_empty(), "Should find at least one cycle");
    for cycle in &cycles {
        assert!(cycle.bottleneck > Decimal::ZERO);
        assert!(cycle.potential_savings() > Decimal::ZERO);
    }

    // Run netting
    let result = NettingEngine::multilateral_net(&set);
    assert!(result.is_valid());
    assert!(result.net_total() < result.gross_total());
    assert!(result.savings_percent() > 0.0);

    // Verify specific positions
    let br_pos = result.net_position(&brazil, &usd);
    let in_pos = result.net_position(&india, &usd);
    let cn_pos = result.net_position(&china, &usd);
    let ru_pos = result.net_position(&russia, &usd);
    let za_pos = result.net_position(&south_africa, &usd);

    // Positions must sum to zero
    assert_eq!(br_pos + in_pos + cn_pos + ru_pos + za_pos, Decimal::ZERO);

    // Liquidity analysis
    let liquidity = LiquidityAnalysis::from_netting_result(&result);
    assert!(liquidity.net_requirement <= liquidity.gross_requirement);
    assert!(liquidity.savings_ratio() >= 0.0);
    assert!(liquidity.savings_ratio() <= 1.0);
}

/// Test JSON serialization round-trip for obligations.
#[test]
fn obligation_json_round_trip() {
    let ob = Obligation::new(
        PartyId::new("BR-TREASURY"),
        PartyId::new("IN-RBI"),
        dec!(100_000_000),
        CurrencyCode::new("USD"),
    );

    let json = serde_json::to_string(&ob).unwrap();
    let deserialized: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized["debtor"], "BR-TREASURY");
    assert_eq!(deserialized["creditor"], "IN-RBI");
    assert_eq!(deserialized["currency"], "USD");
}

/// Test JSON serialization of netting results.
#[test]
fn netting_result_serializes() {
    let mut set = ObligationSet::new();
    let usd = CurrencyCode::new("USD");
    set.add(Obligation::new(
        PartyId::new("A"), PartyId::new("B"), dec!(100), usd.clone(),
    ));
    set.add(Obligation::new(
        PartyId::new("B"), PartyId::new("A"), dec!(60), usd,
    ));

    let result = NettingEngine::multilateral_net(&set);
    let json = serde_json::to_string_pretty(&result).unwrap();

    // Verify it's valid JSON and contains expected fields
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.get("gross_total").is_some());
    assert!(parsed.get("net_total").is_some());
    assert!(parsed.get("ledger").is_some());
}

/// Test that an empty obligation set produces valid zero results.
#[test]
fn empty_set_produces_valid_zero() {
    let set = ObligationSet::new();
    let result = NettingEngine::multilateral_net(&set);

    assert_eq!(result.gross_total(), Decimal::ZERO);
    assert_eq!(result.net_total(), Decimal::ZERO);
    assert_eq!(result.savings(), Decimal::ZERO);
    assert!(result.is_valid());

    let json = serde_json::to_string(&result).unwrap();
    assert!(!json.is_empty());
}

/// Test multi-currency netting keeps currencies independent.
#[test]
fn multi_currency_independence() {
    let mut set = ObligationSet::new();
    let usd = CurrencyCode::new("USD");
    let brl = CurrencyCode::new("BRL");

    // USD: perfect cycle → nets to zero
    set.add(Obligation::new(PartyId::new("A"), PartyId::new("B"), dec!(100), usd.clone()));
    set.add(Obligation::new(PartyId::new("B"), PartyId::new("A"), dec!(100), usd.clone()));

    // BRL: one-way → nets to full amount
    set.add(Obligation::new(PartyId::new("A"), PartyId::new("B"), dec!(500), brl.clone()));

    let result = NettingEngine::multilateral_net(&set);
    assert!(result.is_valid());

    // A's USD position should be zero (cycle cancels)
    assert_eq!(result.net_position(&PartyId::new("A"), &usd), Decimal::ZERO);

    // A's BRL position should be -500 (owes)
    assert_eq!(result.net_position(&PartyId::new("A"), &brl), dec!(-500));

    // Currency breakdown should reflect this
    let usd_breakdown = &result.currency_breakdown()[&usd];
    assert_eq!(usd_breakdown.net_total, Decimal::ZERO);

    let brl_breakdown = &result.currency_breakdown()[&brl];
    assert_eq!(brl_breakdown.net_total, dec!(500));
}
