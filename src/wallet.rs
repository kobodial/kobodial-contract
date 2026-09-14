//! Wallet storage: the data model, the storage keys, and the events.
//!
//! Keyed by phone_hash — the SHA-256 of the user's phone number. Raw phone
//! numbers are PII and never touch the chain; the gateway hashes them
//! off-chain and only the hash is ever passed to the contract. The same
//! applies to PINs: only pin_hash is stored, hashed off-chain by the
//! gateway. The contract can verify a hash it is given but can never learn
//! the PIN itself.

use soroban_sdk::{contracttype, symbol_short, Address, BytesN, Env};

/// The gateway admin's address, set once at deployment. The admin is the
/// relayer service's own Soroban account — the only account that ever
/// signs transactions against this contract. Users never hold keys; the
/// PIN inside the payload is their authorization, and the admin's
/// signature only attests that the payload is what the user approved over
/// USSD, never that the admin approves the action itself.
const ADMIN_KEY: &str = "Admin";

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

#[contracttype]
#[derive(Debug)]
pub struct WalletRegistered {
    pub phone_hash: BytesN<32>,
}

#[contracttype]
#[derive(Debug)]
pub struct Funded {
    pub phone_hash: BytesN<32>,
    pub amount: i128,
    pub balance: i128,
}

#[contracttype]
#[derive(Debug)]
pub struct Sent {
    pub from_hash: BytesN<32>,
    pub to_hash: BytesN<32>,
    pub amount: i128,
    pub from_balance: i128,
    pub to_balance: i128,
}

#[contracttype]
#[derive(Debug)]
pub struct CashedOut {
    pub phone_hash: BytesN<32>,
    pub amount: i128,
    pub balance: i128,
}

#[contracttype]
#[derive(Debug)]
pub struct PinChanged {
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
    env.events().publish(
        (symbol_short!("reged"),),
        WalletRegistered { phone_hash: phone_hash.clone() },
    );
}

pub fn emit_funded(env: &Env, phone_hash: &BytesN<32>, amount: i128, balance: i128) {
    env.events().publish(
        (symbol_short!("funded"),),
        Funded { phone_hash: phone_hash.clone(), amount, balance },
    );
}

pub fn emit_sent(
    env: &Env,
    from_hash: &BytesN<32>,
    to_hash: &BytesN<32>,
    amount: i128,
    from_balance: i128,
    to_balance: i128,
) {
    env.events().publish(
        (symbol_short!("sent"),),
        Sent { from_hash: from_hash.clone(), to_hash: to_hash.clone(), amount, from_balance, to_balance },
    );
}

pub fn emit_cashed_out(env: &Env, phone_hash: &BytesN<32>, amount: i128, balance: i128) {
    env.events().publish(
        (symbol_short!("cout"),),
        CashedOut { phone_hash: phone_hash.clone(), amount, balance },
    );
}

pub fn emit_pin_changed(env: &Env, phone_hash: &BytesN<32>) {
    env.events().publish(
        (symbol_short!("pinch"),),
        PinChanged { phone_hash: phone_hash.clone() },
    );
}
