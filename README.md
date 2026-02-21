# clearing-engine

**Open multi-currency clearing and liquidity optimization engine.**

[![CI](https://github.com/OpenSettlement/clearing-engine/actions/workflows/ci.yml/badge.svg)](https://github.com/OpenSettlement/clearing-engine/actions)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/clearing-engine.svg)](https://crates.io/crates/clearing-engine)

---

## The Problem

Cross-border settlement is expensive. When multiple parties owe each other across currencies, the standard approach is **gross settlement** — every obligation settled individually. This demands enormous liquidity, introduces systemic risk, and generates unnecessary transaction costs.

Consider three counterparties:

```
Brazil  owes India   $100M equivalent
India   owes China   $100M equivalent
China   owes Brazil  $100M equivalent
```

**Gross settlement:** $300M in transfers.
**With cycle detection and netting:** $0 net settlement required.

Liquidity saved = systemic risk reduced.

This math scales. For N parties with M currencies, optimal netting can reduce gross settlement requirements by 60–90% in typical trade networks.

Yet there is **no open, modular, sovereign-agnostic engine** that performs multi-currency clearing optimization. Every existing system — SWIFT, CIPS, TARGET2, mBridge — is closed, proprietary, and institutionally controlled.

## What This Is

`clearing-engine` is a reference implementation of **multi-party, multi-currency liquidity optimization** for settlement systems.

It takes a directed graph of payment obligations denominated in multiple currencies and computes:

- **Net settlement positions** for each party
- **Cycle compression** to eliminate circular flows
- **Liquidity savings** quantification
- **Residual exposure** analysis
- **FX-adjusted clearing paths**

It is designed to be:

- **Correct** — deterministic, well-tested financial logic
- **Modular** — use only the components you need
- **Auditable** — clean Rust code with no hidden state
- **Sovereign-agnostic** — no assumption about which rails or currencies are involved

## What This Is Not

This is not a payment network. It does not move money. It does not connect to banks.

It is **infrastructure for reasoning about settlement** — a computational engine that can be embedded in simulators, research tools, sandbox environments, or (after hardening) production clearing systems.

## Quick Start

```bash
cargo add clearing-engine
```

```rust
use clearing_engine::prelude::*;
use rust_decimal_macros::dec;

fn main() {
    let mut graph = PaymentGraph::new();

    // Define parties
    let brazil = PartyId::new("BR-TREASURY");
    let india  = PartyId::new("IN-RBI");
    let china  = PartyId::new("CN-PBOC");

    // Add obligations
    let usd = CurrencyCode::new("USD");
    graph.add_obligation(Obligation::new(brazil, india, dec!(100_000_000), usd));
    graph.add_obligation(Obligation::new(india, china, dec!(100_000_000), usd));
    graph.add_obligation(Obligation::new(china, brazil, dec!(100_000_000), usd));

    // Run netting
    let result = graph.compute_net_positions();
    println!("Gross:     {}", result.gross_total());
    println!("Net:       {}", result.net_total());
    println!("Savings:   {:.1}%", result.savings_percent());
    // → Gross: 300000000, Net: 0, Savings: 100.0%
}
```

## Architecture

```
┌──────────────────────────────────────────────────────┐
│                   clearing-engine                    │
├──────────┬──────────┬───────────────┬────────────────┤
│   core   │  graph   │ optimization  │  simulation    │
├──────────┼──────────┼───────────────┼────────────────┤
│Obligation│ Payment  │   Bilateral   │  Stress Test   │
│ Currency │  Graph   │    Netting    │  FX Volatility │
│  Ledger  │  Cycle   │ Multilateral  │  Monte Carlo   │
│  Party   │Detection │   Netting     │  Scenario      │
│          │  SCC     │  Liquidity    │                │
│          │          │ Minimizer     │                │
└──────────┴──────────┴───────────────┴────────────────┘
```

### Modules

| Module | Purpose |
|--------|---------|
| `core` | Foundational types: obligations, currencies, parties, ledger state |
| `graph` | Payment graph construction, cycle detection, strongly connected components |
| `optimization` | Bilateral and multilateral netting, liquidity minimization algorithms |
| `simulation` | Stress testing, FX shock modeling, Monte Carlo scenarios |

## Use Cases

- **Academic research** on clearing and settlement optimization
- **Central bank simulation** of liquidity requirements under different netting regimes
- **Fintech prototyping** of multi-currency settlement flows
- **Regional payment consortium** feasibility analysis
- **Teaching** graph algorithms in a financial systems context

## Design Principles

1. **Correctness over speed.** Financial logic must be deterministic and auditable. We use `rust_decimal` for arbitrary-precision arithmetic — no floating-point in monetary calculations.

2. **No hidden state.** Every computation is a pure function from inputs to outputs. No global mutable state. No side effects in the engine core.

3. **Composition over configuration.** Components are designed to be composed, not configured via flags. Use the netting engine without the simulator. Use cycle detection without the ledger.

4. **Currency-agnostic.** The engine makes no assumptions about which currencies exist or their properties. USD, BRL, INR, CNY, digital currencies, and currencies that don't exist yet are all first-class.

5. **Institutionally presentable.** Code quality, documentation, and governance are maintained at a level suitable for institutional review.

## Examples

See the [`examples/`](examples/) directory:

- `basic_netting.rs` — Simple bilateral and multilateral netting
- `trilateral_cycle.rs` — Cycle detection and compression in a 3-party network

## Development

```bash
# Build
cargo build

# Test
cargo test

# Run examples
cargo run --example basic_netting
cargo run --example trilateral_cycle

# Benchmarks
cargo bench
```

## Roadmap

See [ROADMAP.md](ROADMAP.md) for the full development plan.

**Phase 1 (Current):** Core engine — obligation modeling, payment graph, cycle detection, bilateral & multilateral netting.

**Phase 2:** Simulation layer — stress testing, FX volatility modeling, Monte Carlo scenarios.

**Phase 3:** ISO 20022 awareness, deterministic settlement cycles, institutional documentation.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). We use an RFC process for significant changes.

## Governance

See [GOVERNANCE.md](GOVERNANCE.md).

## License

Apache 2.0. See [LICENSE](LICENSE).

Chosen deliberately: patent protection, enterprise-friendly, widely accepted for infrastructure projects.

---

**OpenSettlement** — open infrastructure for multi-currency settlement systems.
