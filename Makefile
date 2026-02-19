.PHONY: help build run test clean fmt lint check run-guidelines package

help:
	@echo "Available targets:"
	@echo "  build          - Build the project (with interchange)"
	@echo "  run            - Run the project"
	@echo "  test           - Run tests (with interchange)"
	@echo "  clean          - Clean build artifacts"
	@echo "  fmt            - Format code with rustfmt"
	@echo "  lint           - Run clippy linter (with interchange)"
	@echo "  check          - Run fmt + lint + test"
	@echo "  run-guidelines - Run complete validation (fmt + lint + build + test)"

build:
	cargo build --features interchange

release:
	cargo build --release --features interchange

run:
	cargo run --features interchange

test:
	cargo test --features interchange

test-verbose:
	cargo test --features interchange -- --nocapture

clean:
	cargo clean

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

lint:
	cargo clippy --all-targets --features interchange -- -D warnings

check: fmt-check lint test

run-guidelines:
	@echo "=== Running Complete Validation Pipeline ==="
	@echo ""
	@echo "Step 1/3: Formatting code..."
	@cargo fmt
	@echo "✓ Code formatted"
	@echo ""
	@echo "Step 2/3: Running linter (includes build)..."
	@cargo clippy --all-targets --features interchange -- -D warnings
	@echo "✓ Linting passed"
	@echo ""
	@echo "Step 3/3: Running tests..."
	@cargo test --features interchange
	@echo ""
	@echo "=== ✓ All guidelines passed! ==="

package:
	@echo "Building package..."
	@cargo build --release
	@echo "✓ Package built"
