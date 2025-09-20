# Rust Starter

This is a sample starter project for Rust. It includes a simple hello world program. Adds several GitHub Actions workflows for CI/CD.

## GitHub Actions Workflows

Several workflows are included in the repository. They are triggered on different events.

### Analyzers

- rustfmt
- clippy
- cargo-audit

### Tests

- cargo test (Includes doc tests)
- coverage (via grcov). Defaults to 70% coverage threshold requirement. The build will fail if the coverage is below the threshold.
