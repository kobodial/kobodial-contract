# Changelog

All notable changes to this project are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — MVP

Initial release: the phone-number-keyed wallet and the PIN-gated relay
pattern.

### Added

- `register`, `fund`, `send`, `cash_out`, `change_pin`, `get_balance`,
  `get_nonce` entry points.
- Wallet storage keyed by `phone_hash`, holding a hashed PIN, a
  balance, and a monotonically increasing nonce.
- Exact-match nonce enforcement on `send` and `cash_out`, so a relayed
  authorization is usable exactly once.
- Typed errors (`WalletNotFound`, `InvalidPin`, `InvalidNonce`,
  `InsufficientBalance`, `Unauthorized`, `AlreadyRegistered`) on every
  user-facing failure path.
- Events (`wallet_registered`, `funded`, `sent`, `cashed_out`,
  `pin_changed`) with the phone hash as an indexed topic.
- CI running fmt, clippy, the test suite, and a wasm build on every
  push and pull request.
- Deployed to Stellar testnet:
  `CCPXFBZNI2HR6TPCA5QIYLQRU5W4IXCEK5Y6ANTXKCRLXCXNRILIQQJU`.

### Deliberately not included

Multi-token support, interest or yield, and admin fee logic — tracked
as future-scope issues rather than folded into the MVP.
