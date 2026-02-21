//! clearing-engine CLI
//!
//! Run multi-currency netting from the command line.
//!
//! # Usage
//!
//! ```bash
//! # Net obligations from a JSON file
//! clearing-engine net --input obligations.json
//!
//! # Output as JSON
//! clearing-engine net --input obligations.json --format json
//!
//! # Analyze cycles
//! clearing-engine cycles --input obligations.json
//!
//! # Generate a random network for testing
//! clearing-engine generate --parties 10 --obligations 30
//! ```

use clearing_engine::core::currency::CurrencyCode;
use clearing_engine::core::obligation::{Obligation, ObligationSet};
use clearing_engine::core::party::PartyId;
use clearing_engine::graph::cycle_detection::find_cycles;
use clearing_engine::graph::payment_graph::PaymentGraph;
use clearing_engine::optimization::liquidity::LiquidityAnalysis;
use clearing_engine::optimization::netting::NettingEngine;
use clearing_engine::simulation::stress_test::{generate_random_network, NetworkConfig};
use rust_decimal::Decimal;
use std::fs;
use std::process;

fn print_usage() {
    eprintln!(
        r#"clearing-engine — open multi-currency clearing and liquidity optimization

USAGE:
    clearing-engine <COMMAND> [OPTIONS]

COMMANDS:
    net         Run multilateral netting on an obligation set
    cycles      Detect payment cycles in the obligation graph
    generate    Generate a random obligation network (for testing)
    help        Show this message

OPTIONS (net, cycles):
    --input <FILE>      Path to JSON obligations file
    --format <FORMAT>   Output format: text (default) or json

OPTIONS (generate):
    --parties <N>       Number of parties (default: 10)
    --obligations <N>   Number of obligations (default: 30)
    --currencies <LIST> Comma-separated currency codes (default: USD)
    --output <FILE>     Write to file instead of stdout

EXAMPLES:
    clearing-engine net --input obligations.json
    clearing-engine net --input obligations.json --format json
    clearing-engine cycles --input obligations.json
    clearing-engine generate --parties 20 --obligations 60
    clearing-engine generate --parties 5 --currencies USD,BRL,INR --output test.json"#
    );
}

/// JSON schema for input obligations.
#[derive(serde::Deserialize)]
struct ObligationInput {
    from: String,
    to: String,
    amount: String,
    #[serde(default = "default_currency")]
    currency: String,
}

fn default_currency() -> String {
    "USD".to_string()
}

#[derive(serde::Deserialize)]
struct ObligationsFile {
    obligations: Vec<ObligationInput>,
}

/// JSON output schema for netting results.
#[derive(serde::Serialize)]
struct NettingOutput {
    gross_total: String,
    net_total: String,
    savings: String,
    savings_percent: f64,
    valid: bool,
    positions: Vec<PositionOutput>,
}

#[derive(serde::Serialize)]
struct PositionOutput {
    party: String,
    currency: String,
    net_position: String,
    status: String,
}

#[derive(serde::Serialize)]
struct CycleOutput {
    parties: Vec<String>,
    currency: String,
    bottleneck: String,
    potential_savings: String,
}

fn load_obligations(path: &str) -> ObligationSet {
    let content = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Error reading file '{}': {}", path, e);
        process::exit(1);
    });

    let file: ObligationsFile = serde_json::from_str(&content).unwrap_or_else(|e| {
        eprintln!("Error parsing JSON: {}", e);
        eprintln!("Expected format:");
        eprintln!(r#"{{
  "obligations": [
    {{ "from": "BR-TREASURY", "to": "IN-RBI", "amount": "100000000", "currency": "USD" }}
  ]
}}"#);
        process::exit(1);
    });

    let mut set = ObligationSet::new();
    for ob in file.obligations {
        let amount: Decimal = ob.amount.parse().unwrap_or_else(|e| {
            eprintln!("Invalid amount '{}': {}", ob.amount, e);
            process::exit(1);
        });
        set.add(Obligation::new(
            PartyId::new(&ob.from),
            PartyId::new(&ob.to),
            amount,
            CurrencyCode::new(&ob.currency),
        ));
    }
    set
}

fn cmd_net(args: &[String]) {
    let mut input_path = None;
    let mut format = "text".to_string();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--input" => {
                i += 1;
                input_path = Some(args.get(i).cloned().unwrap_or_else(|| {
                    eprintln!("--input requires a file path");
                    process::exit(1);
                }));
            }
            "--format" => {
                i += 1;
                format = args.get(i).cloned().unwrap_or_else(|| {
                    eprintln!("--format requires 'text' or 'json'");
                    process::exit(1);
                });
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                process::exit(1);
            }
        }
        i += 1;
    }

    let path = input_path.unwrap_or_else(|| {
        eprintln!("Error: --input <FILE> is required");
        process::exit(1);
    });

    let set = load_obligations(&path);
    let result = NettingEngine::multilateral_net(&set);

    if format == "json" {
        let mut positions = Vec::new();
        for ((party, currency), amount) in result.ledger().all_positions() {
            if *amount != Decimal::ZERO {
                positions.push(PositionOutput {
                    party: party.to_string(),
                    currency: currency.to_string(),
                    net_position: amount.to_string(),
                    status: if *amount > Decimal::ZERO {
                        "CREDITOR".to_string()
                    } else {
                        "DEBTOR".to_string()
                    },
                });
            }
        }
        positions.sort_by(|a, b| a.party.cmp(&b.party));

        let output = NettingOutput {
            gross_total: result.gross_total().to_string(),
            net_total: result.net_total().to_string(),
            savings: result.savings().to_string(),
            savings_percent: result.savings_percent(),
            valid: result.is_valid(),
            positions,
        };

        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        println!("{}", result);

        let liquidity = LiquidityAnalysis::from_netting_result(&result);
        println!("{}", liquidity);
    }
}

fn cmd_cycles(args: &[String]) {
    let mut input_path = None;
    let mut format = "text".to_string();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--input" => {
                i += 1;
                input_path = Some(args.get(i).cloned().unwrap_or_else(|| {
                    eprintln!("--input requires a file path");
                    process::exit(1);
                }));
            }
            "--format" => {
                i += 1;
                format = args.get(i).cloned().unwrap_or_else(|| {
                    eprintln!("--format requires 'text' or 'json'");
                    process::exit(1);
                });
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                process::exit(1);
            }
        }
        i += 1;
    }

    let path = input_path.unwrap_or_else(|| {
        eprintln!("Error: --input <FILE> is required");
        process::exit(1);
    });

    let set = load_obligations(&path);
    let mut graph = PaymentGraph::new();
    for ob in set.obligations() {
        graph.add_obligation(ob.clone());
    }

    if format == "json" {
        let mut all_cycles = Vec::new();
        for currency in graph.currencies() {
            let cycles = find_cycles(&graph, currency);
            for cycle in cycles {
                all_cycles.push(CycleOutput {
                    parties: cycle.parties.iter().map(|p| p.to_string()).collect(),
                    currency: currency.to_string(),
                    bottleneck: cycle.bottleneck.to_string(),
                    potential_savings: cycle.potential_savings().to_string(),
                });
            }
        }
        println!("{}", serde_json::to_string_pretty(&all_cycles).unwrap());
    } else {
        let mut total_cycles = 0;
        for currency in graph.currencies() {
            let cycles = find_cycles(&graph, currency);
            if !cycles.is_empty() {
                println!("Currency: {}", currency);
                for (i, cycle) in cycles.iter().enumerate() {
                    let parties: Vec<String> =
                        cycle.parties.iter().map(|p| p.to_string()).collect();
                    println!(
                        "  Cycle {}: {} → (back to start)",
                        i,
                        parties.join(" → ")
                    );
                    println!("    Bottleneck:        {}", cycle.bottleneck);
                    println!("    Potential savings: {}", cycle.potential_savings());
                }
                total_cycles += cycles.len();
            }
        }
        if total_cycles == 0 {
            println!("No cycles detected.");
        } else {
            println!("\nTotal cycles: {}", total_cycles);
        }
    }
}

fn cmd_generate(args: &[String]) {
    let mut parties = 10usize;
    let mut obligations_count = 30usize;
    let mut currencies_str = "USD".to_string();
    let mut output_path: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--parties" => {
                i += 1;
                parties = args
                    .get(i)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| {
                        eprintln!("--parties requires a number");
                        process::exit(1);
                    });
            }
            "--obligations" => {
                i += 1;
                obligations_count = args
                    .get(i)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| {
                        eprintln!("--obligations requires a number");
                        process::exit(1);
                    });
            }
            "--currencies" => {
                i += 1;
                currencies_str = args.get(i).cloned().unwrap_or_else(|| {
                    eprintln!("--currencies requires a comma-separated list");
                    process::exit(1);
                });
            }
            "--output" => {
                i += 1;
                output_path = Some(args.get(i).cloned().unwrap_or_else(|| {
                    eprintln!("--output requires a file path");
                    process::exit(1);
                }));
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                process::exit(1);
            }
        }
        i += 1;
    }

    let currencies: Vec<CurrencyCode> = currencies_str
        .split(',')
        .map(|s| CurrencyCode::new(s.trim()))
        .collect();

    let config = NetworkConfig {
        party_count: parties,
        currencies,
        avg_obligations_per_party: obligations_count / parties.max(1),
        ..Default::default()
    };

    let set = generate_random_network(&config);

    #[derive(serde::Serialize)]
    struct OutputObligation {
        from: String,
        to: String,
        amount: String,
        currency: String,
    }

    #[derive(serde::Serialize)]
    struct OutputFile {
        obligations: Vec<OutputObligation>,
    }

    let output = OutputFile {
        obligations: set
            .obligations()
            .iter()
            .map(|ob| OutputObligation {
                from: ob.debtor().to_string(),
                to: ob.creditor().to_string(),
                amount: ob.amount().to_string(),
                currency: ob.currency().to_string(),
            })
            .collect(),
    };

    let json = serde_json::to_string_pretty(&output).unwrap();

    if let Some(path) = output_path {
        fs::write(&path, &json).unwrap_or_else(|e| {
            eprintln!("Error writing to '{}': {}", path, e);
            process::exit(1);
        });
        eprintln!(
            "Generated {} obligations across {} parties → {}",
            set.len(),
            parties,
            path
        );
    } else {
        println!("{}", json);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let command = args[1].as_str();
    let rest = &args[2..];

    match command {
        "net" => cmd_net(rest),
        "cycles" => cmd_cycles(rest),
        "generate" => cmd_generate(rest),
        "help" | "--help" | "-h" => print_usage(),
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage();
            process::exit(1);
        }
    }
}
