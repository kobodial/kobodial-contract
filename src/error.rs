//! Typed errors for every user-facing failure path.
//!
//! A gateway serving USSD has a few seconds of the user's attention and a
//! 160-character message. It cannot translate a WASM panic into "wrong
//! PIN, try again" — so the contract never panics where a user can cause
//! the failure. Every failure the relayer can trigger on a user's behalf
//! maps to a distinct variant, and the gateway maps variants to sensible
//! USSD messages.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// No wallet exists for this phone hash — the number is not registered.
    WalletNotFound = 1,
    /// The supplied PIN hash does not match the wallet's stored PIN hash.
    /// Deliberately identical whether the PIN is merely wrong or the wallet
    /// is under attack: never reveal which.
    InvalidPin = 2,
    /// The supplied nonce is not the wallet's current nonce — either a
    /// replay of an already-executed action or a stale USSD session.
    InvalidNonce = 3,
    /// The wallet's balance is below the requested amount.
    InsufficientBalance = 4,
    /// The caller is not the gateway admin. Only the admin may register
    /// wallets, take cash-in, and execute cash-outs.
    Unauthorized = 5,
    /// A wallet already exists for this phone hash. Registration would
    /// otherwise reset the PIN and take over the balance, so it is
    /// refused rather than silently ignored.
    AlreadyRegistered = 6,
}
