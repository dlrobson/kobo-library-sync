coverage-html-directory := ""
coverage-threshold := "80"
profile := "dev"

_coverage-html-output-directory-argument := if coverage-html-directory != "" { "--output-dir=" + coverage-html-directory } else { "" }

audit:
    cargo deny check

build:
    cargo build --locked --profile {{profile}}

clippy *ARGS:
    cargo clippy --locked --profile {{profile}} --workspace --all-features --all-targets {{ARGS}} -- -D warnings

clippy-fix *ARGS: (clippy "--fix" "--allow-dirty" ARGS)

coverage:
    cargo +nightly llvm-cov --all-features --workspace --locked --branch
    cargo +nightly llvm-cov report --html {{_coverage-html-output-directory-argument}} --fail-under-lines={{coverage-threshold}}

doc $RUSTDOCFLAGS="-D warnings":
    cargo doc --locked --profile {{profile}} --lib --no-deps --all-features --document-private-items

fmt *ARGS:
    cargo +nightly fmt {{ARGS}}

test *ARGS:
    cargo test --locked --profile {{profile}} {{ARGS}}

unit-test: (test "--lib")
