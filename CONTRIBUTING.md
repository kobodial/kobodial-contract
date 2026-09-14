# Contributing to KoboDial

Thanks for your interest. KoboDial is a smart contract that holds
people's money, and it is operated by users who cannot inspect it — a
feature-phone user dialling a USSD code has no way to read the code or
verify a transaction. That asymmetry sets the bar for changes here.

## Before you start

Open an issue describing the problem before writing code, especially for
anything touching the authorization path. A change to how PINs or nonces
are checked is a change to the only thing standing between a relayer and
a user's balance.

## Running the tests

```sh
cargo test              # the full suite against the Soroban test env
cargo fmt --all         # formatting
cargo clippy --all-targets -- -D warnings
```

Build for the chain, not just the host, before you claim something
works:

```sh
rustup target add wasm32v1-none
cargo build --target wasm32v1-none --release
```

CI runs all four on every pull request.

## What a change needs

**Tests for the failure paths, not just the happy one.** The suite is
organised around what goes wrong: a wrong PIN, a replayed nonce, an
insufficient balance, a caller who is not the admin. A new entry point
needs the same treatment — show what happens when the user gets it
wrong, and assert the state did not move.

**A typed error, never a panic, on any path a user can reach.** The
gateway has 160 characters and a few seconds to tell someone on a
feature phone what happened. It can turn `InvalidNonce` into "your
session expired, please dial again". It can do nothing useful with a
WASM trap. If you add a failure mode, add a variant in `error.rs`.

**No PII on-chain.** Phone numbers and PINs are hashed off-chain by the
gateway and only hashes ever reach the contract. A change that stores,
logs or emits a raw phone number or PIN will not be merged.

**Nonce discipline.** Every fund-moving action must consume the wallet's
exact current nonce and increment it on success only. A failed action
must leave the nonce untouched, so a user who mistypes their PIN can
retry with the same USSD confirmation. If you find yourself relaxing the
equality check to `>=`, stop: that is the door the replay guard closes.

## Scope

The MVP is deliberately the wallet plus the PIN-gated relay pattern.
Multi-token support, interest or yield, and admin fee logic are
out of scope here and tracked as separate issues — a wallet that cannot
be reasoned about is worse than a wallet that does less.

## Commits

Explain why in the commit message, not just what. The diff already says
what changed.
