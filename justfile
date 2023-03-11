check: fmt lint

fmt:
    cargo +nightly fmt

lint:
    cargo clippy
