//! Basic bilateral and multilateral netting example.
//!
//! Demonstrates how the clearing engine reduces gross settlement
//! requirements through netting.

use clearing_engine::core::currency::CurrencyCode;
use clearing_engine::core::obligation::{Obligation, ObligationSet};
use clearing_engine::core::party::PartyId;
use clearing_engine::optimization::liquidity::LiquidityAnalysis;
use clearing_engine::optimization::netting::NettingEngine;
use rust_decimal_macros::dec;

fn main() {
    println!("╔══════════════════════════════════════════╗");
    println!("║  clearing-engine: Basic Netting Example  ║");
    println!("╚══════════════════════════════════════════╝\n");

    // --- Scenario 1: Bilateral netting ---
    println!("━━━ Scenario 1: Bilateral Netting ━━━\n");

    let mut set = ObligationSet::new();
    let usd = CurrencyCode::new("USD");
    let brazil = PartyId::new("BR-TREASURY");
    let india = PartyId::new("IN-RBI");

    set.add(Obligation::new(
        brazil.clone(),
        india.clone(),
        dec!(100_000_000),
        usd.clone(),
    ));
    set.add(Obligation::new(
        india.clone(),
        brazil.clone(),
        dec!(65_000_000),
        usd.clone(),
    ));

    let bilateral = NettingEngine::bilateral_net(&set, &brazil, &india, &usd);

    println!("Brazil owes India:  ${}", bilateral.gross_a_to_b);
    println!("India owes Brazil:  ${}", bilateral.gross_b_to_a);
    println!("Net (A→B):          ${}", bilateral.net_amount);
    println!("Savings:            ${}", bilateral.savings);
    println!();

    // --- Scenario 2: Multilateral netting ---
    println!("━━━ Scenario 2: Multilateral Netting (5 parties) ━━━\n");

    let mut set = ObligationSet::new();
    let china = PartyId::new("CN-PBOC");
    let russia = PartyId::new("RU-CBR");
    let south_africa = PartyId::new("ZA-SARB");

    // Create a realistic web of obligations
    set.add(Obligation::new(brazil.clone(), india.clone(), dec!(100_000_000), usd.clone()));
    set.add(Obligation::new(india.clone(), china.clone(), dec!(80_000_000), usd.clone()));
    set.add(Obligation::new(china.clone(), russia.clone(), dec!(120_000_000), usd.clone()));
    set.add(Obligation::new(russia.clone(), brazil.clone(), dec!(90_000_000), usd.clone()));
    set.add(Obligation::new(south_africa.clone(), india.clone(), dec!(40_000_000), usd.clone()));
    set.add(Obligation::new(china.clone(), brazil.clone(), dec!(70_000_000), usd.clone()));
    set.add(Obligation::new(india.clone(), russia.clone(), dec!(30_000_000), usd.clone()));
    set.add(Obligation::new(russia.clone(), south_africa.clone(), dec!(25_000_000), usd.clone()));

    let result = NettingEngine::multilateral_net(&set);

    println!("{}", result);

    // Liquidity analysis
    let liquidity = LiquidityAnalysis::from_netting_result(&result);
    println!("{}", liquidity);

    // Show individual net positions
    println!("━━━ Net Positions ━━━\n");
    for party in [&brazil, &india, &china, &russia, &south_africa] {
        let pos = result.net_position(party, &usd);
        let status = if pos > dec!(0) {
            "CREDITOR"
        } else if pos < dec!(0) {
            "DEBTOR"
        } else {
            "FLAT"
        };
        println!("  {:<15} {:>15} USD  [{}]", party, pos, status);
    }
}
