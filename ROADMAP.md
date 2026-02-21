# Roadmap

## Vision

Build the standard open-source toolkit for multi-currency clearing and liquidity optimization — usable by researchers, fintechs, regional payment networks, and institutional simulation environments.

---

## Phase 1: Core Engine (Months 1–3) ← **Current**

**Goal:** Ship a correct, well-tested, well-documented clearing engine.

### Milestone 1.0: Foundation ✅
- [x] Core types: `PartyId`, `CurrencyCode`, `Obligation`, `Ledger`
- [x] `PaymentGraph` — directed graph construction with aggregation
- [x] `FxRateTable` — exchange rate storage and conversion
- [x] Bilateral netting algorithm
- [x] Multilateral netting algorithm
- [x] `LiquidityAnalysis` — debtor requirements from netting results
- [x] Cycle detection (DFS-based)
- [x] Strongly connected components (Tarjan's algorithm)
- [x] Random network generator for stress testing
- [x] GitHub Actions CI (check, test, fmt, clippy, docs)
- [x] Examples: `basic_netting`, `trilateral_cycle`

### Milestone 1.1: Hardening ← **In Progress**
- [x] Property-based tests (9 invariants: ledger balance, net ≤ gross, determinism, bilateral correctness, cycle bottleneck validity, currency breakdown consistency)
- [x] Integration tests (full pipeline, JSON round-trip, multi-currency independence, empty set handling)
- [x] CLI binary (`clearing-engine net|cycles|generate`)
- [x] JSON serialization for all result types (input/output via CLI)
- [x] Documentation: architecture guide in `docs/architecture.md`
- [x] Interactive explorer in `docs/explorer/`
- [ ] Benchmarks with `criterion` (10, 100, 1000, 10000 party networks) — scaffolded, needs real runs
- [ ] First crates.io publish

---

## Phase 2: Simulation Layer (Months 3–6)

**Goal:** Enable "what-if" analysis and stress testing.

### Milestone 2.0: FX Volatility
- [ ] FX shock scenarios — apply rate changes to obligation sets
- [ ] Recompute netting under stressed rates
- [ ] VaR-style exposure reporting
- [ ] Monte Carlo simulation over rate distributions

### Milestone 2.1: Deterministic Simulation
- [ ] Time-stepped clearing cycles
- [ ] Queue-based settlement simulation
- [ ] Liquidity injection/withdrawal modeling
- [ ] Gridlock detection and resolution algorithms

### Milestone 2.2: Visualization
- [ ] Export netting results to DOT/Graphviz format
- [ ] Settlement flow diagrams
- [ ] Liquidity heatmaps (optional web UI)

---

## Phase 3: Standards & Interoperability (Months 6–9)

**Goal:** Connect the engine to real-world message formats.

### Milestone 3.0: ISO 20022 Awareness
- [ ] Parse ISO 20022 `pacs.008` (credit transfer) messages
- [ ] Convert ISO messages to internal `Obligation` format
- [ ] Validate message schemas
- [ ] CLI: `clearing-engine convert --from iso20022 --input payment.xml`

### Milestone 3.1: Output Formats
- [ ] Generate settlement instructions in ISO 20022 format
- [ ] CSV/Excel export for netting results
- [ ] JSON-LD output for academic/research use

---

## Phase 4: Advanced Optimization (Months 9–12)

**Goal:** Publish research-grade optimization algorithms.

### Milestone 4.0: Advanced Netting
- [ ] Partial netting with priority ordering
- [ ] Time-windowed netting (batch vs. continuous)
- [ ] Multi-currency cross-netting with FX optimization
- [ ] Minimum-cost flow formulation for settlement

### Milestone 4.1: Gridlock Resolution
- [ ] Queue optimization under liquidity constraints
- [ ] Pareto-optimal settlement ordering
- [ ] Cooperative vs. non-cooperative netting game theory models

---

## Phase 5: Ecosystem (Year 2+)

**Goal:** Grow beyond a single repo into a platform.

### Planned Sibling Repos (under Polysettle org)
- `iso20022-toolkit` — Standalone ISO 20022 validator and converter (Go or Rust)
- `rail-adapters` — Mock connectors for sovereign payment systems (sandbox APIs)
- `compliance-kit` — Modular AML/sanctions screening engine
- `fx-exposure-engine` — Real-time multi-currency exposure tracking
- `settlement-sandbox` — Full-stack dev environment combining all components

### Community Goals
- Academic paper referencing the engine
- 3+ external contributors
- Used in at least one university course or research project
- Forked by a regional payment initiative

---

## Non-Goals

Things this project will deliberately **not** do:

- Move real money
- Connect to live banking systems
- Implement specific national regulations
- Take political positions on currency systems
- Require blockchain or tokens

---

## How to Influence the Roadmap

Open an issue with the `roadmap` label. Describe the use case, not just the feature. Prioritization is based on: correctness impact > generality > community demand > implementation effort.
