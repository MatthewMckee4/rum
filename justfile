# https://just.systems

default:
    @just --list

test *args:
    @if command -v cargo-nextest > /dev/null 2>&1; then \
        cargo nextest run {{args}}; \
    else \
        cargo test {{args}}; \
    fi

build:
    cargo build --release

lint:
    cargo fmt -- --check
    cargo clippy --all-targets --all-features -- -D warnings

fmt:
    cargo fmt

bench *args:
    cargo bench --bench delete {{args}}

pre-commit:
    uvx prek run -a
