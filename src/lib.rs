//! KoboDial — a phone-number-keyed smart wallet operated via USSD.
//!
//! Named for the kobo, the smallest unit of the Nigerian Naira.
//!
//! The problem: users on basic feature phones cannot hold private keys or
//! sign transactions — there is no wallet app on a USSD phone. A backend
//! relayer therefore submits every transaction on their behalf. The
//! contract's job is to make that safe: the relayer's signature attests
//! only that the payload matches what the user approved over USSD, while
//! the actual authorization for any fund-moving action is the user's PIN,
//! checked on-chain, plus a nonce that makes each approval usable exactly
//! once. The relayer can relay; it cannot decide.

#![no_std]

mod error;
mod wallet;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, BytesN, Env};

use crate::error::Error;
use crate::wallet::Wallet;

#[contract]
pub struct KoboDial;

/// The authorization check shared by send and cash_out: the PIN hash must
/// match the stored hash, and the supplied nonce must equal the wallet's
/// current nonce exactly. Exact equality, not >= : a stale nonce is a
/// replay of an action the user already approved and the contract already
/// executed — the second execution must fail, not silently skip ahead.
fn authorize(wallet: &Wallet, pin_hash: &BytesN<32>, nonce: u32) -> Result<(), Error> {
    if &wallet.pin_hash != pin_hash {
        return Err(Error::InvalidPin);
    }
    if wallet.nonce != nonce {
        return Err(Error::InvalidNonce);
    }
    Ok(())
}

#[contractimpl]
impl KoboDial {
    /// Sets the gateway admin. Called once at deployment.
    pub fn __constructor(env: Env, admin: Address) {
        wallet::set_admin(&env, &admin);
    }

    /// Creates a wallet for a phone hash with an initial PIN hash.
    /// Admin-only: registration happens through the gateway after the user
    /// dials in, and the admin's signature is the gateway's attestation
    /// that the phone number was verified over USSD.
    pub fn register(
        env: Env,
        admin: Address,
        phone_hash: BytesN<32>,
        pin_hash: BytesN<32>,
    ) -> Result<(), Error> {
        if admin != wallet::admin(&env) {
            return Err(Error::Unauthorized);
        }
        admin.require_auth();

        // Re-registering an existing wallet would reset its PIN — a
        // takeover of the balance. Refuse explicitly so the gateway can
        // tell the user the number is already enrolled.
        if env
            .storage()
            .persistent()
            .has(&wallet::StorageKey::Wallet(phone_hash.clone()))
        {
            return Err(Error::AlreadyRegistered);
        }

        let w = Wallet {
            pin_hash,
            balance: 0,
            nonce: 0,
        };
        wallet::put(&env, &phone_hash, &w);
        wallet::emit_registered(&env, &phone_hash);
        Ok(())
    }

    /// Agent cash-in: the admin credits a wallet's tracked balance. The
    /// admin's signature attests an agent took physical cash and the user
    /// confirmed the amount over USSD.
    pub fn fund(
        env: Env,
        admin: Address,
        phone_hash: BytesN<32>,
        amount: i128,
    ) -> Result<(), Error> {
        if admin != wallet::admin(&env) {
            return Err(Error::Unauthorized);
        }
        admin.require_auth();
        if amount <= 0 {
            return Err(Error::InsufficientBalance);
        }

        let mut w = wallet::get(&env, &phone_hash)?;
        w.balance = w
            .balance
            .checked_add(amount)
            .ok_or(Error::InsufficientBalance)?;
        wallet::put(&env, &phone_hash, &w);
        wallet::emit_funded(&env, &phone_hash, amount, w.balance);
        Ok(())
    }

    /// PIN-authorized transfer between two wallets. The relayer submits
    /// and must sign; the PIN and the exact current nonce authorize the
    /// user's side. On success the sender's nonce increments, so this
    /// approval can never be reused.
    ///
    /// The admin signature is not ceremony. `pin_hash` and `nonce` both
    /// live in this contract's storage, and Soroban contract storage is
    /// public — so on its own `pin_hash` is a bearer token published
    /// next to the balance it protects, and anyone could read it and
    /// drain the wallet. The admin's signature is what makes the pair
    /// meaningful: it attests the payload is what the user approved over
    /// USSD.
    pub fn send(
        env: Env,
        admin: Address,
        from_hash: BytesN<32>,
        to_hash: BytesN<32>,
        amount: i128,
        pin_hash: BytesN<32>,
        nonce: u32,
    ) -> Result<(), Error> {
        if admin != wallet::admin(&env) {
            return Err(Error::Unauthorized);
        }
        admin.require_auth();

        if amount <= 0 {
            return Err(Error::InsufficientBalance);
        }

        let mut from = wallet::get(&env, &from_hash)?;
        authorize(&from, &pin_hash, nonce)?;

        if from.balance < amount {
            return Err(Error::InsufficientBalance);
        }
        let mut to = wallet::get(&env, &to_hash)?;

        from.balance -= amount;
        from.nonce += 1;
        to.balance += amount;

        wallet::put(&env, &from_hash, &from);
        wallet::put(&env, &to_hash, &to);
        wallet::emit_sent(&env, &from_hash, &to_hash, amount, from.balance, to.balance);
        Ok(())
    }

    /// PIN change. The relayer must sign, and the old PIN hash must
    /// match. No nonce: changing a PIN moves no funds, so there is no
    /// transfer to replay.
    ///
    /// Admin auth matters more here than on `send`, not less. The stored
    /// `old_pin_hash` is readable from public contract storage, so
    /// without a signature anyone could present it and set a PIN of
    /// their own — a permanent takeover rather than a single transfer.
    pub fn change_pin(
        env: Env,
        admin: Address,
        phone_hash: BytesN<32>,
        old_pin_hash: BytesN<32>,
        new_pin_hash: BytesN<32>,
    ) -> Result<(), Error> {
        if admin != wallet::admin(&env) {
            return Err(Error::Unauthorized);
        }
        admin.require_auth();

        let mut w = wallet::get(&env, &phone_hash)?;
        if w.pin_hash != old_pin_hash {
            return Err(Error::InvalidPin);
        }
        w.pin_hash = new_pin_hash;
        wallet::put(&env, &phone_hash, &w);
        wallet::emit_pin_changed(&env, &phone_hash);
        Ok(())
    }

    /// PIN-authorized cash-out: the admin pays out physical cash and the
    /// wallet's tracked balance decreases. Same PIN + exact-nonce check
    /// as send.
    pub fn cash_out(
        env: Env,
        admin: Address,
        phone_hash: BytesN<32>,
        amount: i128,
        pin_hash: BytesN<32>,
        nonce: u32,
    ) -> Result<(), Error> {
        if admin != wallet::admin(&env) {
            return Err(Error::Unauthorized);
        }
        admin.require_auth();
        if amount <= 0 {
            return Err(Error::InsufficientBalance);
        }

        let mut w = wallet::get(&env, &phone_hash)?;
        authorize(&w, &pin_hash, nonce)?;

        if w.balance < amount {
            return Err(Error::InsufficientBalance);
        }
        w.balance -= amount;
        w.nonce += 1;
        wallet::put(&env, &phone_hash, &w);
        wallet::emit_cashed_out(&env, &phone_hash, amount, w.balance);
        Ok(())
    }

    /// Read-only: a wallet's tracked balance.
    pub fn get_balance(env: Env, phone_hash: BytesN<32>) -> Result<i128, Error> {
        Ok(wallet::get(&env, &phone_hash)?.balance)
    }

    /// Read-only: a wallet's current nonce. The gateway reads this when a
    /// USSD session starts so the action it relays carries the right one.
    pub fn get_nonce(env: Env, phone_hash: BytesN<32>) -> Result<u32, Error> {
        Ok(wallet::get(&env, &phone_hash)?.nonce)
    }
}
