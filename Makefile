.PHONY: build test integration-test fmt fmt-check lint deny clean deploy-local deploy-testnet setup setup-hooks export-streams sync-labels

# Configuration for the export-streams target — override on the command line:
#   make export-streams CONTRACT_ID=C... EMPLOYER=G...
CONTRACT_ID ?= $(STREAM_CONTRACT_ID)
EMPLOYER ?= $(EMPLOYER_ADDRESS)

build:
	stellar contract build

test:
	cargo build -p paystream-stream --target wasm32v1-none --release
	cargo test

coverage:
	cargo llvm-cov --workspace --summary-only

mutation-test:
	cargo mutants -p paystream-stream

integration-test:
	docker run -d --rm --name paystream-sandbox -p 8000:8000 stellar/quickstart:latest --local --enable-soroban-rpc
	./tests/integration/run.sh; status=$$?; docker stop paystream-sandbox >/dev/null; exit $$status

fmt:
	cargo fmt

fmt-check:
	cargo fmt --check

lint:
	cargo clippy --all-targets -- -D warnings

check:
	cargo check --all

deny:
	cargo deny check

clean:
	cargo clean

deploy-local:
	./scripts/deploy-local.sh

deploy-testnet:
	./scripts/deploy-testnet.sh

# Export an employer's streams as CSV for payroll reconciliation.
# Add JSON=1 to emit JSON instead of CSV, NETWORK=local to query another network.
export-streams:
	./scripts/export-streams.sh "$(CONTRACT_ID)" "$(EMPLOYER)" $(if $(NETWORK),--network $(NETWORK),) $(if $(JSON),--json,)

# Apply .github/labels.yml to the repository (add --dry-run to preview).
sync-labels:
	./scripts/sync-labels.sh

setup:
	@echo "Setting up development environment..."
	@echo "Checking for Rust installation..."
	@which rustc > /dev/null 2>&1 || (echo "Installing Rust via rustup..." && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y)
	@. "$$HOME/.cargo/env" && which rustc > /dev/null 2>&1 && echo "✓ Rust installed" || (echo "✗ Rust installation failed. Please run: source $$HOME/.cargo/env" && exit 1)
	@echo "Adding wasm32-unknown-unknown target..."
	@. "$$HOME/.cargo/env" && rustup target add wasm32-unknown-unknown && echo "✓ wasm32-unknown-unknown target added" || (echo "✗ Failed to add wasm32-unknown-unknown target" && exit 1)
	@echo "Installing Stellar CLI (version 27.0.0)..."
	@. "$$HOME/.cargo/env" && cargo install --locked stellar-cli --version 27.0.0 && echo "✓ Stellar CLI installed" || (echo "✗ Stellar CLI installation failed" && exit 1)
	@echo ""
	@echo "Setup complete! You can now run 'make test' to verify your installation."
	@echo "Note: You may need to run 'source $$HOME/.cargo/env' or restart your terminal for Rust to be available in new shell sessions."

setup-hooks:
	@echo "Installing pre-commit hook..."
	@cp scripts/pre-commit-hook.sh .git/hooks/pre-commit
	@chmod +x .git/hooks/pre-commit
	@echo "✓ Pre-commit hook installed successfully."
	@echo "The hook will run 'cargo fmt --check' and 'cargo clippy' before each commit."
	@echo "To skip the hook, use: git commit --no-verify"
