# Contributing to clearing-engine

Thank you for your interest in contributing to the OpenSettlement clearing engine.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/clearing-engine.git`
3. Create a branch: `git checkout -b feature/your-feature`
4. Make your changes
5. Run tests: `cargo test`
6. Run clippy: `cargo clippy -- -D warnings`
7. Run formatter: `cargo fmt`
8. Submit a pull request

## Development Requirements

- Rust stable (latest)
- `cargo fmt` and `cargo clippy` must pass with no warnings

## Code Standards

### Correctness First

This is financial infrastructure. Correctness is non-negotiable.

- **No floating-point arithmetic** for monetary values. Use `rust_decimal`.
- **No hidden state**. Core engine functions must be pure.
- **Deterministic outputs**. Same inputs must always produce same outputs.
- **Test coverage > 80%** for all modules.

### Style

- Follow standard Rust conventions (`rustfmt` defaults)
- Document all public items with doc comments
- Include examples in doc comments where practical
- Use descriptive variable names — `debtor`, not `d`

### Commit Messages

Use conventional commits:

```
feat: add multilateral netting for N currencies
fix: correct cycle detection for self-loops
docs: update README with new example
test: add stress test for 100-party network
refactor: extract graph traversal into helper
```

## RFC Process

For significant changes (new modules, algorithm changes, API redesign), open an RFC:

1. Create an issue with the `rfc` label
2. Title format: `RFC: <short description>`
3. Include: motivation, design, alternatives considered, migration path
4. Allow 7 days for discussion before implementation

## What We're Looking For

High-value contributions:

- **Algorithm improvements** — better netting efficiency, faster cycle detection
- **New optimization strategies** — partial netting, time-windowed clearing
- **Test coverage** — edge cases, property-based tests, benchmarks
- **Documentation** — tutorials, architecture guides, academic references
- **ISO 20022 expertise** — message format support, schema validation

## What to Avoid

- Political framing in code, docs, or discussions
- Dependencies on specific payment rails or currencies
- Breaking changes without an RFC
- Commits that reduce test coverage

## Questions?

Open a discussion on GitHub. We're building infrastructure — thoughtful questions are always welcome.
