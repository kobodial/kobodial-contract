# KoboDial

A phone-number-keyed smart wallet for Stellar/Soroban, operated over USSD
from a feature phone. Named for the **kobo**, the smallest unit of the
Nigerian Naira.

## The problem

A wallet normally works because its owner holds a private key and signs
their own transactions. Someone on a basic phone cannot do that: there
is no wallet app on a USSD session, no secure storage for a key, and no
way to sign anything. The usual answer is a custodial account — the
operator holds the keys and the user holds a promise.

KoboDial takes a narrower path. A backend relayer does submit every
transaction, because someone has to. But the **contract** decides whether
the action was authorized, and it will only accept an action carrying the
user's PIN and the wallet's exact current nonce. The relayer can relay;
it cannot decide.

That is not full self-custody, and this README will not pretend
otherwise — see [SECURITY.md](SECURITY.md) for what the relayer *can*
still do (censor, refuse service, and credit unbacked balances in the
MVP). What it buys is that a compromised or dishonest relayer cannot
move a user's money, which is the failure that matters most.

## How authorization works

Two things guard every fund-moving call:

1. **The PIN hash must match.** The gateway hashes the PIN the user typed
   and sends the hash; the contract compares it against the stored hash.
   Neither the PIN nor the phone number is ever stored on-chain — only
   SHA-256 hashes, computed off-chain.
2. **The nonce must match exactly.** Each wallet carries a counter. An
   action must carry the wallet's current value, and a successful action
   increments it. Not "greater than" — *exactly equal*. That one detail
   is what makes a relayed authorization usable once and only once.

A failed action leaves the nonce untouched, so a user who mistypes their
PIN can retry with the same USSD confirmation rather than being locked
out of their own session.

```
 User (feature phone)        Gateway / relayer           KoboDial contract
         │                          │                            │
         │  *737# → send 500 to X   │                            │
         ├─────────────────────────►│                            │
         │                          │  get_nonce(phone_hash)     │
         │                          ├───────────────────────────►│
         │                          │◄───────────────────────────┤
         │                          │            nonce = 7       │
         │   "Enter PIN to confirm" │                            │
         │◄─────────────────────────┤                            │
         │        1 2 3 4           │                            │
         ├─────────────────────────►│                            │
         │                          │ hash the PIN off-chain     │
         │                          │                            │
         │                          │ send(from, to, 500,        │
         │                          │      pin_hash, nonce=7)    │
         │                          ├───────────────────────────►│
         │                          │                            │─┐ pin_hash == stored?
         │                          │                            │ │ nonce == 7 exactly?
         │                          │                            │ │ balance >= 500?
         │                          │                            │◄┘
         │                          │      Ok — nonce becomes 8  │
         │                          │◄───────────────────────────┤
         │  "Sent. Balance: 1,200"  │                            │
         │◄─────────────────────────┤                            │
```

Replay the same message and the second attempt hits `nonce == 7` against
a wallet whose nonce is now 8, and fails with `InvalidNonce`. The relayer
cannot run it twice; nor can anyone who captured it in flight.

## Public interface

| Function | Who signs | Authorized by | Purpose |
| --- | --- | --- | --- |
| `register(admin, phone_hash, pin_hash)` | admin | admin identity | Enrol a phone number after USSD verification |
| `fund(admin, phone_hash, amount)` | admin | admin identity | Agent cash-in: credit a balance against cash received |
| `send(from_hash, to_hash, amount, pin_hash, nonce)` | relayer | **user PIN + nonce** | Transfer between two wallets |
| `cash_out(admin, phone_hash, amount, pin_hash, nonce)` | admin | **user PIN + nonce** | Debit for physical cash payout by an agent |
| `change_pin(phone_hash, old_pin_hash, new_pin_hash)` | relayer | **old PIN** | Change the PIN |
| `get_balance(phone_hash)` | — | — | Read a balance |
| `get_nonce(phone_hash)` | — | — | Read the next nonce a call must carry |

Errors are typed, never panics: `WalletNotFound`, `InvalidPin`,
`InvalidNonce`, `InsufficientBalance`, `Unauthorized`,
`AlreadyRegistered`. The gateway maps each to a USSD message — it has
160 characters and a few seconds, and can do nothing useful with a WASM
trap.

Events — `wallet_registered`, `funded`, `sent`, `cashed_out`,
`pin_changed` — carry the phone hash as an indexed topic and the
resulting balance in the data, so the gateway can confirm a step without
a follow-up read.

## Building and testing

```sh
cargo test                                   # full suite, Soroban test env
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings

rustup target add wasm32v1-none
cargo build --target wasm32v1-none --release # the deployable artifact
```

## Testnet deployment

```sh
# One-time: an identity to deploy with, funded by friendbot
stellar keys generate --global deployer --network testnet --fund

# Build and deploy. The constructor takes the gateway admin address —
# the relayer's own account, which is the only account that will ever
# sign against this contract.
stellar contract build
stellar contract deploy \
  --wasm target/wasm32v1-none/release/kobodial.wasm \
  --source deployer \
  --network testnet \
  -- --admin "$(stellar keys address deployer)"
```

Then exercise it:

```sh
stellar contract invoke --id <CONTRACT_ID> --source deployer --network testnet \
  -- register --admin "$(stellar keys address deployer)" \
     --phone_hash <32-byte-hex> --pin_hash <32-byte-hex>

stellar contract invoke --id <CONTRACT_ID> --source deployer --network testnet \
  -- get_balance --phone_hash <32-byte-hex>
```

The hashes are plain SHA-256 digests, hex-encoded — computed by the
gateway, never by the contract:

```sh
printf '+2348012345678' | sha256sum   # phone_hash
printf '1234'           | sha256sum   # pin_hash
```

## Scope

The MVP is the wallet and the PIN-gated relay pattern, deliberately.
Multi-token support, interest or yield, and admin fee logic are out of
scope and tracked as separate issues.

## License

MIT — see [LICENSE](LICENSE).
