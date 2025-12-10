profile := "dev"
coverage-html-directory := "/tmp/coverage"
coverage-threshold := "80"

audit:
    cargo deny check

build:
    cargo build --locked --profile {{profile}}

clippy:
    cargo clippy --locked --profile {{profile}} --workspace --all-targets --no-deps --all-features -- -D warnings

clippy-fix:
    cargo clippy --locked --profile {{profile}} --workspace --all-targets --no-deps --all-features --fix --allow-dirty -- -D warnings

coverage:
    cargo +nightly llvm-cov --all-features --workspace --locked --branch
    cargo +nightly llvm-cov report --html --output-dir={{coverage-html-directory}}

coverage-check: coverage
    cargo +nightly llvm-cov report --fail-under-lines={{coverage-threshold}}

doc $RUSTDOCFLAGS="-D warnings":
    cargo doc --locked --lib --no-deps --all-features --document-private-items

fmt:
    cargo +nightly fmt

fmt-check:
    cargo +nightly fmt --check

test:
    cargo test --locked --profile {{profile}}

unit-test:
    cargo test --locked --profile {{profile}} --lib