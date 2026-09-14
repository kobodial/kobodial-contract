.PHONY: test fmt fmt-check clippy build build-wasm check deploy-testnet

test:
	cargo test

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy --all-targets -- -D warnings

build:
	cargo build

# The deployable artifact: the contract only exists on-chain as wasm, so a
# native build passing is necessary but never sufficient.
build-wasm:
	rustup target add wasm32v1-none
	cargo build --target wasm32v1-none --release

# What CI runs, runnable locally before you push.
check: fmt-check clippy test build-wasm

deploy-testnet:
	@test -n "$(ADMIN)" || (echo "usage: make deploy-testnet ADMIN=<address>" && exit 1)
	stellar contract deploy \
		--wasm target/wasm32v1-none/release/kobodial.wasm \
		--source kobodial-deployer \
		--network testnet \
		-- --admin "$(ADMIN)"
