# Architecture

## Overview

The clearing engine is a computational library that takes payment obligations as input and produces optimized settlement positions as output. It does not move money, connect to networks, or manage state between runs.

```
                    ┌─────────────────┐
                    │   Input Layer   │
                    │                 │
                    │  JSON / CLI /   │
                    │  Rust API       │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │      core       │
                    │                 │
                    │  Obligation     │
                    │  PartyId        │
                    │  CurrencyCode   │
                    │  Ledger         │
                    │  FxRateTable    │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
     ┌────────▼───────┐ ┌───▼────────┐ ┌───▼──────────┐
     │     graph      │ │optimization│ │  simulation  │
     │                │ │            │ │              │
     │ PaymentGraph   │ │ Netting    │ │ StressTest   │
     │ CycleDetection │ │ Liquidity  │ │ FxVolatility │
     │ SCC (Tarjan)   │ │            │ │              │
     └────────────────┘ └────────────┘ └──────────────┘
                             │
                    ┌────────▼────────┐
                    │  Output Layer   │
                    │                 │
                    │  NettingResult  │
                    │  LiquidityAnal. │
                    │  JSON / stdout  │
                    └─────────────────┘
```

## Module Dependency Graph

Dependencies flow downward only. No module imports from a module at the same level or above it.

```
core ← has zero dependencies on other engine modules
  ↑
  ├── graph ← depends only on core
  ├── optimization ← depends only on core
  └── simulation ← depends on core + optimization
```

This means:

- You can use `core` types without pulling in any algorithms.
- You can use `optimization::netting` without ever touching the graph module.
- `simulation` is the only module that crosses boundaries.

## Data Flow

A typical computation follows this path:

```
1. Create Obligations
   User defines who owes whom, how much, in what currency.

2. Build PaymentGraph (optional)
   Aggregates obligations into a directed graph.
   Enables cycle detection and SCC analysis.

3. Run Netting
   NettingEngine::multilateral_net() processes all obligations.
   Builds a Ledger internally.
   Returns NettingResult with net positions per party per currency.

4. Analyze Liquidity (optional)
   LiquidityAnalysis::from_netting_result() computes
   how much cash each net debtor actually needs.

5. Output
   NettingResult serializes to JSON.
   Display impl prints human-readable summary.
```

## Key Design Decisions

### Decimal Arithmetic

All monetary values use `rust_decimal::Decimal`, never floating-point. This is non-negotiable for financial software. Floating-point introduces rounding errors that compound across thousands of obligations and make results non-deterministic.

```rust
// CORRECT
use rust_decimal_macros::dec;
let amount = dec!(100_000_000.50);

// WRONG — never do this for money
let amount: f64 = 100_000_000.50;
```

### Immutable Obligations

Once created, an `Obligation` cannot be modified. The clearing engine operates on snapshots of obligation sets. This makes the engine stateless and deterministic — the same input always produces the same output, regardless of when or how many times you run it.

### Currency Agnosticism

The engine treats currency codes as opaque strings. There is no hardcoded list of valid currencies. `CurrencyCode::new("USD")`, `CurrencyCode::new("BTC")`, and `CurrencyCode::new("EXPERIMENTAL-TOKEN-7")` are all equally valid. This ensures the engine works for fiat currencies, CBDCs, digital assets, and currencies that do not yet exist.

### Separation of Graph Analysis and Netting

Cycle detection (`graph/`) and netting (`optimization/`) are independent. You do not need to find cycles before netting — `multilateral_net()` computes correct net positions purely from the ledger, without graph traversal.

The graph module exists for *insight*: understanding the structure of the obligation network, identifying bottlenecks, and finding compression opportunities. The netting module exists for *computation*: producing the actual settlement numbers.

### No Side Effects in Core

The core engine (`core/`, `graph/`, `optimization/`) performs no I/O, no network calls, no file access, and no logging. It is a pure computational library. I/O happens only at the CLI/application boundary.

## The Netting Algorithm

Multilateral netting works as follows:

1. **Initialize a Ledger** — empty position for each (party, currency) pair.

2. **Apply each Obligation** — for every obligation, subtract the amount from the debtor's position and add it to the creditor's position.

3. **Read net positions** — each party's position is the sum of all credits minus all debits. Positive = net creditor. Negative = net debtor. Zero = fully netted.

4. **Compute net settlement total** — sum of all positive positions (which equals the absolute sum of all negative positions, since the ledger always balances to zero).

5. **Compute savings** — gross total minus net total.

This is O(n) in the number of obligations. It does not require graph construction, cycle detection, or any iterative optimization. The ledger-based approach computes the theoretical maximum netting efficiency in a single pass.

Cycle detection is a separate analysis that explains *why* netting works — it identifies the circular flows that cancel out. But the netting result is identical whether or not you run cycle detection.

## File Layout

```
src/
├── lib.rs                      # Crate root, module declarations, prelude
├── core/
│   ├── mod.rs                  # Module declarations
│   ├── party.rs                # PartyId — counterparty identifier
│   ├── currency.rs             # CurrencyCode, FxRateTable
│   ├── obligation.rs           # Obligation, ObligationSet
│   └── ledger.rs               # Ledger — net position tracker
├── graph/
│   ├── mod.rs
│   ├── payment_graph.rs        # Directed graph with edge aggregation
│   ├── cycle_detection.rs      # DFS-based cycle finder
│   └── scc.rs                  # Tarjan's SCC algorithm
├── optimization/
│   ├── mod.rs
│   ├── netting.rs              # Bilateral + multilateral netting
│   └── liquidity.rs            # Liquidity requirement analysis
└── simulation/
    ├── mod.rs
    ├── stress_test.rs          # Random network generator
    └── fx_volatility.rs        # FX shock modeling (Phase 2)
```

## Testing Strategy

- **Unit tests** — in each module file, testing individual functions.
- **Property-based tests** — in `tests/`, using `proptest` to verify invariants hold across randomly generated inputs (e.g., ledger always balances, net ≤ gross).
- **Integration tests** — in `tests/`, testing full pipelines from obligation creation through netting to JSON output.
- **Benchmarks** — in `benches/`, measuring performance at various network sizes.
- **Examples** — in `examples/`, demonstrating common use cases.
