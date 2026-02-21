//! Trilateral cycle detection and compression example.
//!
//! Demonstrates how the engine detects circular payment flows
//! and computes the maximum amount that can be compressed.

use clearing_engine::core::currency::CurrencyCode;
use clearing_engine::core::obligation::Obligation;
use clearing_engine::core::party::PartyId;
use clearing_engine::graph::cycle_detection::find_cycles;
use clearing_engine::graph::payment_graph::PaymentGraph;
use clearing_engine::graph::scc::find_sccs;
use rust_decimal_macros::dec;

fn main() {
    println!("╔═══════════════════════════════════════════════╗");
    println!("║  clearing-engine: Trilateral Cycle Detection  ║");
    println!("╚═══════════════════════════════════════════════╝\n");

    let mut graph = PaymentGraph::new();
    let usd = CurrencyCode::new("USD");

    let brazil = PartyId::new("BR-TREASURY");
    let india = PartyId::new("IN-RBI");
    let china = PartyId::new("CN-PBOC");

    // Classic trilateral cycle with different amounts
    println!("Obligations:");
    println!("  Brazil → India:  $100M");
    println!("  India  → China:  $80M");
    println!("  China  → Brazil: $120M\n");

    graph.add_obligation(Obligation::new(
        brazil.clone(),
        india.clone(),
        dec!(100_000_000),
        usd.clone(),
    ));
    graph.add_obligation(Obligation::new(
        india.clone(),
        china.clone(),
        dec!(80_000_000),
        usd.clone(),
    ));
    graph.add_obligation(Obligation::new(
        china.clone(),
        brazil.clone(),
        dec!(120_000_000),
        usd.clone(),
    ));

    // Find strongly connected components
    println!("━━━ Strongly Connected Components ━━━\n");
    let sccs = find_sccs(&graph, &usd);
    for (i, scc) in sccs.iter().enumerate() {
        let parties: Vec<String> = scc.parties.iter().map(|p| p.to_string()).collect();
        println!(
            "  SCC {}: [{}] — nettable: {}",
            i,
            parties.join(", "),
            scc.is_nettable()
        );
    }
    println!();

    // Find cycles
    println!("━━━ Payment Cycles ━━━\n");
    let cycles = find_cycles(&graph, &usd);
    for (i, cycle) in cycles.iter().enumerate() {
        let parties: Vec<String> = cycle.parties.iter().map(|p| p.to_string()).collect();
        println!("  Cycle {}: {} → (back to start)", i, parties.join(" → "));
        println!("    Bottleneck:         ${}", cycle.bottleneck);
        println!("    Potential savings:  ${}", cycle.potential_savings());
        println!();
    }

    // Netting result
    println!("━━━ Netting Result ━━━\n");
    let result = graph.compute_net_positions();
    println!("  Gross total:    ${}", result.gross_total());
    println!("  Net total:      ${}", result.net_total());
    println!("  Savings:        ${}", result.savings());
    println!("  Savings:        {:.1}%", result.savings_percent());
    println!("  Valid:          {}", result.is_valid());

    println!("\n━━━ Interpretation ━━━\n");
    println!("  The bottleneck of $80M can circulate through the cycle without");
    println!("  any party needing to fund it. Only the residual amounts need");
    println!("  actual liquidity to settle.");
}
