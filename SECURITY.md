# Security Policy

**Version**: 0.3.0

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
- The `trit-sandbox` binary
- The `dhat-profile` profiling binary

### Out of Scope

- Scenarios in `scenarios/` — these are synthetic test cases, not production data
- Dependencies — vulnerabilities in third-party crates should be reported upstream
- Any distributed node protocol or network layer (removed in v0.2.0; if revived as a separate crate, it will have its own security policy)

## Known Safe Design Properties

- `#![forbid(unsafe_code)]` — no unsafe Rust in the entire project
- Input JSON size is capped at `MAX_JSON_SIZE = 64 * 1024` bytes
- Scenarios are limited to `MAX_SIGNALS = 100` signals
- Free-form string fields are capped at `MAX_STRING_LEN = 1024` characters
- `SafeFallback` implements IEC 61508 fail-safe semantics for dangerous domains (`Physical`, `Engineering`, and registered dangerous `Custom` domains)
- Phase values are validated to be finite and within `[0.0, 1.0]`
- Log fields are sanitized to remove control characters before emission

## Current Attack Surface

### `trit-sandbox`

- **Path traversal**: The binary opens the file provided via `--scenario`. It relies on the OS path resolution and does not perform additional sandboxing of the file path. Do not pass untrusted paths without validation.
- **Resource exhaustion**: Input size and signal count are bounded; extremely deeply nested or malformed JSON is rejected by `serde_json` and `validate_scenario`.
- **Log injection**: Scenario fields are sanitized via `sanitize_log_field` before being emitted in tracing events, reducing the risk of log-forging or control-character injection.
- **Information disclosure**: `--diagnostic` and `--trace` emit detailed internal state; do not enable in production with sensitive inputs.

### `dhat-profile`

- This binary is intended for local heap-profiling only. It is not hardened for production or adversarial inputs.
- It shares the same input validation path as `trit-sandbox`.

## Reporting Template

When reporting a vulnerability, please include:

1. Affected component (`trit-core`, `trit-sandbox`, or `dhat-profile`)
2. Steps to reproduce
3. Expected vs. actual behavior
4. Impact assessment
5. Suggested fix (if any)
