.PHONY: help build test check clean run-init run-apply run-account run-meters run-report fmt clippy doc demo

# Default target
help:
	@echo "Metering Chain - Available commands:"
	@echo ""
	@echo "  make build          - Build the project (debug)"
	@echo "  make build-release  - Build the project (release)"
	@echo "  make test           - Run all tests"
	@echo "  make test-lib       - Run library tests only"
	@echo "  make test-flow      - Run integration tests"
	@echo "  make check          - Check code without building"
	@echo "  make clean          - Clean build artifacts"
	@echo "  make fmt            - Format code"
	@echo "  make clippy         - Run clippy linter"
	@echo "  make doc            - Generate documentation"
	@echo "  make demo           - Run demo flow (Mint, OpenMeter, Consume, CloseMeter)"
	@echo ""
	@echo "CLI commands:"
	@echo "  make run-init       - Initialize data directory"
	@echo "  make run-apply TX=... - Apply transaction (JSON)"
	@echo "  make run-account ADDR=... - Show account info"
	@echo "  make run-meters ADDR=... - List meters for account"
	@echo "  make run-report [ADDR=...] - Show usage report"

# Build commands
build:
	cargo build

build-release:
	cargo build --release

# Test commands
test:
	cargo test

test-lib:
	cargo test --lib

test-flow:
	cargo test --test basic_flow

# Check and lint
check:
	cargo check

clippy:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

# Clean
clean:
	cargo clean

# Documentation
doc:
	cargo doc --no-deps --open

# Demo flow
demo: clean
	@echo "=== Metering Chain Demo ==="
	@echo ""
	@echo "1. Initializing data directory..."
	@./target/debug/metering-chain init || cargo build && ./target/debug/metering-chain init
	@echo ""
	@echo "2. Minting 1000 to alice..."
	@echo '{"signer":"authority","nonce":0,"kind":{"Mint":{"to":"alice","amount":1000}}}' | ./target/debug/metering-chain apply || echo '{"signer":"authority","nonce":0,"kind":{"Mint":{"to":"alice","amount":1000}}}' | cargo run --bin metering-chain -- apply
	@echo ""
	@echo "3. Opening storage meter with 100 deposit..."
	@echo '{"signer":"alice","nonce":0,"kind":{"OpenMeter":{"owner":"alice","service_id":"storage","deposit":100}}}' | ./target/debug/metering-chain apply || echo '{"signer":"alice","nonce":0,"kind":{"OpenMeter":{"owner":"alice","service_id":"storage","deposit":100}}}' | cargo run --bin metering-chain -- apply
	@echo ""
	@echo "4. Consuming 10 units at 5 per unit..."
	@echo '{"signer":"alice","nonce":1,"kind":{"Consume":{"owner":"alice","service_id":"storage","units":10,"pricing":{"UnitPrice":5}}}}' | ./target/debug/metering-chain apply || echo '{"signer":"alice","nonce":1,"kind":{"Consume":{"owner":"alice","service_id":"storage","units":10,"pricing":{"UnitPrice":5}}}}' | cargo run --bin metering-chain -- apply
	@echo ""
	@echo "5. Consuming 5 more units..."
	@echo '{"signer":"alice","nonce":2,"kind":{"Consume":{"owner":"alice","service_id":"storage","units":5,"pricing":{"UnitPrice":5}}}}' | ./target/debug/metering-chain apply || echo '{"signer":"alice","nonce":2,"kind":{"Consume":{"owner":"alice","service_id":"storage","units":5,"pricing":{"UnitPrice":5}}}}' | cargo run --bin metering-chain -- apply
	@echo ""
	@echo "6. Closing meter..."
	@echo '{"signer":"alice","nonce":3,"kind":{"CloseMeter":{"owner":"alice","service_id":"storage"}}}' | ./target/debug/metering-chain apply || echo '{"signer":"alice","nonce":3,"kind":{"CloseMeter":{"owner":"alice","service_id":"storage"}}}' | cargo run --bin metering-chain -- apply
	@echo ""
	@echo "7. Final account state:"
	@./target/debug/metering-chain account alice || cargo run --bin metering-chain -- account alice
	@echo ""
	@echo "8. Final meters:"
	@./target/debug/metering-chain meters alice || cargo run --bin metering-chain -- meters alice
	@echo ""
	@echo "9. Usage report:"
	@./target/debug/metering-chain report || cargo run --bin metering-chain -- report

# CLI command wrappers
run-init:
	cargo run --bin metering-chain -- init

run-apply:
	@if [ -z "$(TX)" ]; then \
		echo "Error: TX variable required. Usage: make run-apply TX='{...}'"; \
		exit 1; \
	fi
	echo "$(TX)" | cargo run --bin metering-chain -- apply

run-account:
	@if [ -z "$(ADDR)" ]; then \
		echo "Error: ADDR variable required. Usage: make run-account ADDR=alice"; \
		exit 1; \
	fi
	cargo run --bin metering-chain -- account $(ADDR)

run-meters:
	@if [ -z "$(ADDR)" ]; then \
		echo "Error: ADDR variable required. Usage: make run-meters ADDR=alice"; \
		exit 1; \
	fi
	cargo run --bin metering-chain -- meters $(ADDR)

run-report:
	cargo run --bin metering-chain -- report $(if $(ADDR),$(ADDR),)
