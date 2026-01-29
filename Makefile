.PHONY: help check test security lint audit deny clippy fmt clean

help:
	@echo "Bitvue Build & Security Targets"
	@echo "================================"
	@echo "make check     - Run cargo check"
	@echo "make test      - Run all tests"
	@echo "make security  - Run all security checks"
	@echo "make lint      - Run clippy lints"
	@echo "make audit     - Check for vulnerable dependencies"
	@echo "make deny      - Run cargo-deny checks"
	@echo "make fmt       - Format code with rustfmt"
	@echo "make clean     - Clean build artifacts"

check:
	cargo check --workspace

test:
	cargo test --workspace

security: audit deny lint
	@echo "✓ All security checks passed"

lint:
	cargo clippy --workspace --all-targets --all-features -- -D warnings

audit:
	@command -v cargo-audit >/dev/null 2>&1 || { echo "Installing cargo-audit..."; cargo install cargo-audit; }
	cargo audit

deny:
	@command -v cargo-deny >/dev/null 2>&1 || { echo "Installing cargo-deny..."; cargo install cargo-deny; }
	cargo deny check

fmt:
	cargo fmt --all

clean:
	cargo clean
