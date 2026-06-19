# Trit-Core v0.3.0

[![CI](https://github.com/trit-core/trit-core/actions/workflows/ci.yml/badge.svg)](https://github.com/trit-core/trit-core/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org)

A ternary decision engine for conflict-aware AI alignment.

```mermaid
graph TD
    L1["Anchor — Steady-state constraints · veto power"]
    L2["Hook — Scenario perception · module scheduling"]
    L3["Adapters — Dynamic cognitive module pool"]
    L4["Core — Ternary algebra · TAND/TOR/TNOT · Phase arithmetic"]
    L5["Meta — Policy engine · conflict arbitration · SafeFallback"]
    Sandbox["Sandbox — Scenario pipeline · depth gating · calibration feedback"]

    L1 --> L2 --> L3 --> L4 --> L5 --> Sandbox
```

## Why Hold matters

Binary logic forces a choice: True or False. When scientific evidence points one way and individual circumstance points another, both answers are wrong. **The act of choosing destroys information.**

Trit-Core introduces **Hold** — intentional suspension of judgment that preserves the conflict instead of collapsing it. Hold is not "uncertain." Hold is "this should not be decided by an algorithm."

```rust
use trit_core::core::{Frame, TernaryAlgebra, TritValue, TritWord};

let science     = TritWord::tru(Frame::Science);
let individual  = TritWord::fals(Frame::Individual);

let (result, interrupt) = TernaryAlgebra::t_and(&science, &individual);

assert_eq!(result.value(), TritValue::Hold); // conflict preserved, not erased
```

## 30 seconds in

```bash
cargo build --release
cargo test --all-features
cargo run --release --bin trit-sandbox -- --scenario scenarios/medical_conflict_01.json
```

## Read more

| Document | For |
|----------|-----|
| [docs/INDEX.md](docs/INDEX.md) | Full documentation map |
| [docs/tutorials/QUICKSTART.md](docs/tutorials/QUICKSTART.md) | 3 minutes from clone to first scenario |
| [docs/technical-whitepaper.md](docs/technical-whitepaper.md) | v0.3.0 technical whitepaper & audit index |

## License

MIT
