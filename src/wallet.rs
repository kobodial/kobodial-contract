//! Wallet storage: the data model, the storage keys, and the events.
//!
//! Keyed by phone_hash — the SHA-256 of the user's phone number. Raw phone
//! numbers are PII and never touch the chain; the gateway hashes them
//! off-chain and only the hash is ever passed to the contract. The same
//! applies to PINs: only pin_hash is stored, hashed off-chain by the
//! gateway. The contract can verify a hash it is given but can never learn
//! the PIN itself.

use soroban_sdk::{contractevent, contracttype, Address, BytesN, Env};

// The gateway admin is the relayer service's own Soroban account — the
// only account that ever signs transactions against this contract. Users
// never hold keys; the PIN inside the payload is their authorization, and
// the admin's signature only attests that the payload is what the user
// approved over USSD, never that the admin approves the action itself.
// Stored under StorageKey::Admin.

/// One wallet, keyed by phone hash. Balances are tracked in stroops of the
/// settlement token; the contract is the ledger of record for who owns
/// what, and moves value by ledger entries rather than by key custody.
#[contracttype]
pub struct Wallet {
    /// SHA-256 of the user's PIN, hashed off-chain by the gateway. Stored
    /// raw-hash only; the contract never sees the PIN.
    pub pin_hash: BytesN<32>,
    /// Tracked balance, in the smallest unit of the settlement asset.
    pub balance: i128,
    /// Monotonically increasing action counter. Every PIN-authorized
    /// action (send, cash_out) must carry the wallet's exact current
    /// nonce, and increments it on success — the replay guard that makes
    /// a relayed authorization usable exactly once.
    pub nonce: u32,
}

#[contracttype]
pub enum StorageKey {
    /// phone_hash -> Wallet
    Wallet(BytesN<32>),
    Admin,
}

// --- events ---------------------------------------------------------------
//
// Each event's fixed topic is its struct name in lower snake case, so the
// published topics are wallet_registered, funded, sent, cashed_out and
// pin_changed. phone_hash is marked #[topic] on every event: the USSD
// gateway indexes a user's activity by phone hash, and a topic is the only
// part of an event the network lets you filter on cheaply.
//
// Balances after the action travel in the data section so the gateway can
// confirm a USSD step with the resulting balance without a follow-up read.

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WalletRegistered {
    #[topic]
    pub phone_hash: BytesN<32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Funded {
    #[topic]
    pub phone_hash: BytesN<32>,
    pub amount: i128,
    pub balance: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sent {
    /// Both sides are topics: a gateway showing one user's history needs to
    /// find the transfers they received as well as the ones they sent.
    #[topic]
    pub from_hash: BytesN<32>,
    #[topic]
    pub to_hash: BytesN<32>,
    pub amount: i128,
    pub from_balance: i128,
    pub to_balance: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CashedOut {
    #[topic]
    pub phone_hash: BytesN<32>,
    pub amount: i128,
    pub balance: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PinChanged {
    #[topic]
    pub phone_hash: BytesN<32>,
}

// --- storage helpers ------------------------------------------------------

/// Sets the admin at deployment. Called only from the constructor.
pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&StorageKey::Admin, admin);
}

/// Returns the deployed gateway admin.
pub fn admin(env: &Env) -> Address {
    env.storage().instance().get(&StorageKey::Admin).unwrap()
}

/// Reads a wallet, or fails with WalletNotFound.
pub fn get(env: &Env, phone_hash: &BytesN<32>) -> Result<Wallet, crate::error::Error> {
    env.storage()
        .persistent()
        .get(&StorageKey::Wallet(phone_hash.clone()))
        .ok_or(crate::error::Error::WalletNotFound)
}

/// Writes a wallet back to storage.
pub fn put(env: &Env, phone_hash: &BytesN<32>, wallet: &Wallet) {
    env.storage()
        .persistent()
        .set(&StorageKey::Wallet(phone_hash.clone()), wallet);
}

pub fn emit_registered(env: &Env, phone_hash: &BytesN<32>) {
    WalletRegistered {
        phone_hash: phone_hash.clone(),
    }
    .publish(env);
}

pub fn emit_funded(env: &Env, phone_hash: &BytesN<32>, amount: i128, balance: i128) {
    Funded {
        phone_hash: phone_hash.clone(),
        amount,
        balance,
    }
    .publish(env);
}

pub fn emit_sent(
    env: &Env,
    from_hash: &BytesN<32>,
    to_hash: &BytesN<32>,
    amount: i128,
    from_balance: i128,
    to_balance: i128,
) {
    Sent {
        from_hash: from_hash.clone(),
        to_hash: to_hash.clone(),
        amount,
        from_balance,
        to_balance,
    }
    .publish(env);
}

pub fn emit_cashed_out(env: &Env, phone_hash: &BytesN<32>, amount: i128, balance: i128) {
    CashedOut {
        phone_hash: phone_hash.clone(),
        amount,
        balance,
    }
    .publish(env);
}

pub fn emit_pin_changed(env: &Env, phone_hash: &BytesN<32>) {
    PinChanged {
        phone_hash: phone_hash.clone(),
    }
    .publish(env);
}
