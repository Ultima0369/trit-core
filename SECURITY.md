# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in Trit-Core, please report it
privately to the project maintainer. Do NOT disclose it publicly through
GitHub Issues or pull requests.

### Contact

Please send vulnerability reports to the project maintainer via GitHub's
private vulnerability reporting system, or by email (TBD).

### Response Timeline

- **Acknowledgement**: Within 48 hours
- **Initial assessment**: Within 5 business days
- **Fix release**: Depends on severity — critical fixes within 7 days

### Scope

This security policy covers:

- The `trit-core` library crate
- The `trit-sandbox` and `trit-node` binaries
- The `dhat-profile` profiling binary
- The Docker Compose configuration
- The TCP frame protocol implementation

### Out of Scope

- Scenarios in `scenarios/` — these are synthetic test cases, not production data
- Dependencies — vulnerabilities in third-party crates should be reported upstream

## Known Safe Design Properties

- `#![forbid(unsafe_code)]` — no unsafe Rust in the entire project
- SafeFallback implements IEC 61508 fail-safe semantics for dangerous domains
- TCP frames are capped at 1 MiB to prevent CWE-770 memory exhaustion
- `MAX_NODES = 256` prevents unbounded node registration
- `MAX_MESSAGE_LOG = 10,000` prevents unbounded log growth
