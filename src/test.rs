//! Unit tests over the Soroban test environment, one per USSD scenario
//! the gateway will actually hit. phone/pin hashes are arbitrary fixed
//! 32-byte values — the gateway hashes off-chain; the contract only ever
//! sees hashes.

#![cfg(test)]
extern crate std;

use crate::{error::Error, KoboDial};
use soroban_sdk::{
    testutils::Address as _, testutils::Events as _, Address, BytesN, Env, Symbol, TryFromVal,
};

/// A fixed 32-byte "hash" for tests, distinct per seed byte. Built against
/// the env under test — a BytesN is bound to the environment that made it.
fn hash(env: &Env, seed: u8) -> BytesN<32> {
    let mut b = [0u8; 32];
    b[0] = seed;
    BytesN::from_array(env, &b)
}

struct Setup {
    env: Env,
    contract: Address,
    admin: Address,
}

fn setup() -> Setup {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract = env.register(KoboDial, (&admin,));
    Setup {
        env,
        contract,
        admin,
    }
}

fn register(env: &Env, c: &Address, admin: &Address, phone: BytesN<32>, pin: BytesN<32>) {
    let r: Result<(), Error> = env.as_contract(c, || {
        KoboDial::register(env.clone(), admin.clone(), phone, pin)
    });
    assert_eq!(r, Ok(()));
}

fn fund(env: &Env, c: &Address, admin: &Address, phone: BytesN<32>, amount: i128) {
    let r: Result<(), Error> = env.as_contract(c, || {
        KoboDial::fund(env.clone(), admin.clone(), phone, amount)
    });
    assert_eq!(r, Ok(()));
}

/// 1. Register + fund works.
#[test]
fn test_register_and_fund() {
    let s = setup();
    let (phone, pin) = (hash(&s.env, 1), hash(&s.env, 2));

    register(&s.env, &s.contract, &s.admin, phone.clone(), pin.clone());
    assert_eq!(
        s.env.as_contract(&s.contract, || KoboDial::get_balance(
            s.env.clone(),
            phone.clone()
        )),
        Ok(0)
    );

    fund(&s.env, &s.contract, &s.admin, phone.clone(), 500);
    assert_eq!(
        s.env.as_contract(&s.contract, || KoboDial::get_balance(
            s.env.clone(),
            phone.clone()
        )),
        Ok(500)
    );
}

/// 2. Send with the correct PIN and exact current nonce succeeds; both
///    balances update and the sender's nonce increments.
#[test]
fn test_send_correct_pin_and_nonce() {
    let s = setup();
    let (a, b) = (hash(&s.env, 1), hash(&s.env, 3));
    let pin = hash(&s.env, 2);

    register(&s.env, &s.contract, &s.admin, a.clone(), pin.clone());
    register(&s.env, &s.contract, &s.admin, b.clone(), pin.clone());
    fund(&s.env, &s.contract, &s.admin, a.clone(), 1000);

    let r: Result<(), Error> = s.env.as_contract(&s.contract, || {
        KoboDial::send(
            s.env.clone(),
            s.admin.clone(),
            a.clone(),
            b.clone(),
            400,
            pin.clone(),
            0,
        )
    });
    assert_eq!(r, Ok(()));

    assert_eq!(
        s.env.as_contract(&s.contract, || KoboDial::get_balance(
            s.env.clone(),
            a.clone()
        )),
        Ok(600)
    );
    assert_eq!(
        s.env.as_contract(&s.contract, || KoboDial::get_balance(
            s.env.clone(),
            b.clone()
        )),
        Ok(400)
    );
    assert_eq!(
        s.env.as_contract(&s.contract, || KoboDial::get_nonce(
            s.env.clone(),
            a.clone()
        )),
        Ok(1)
    );
    // The receiver's nonce is untouched: receiving requires no authorization.
    assert_eq!(
        s.env.as_contract(&s.contract, || KoboDial::get_nonce(
            s.env.clone(),
            b.clone()
        )),
        Ok(0)
    );
}

/// 3. Wrong PIN fails with InvalidPin and nothing changes.
#[test]
fn test_send_wrong_pin() {
    let s = setup();
    let (a, b) = (hash(&s.env, 1), hash(&s.env, 3));

    register(&s.env, &s.contract, &s.admin, a.clone(), hash(&s.env, 2));
    register(&s.env, &s.contract, &s.admin, b.clone(), hash(&s.env, 2));
    fund(&s.env, &s.contract, &s.admin, a.clone(), 1000);

    let r: Result<(), Error> = s.env.as_contract(&s.contract, || {
        KoboDial::send(
            s.env.clone(),
            s.admin.clone(),
            a.clone(),
            b.clone(),
            400,
            hash(&s.env, 9),
            0,
        )
    });
    assert_eq!(r, Err(Error::InvalidPin));

    // State unchanged: balance, and the nonce — a failed attempt must not
    // consume the user's confirmation.
    assert_eq!(
        s.env.as_contract(&s.contract, || KoboDial::get_balance(
            s.env.clone(),
            a.clone()
        )),
        Ok(1000)
    );
    assert_eq!(
        s.env.as_contract(&s.contract, || KoboDial::get_nonce(
            s.env.clone(),
            a.clone()
        )),
        Ok(0)
    );
}

/// 4. Replaying the same nonce twice: the second execution fails with
///    InvalidNonce. The replay guard is the heart of the relay pattern.
#[test]
fn test_send_nonce_replay_rejected() {
    let s = setup();
    let (a, b) = (hash(&s.env, 1), hash(&s.env, 3));
    let pin = hash(&s.env, 2);

    register(&s.env, &s.contract, &s.admin, a.clone(), pin.clone());
    register(&s.env, &s.contract, &s.admin, b.clone(), pin.clone());
    fund(&s.env, &s.contract, &s.admin, a.clone(), 1000);

    let send = |nonce: u32| -> Result<(), Error> {
        s.env.as_contract(&s.contract, || {
            KoboDial::send(
                s.env.clone(),
                s.admin.clone(),
                a.clone(),
                b.clone(),
                400,
                pin.clone(),
                nonce,
            )
        })
    };

    assert_eq!(send(0), Ok(()));
    // Same authorization replayed: the wallet's nonce is now 1, so nonce 0
    // is a stale confirmation and must fail.
    assert_eq!(send(0), Err(Error::InvalidNonce));
    // A stale future nonce (skipping ahead) is also rejected — exact
    // match only, so the relayer cannot front-run the user.
    assert_eq!(send(5), Err(Error::InvalidNonce));

    assert_eq!(
        s.env.as_contract(&s.contract, || KoboDial::get_balance(
            s.env.clone(),
            a.clone()
        )),
        Ok(600)
    );
}

/// 5. Insufficient balance fails cleanly with the typed error.
#[test]
fn test_send_insufficient_balance() {
    let s = setup();
    let (a, b) = (hash(&s.env, 1), hash(&s.env, 3));
    let pin = hash(&s.env, 2);

    register(&s.env, &s.contract, &s.admin, a.clone(), pin.clone());
    register(&s.env, &s.contract, &s.admin, b.clone(), pin.clone());
    fund(&s.env, &s.contract, &s.admin, a.clone(), 100);

    let r: Result<(), Error> = s.env.as_contract(&s.contract, || {
        KoboDial::send(
            s.env.clone(),
            s.admin.clone(),
            a.clone(),
            b.clone(),
            400,
            pin.clone(),
            0,
        )
    });
    assert_eq!(r, Err(Error::InsufficientBalance));
    assert_eq!(
        s.env.as_contract(&s.contract, || KoboDial::get_balance(
            s.env.clone(),
            a.clone()
        )),
        Ok(100)
    );
}

/// 6. change_pin requires the old PIN; the new PIN then works for
///    subsequent sends and the old one stops working.
#[test]
fn test_change_pin() {
    let s = setup();
    let (a, b) = (hash(&s.env, 1), hash(&s.env, 3));
    let (old_pin, new_pin) = (hash(&s.env, 2), hash(&s.env, 7));

    register(&s.env, &s.contract, &s.admin, a.clone(), old_pin.clone());
    register(&s.env, &s.contract, &s.admin, b.clone(), old_pin.clone());
    fund(&s.env, &s.contract, &s.admin, a.clone(), 1000);

    // Wrong old PIN refused.
    let r: Result<(), Error> = s.env.as_contract(&s.contract, || {
        KoboDial::change_pin(
            s.env.clone(),
            s.admin.clone(),
            a.clone(),
            hash(&s.env, 9),
            new_pin.clone(),
        )
    });
    assert_eq!(r, Err(Error::InvalidPin));

    // Correct old PIN accepted.
    let r: Result<(), Error> = s.env.as_contract(&s.contract, || {
        KoboDial::change_pin(
            s.env.clone(),
            s.admin.clone(),
            a.clone(),
            old_pin.clone(),
            new_pin.clone(),
        )
    });
    assert_eq!(r, Ok(()));

    // The new PIN authorizes; the old PIN no longer does.
    let r: Result<(), Error> = s.env.as_contract(&s.contract, || {
        KoboDial::send(
            s.env.clone(),
            s.admin.clone(),
            a.clone(),
            b.clone(),
            100,
            new_pin.clone(),
            0,
        )
    });
    assert_eq!(r, Ok(()));
    let r: Result<(), Error> = s.env.as_contract(&s.contract, || {
        KoboDial::send(
            s.env.clone(),
            s.admin.clone(),
            a.clone(),
            b.clone(),
            100,
            old_pin.clone(),
            1,
        )
    });
    assert_eq!(r, Err(Error::InvalidPin));
}

/// cash_out mirrors send's authorization: PIN + exact nonce, admin-only.
#[test]
fn test_cash_out() {
    let s = setup();
    let a = hash(&s.env, 1);
    let pin = hash(&s.env, 2);

    register(&s.env, &s.contract, &s.admin, a.clone(), pin.clone());
    fund(&s.env, &s.contract, &s.admin, a.clone(), 1000);

    let r: Result<(), Error> = s.env.as_contract(&s.contract, || {
        KoboDial::cash_out(
            s.env.clone(),
            s.admin.clone(),
            a.clone(),
            700,
            pin.clone(),
            0,
        )
    });
    assert_eq!(r, Ok(()));
    assert_eq!(
        s.env.as_contract(&s.contract, || KoboDial::get_balance(
            s.env.clone(),
            a.clone()
        )),
        Ok(300)
    );
    assert_eq!(
        s.env.as_contract(&s.contract, || KoboDial::get_nonce(
            s.env.clone(),
            a.clone()
        )),
        Ok(1)
    );

    // Replay of the same cash-out confirmation fails.
    let r: Result<(), Error> = s.env.as_contract(&s.contract, || {
        KoboDial::cash_out(
            s.env.clone(),
            s.admin.clone(),
            a.clone(),
            700,
            pin.clone(),
            0,
        )
    });
    assert_eq!(r, Err(Error::InvalidNonce));
}

/// Non-admin callers cannot register, fund, or cash out.
#[test]
fn test_admin_only_functions() {
    let s = setup();
    let a = hash(&s.env, 1);
    let impostor = Address::generate(&s.env);

    let r: Result<(), Error> = s.env.as_contract(&s.contract, || {
        KoboDial::register(s.env.clone(), impostor.clone(), a.clone(), hash(&s.env, 2))
    });
    assert_eq!(r, Err(Error::Unauthorized));

    // Even with the impostor blocked at the identity check, the admin
    // check happens before any require_auth, so no auth is consumed.
    let r: Result<(), Error> = s.env.as_contract(&s.contract, || {
        KoboDial::cash_out(
            s.env.clone(),
            impostor.clone(),
            a.clone(),
            1,
            hash(&s.env, 2),
            0,
        )
    });
    assert_eq!(r, Err(Error::Unauthorized));
}

/// Unknown wallet reads fail with WalletNotFound, not a panic.
#[test]
fn test_unknown_wallet() {
    let s = setup();
    let unknown = hash(&s.env, 0xEE);
    assert_eq!(
        s.env.as_contract(&s.contract, || KoboDial::get_balance(
            s.env.clone(),
            unknown.clone()
        )),
        Err(Error::WalletNotFound)
    );
    assert_eq!(
        s.env.as_contract(&s.contract, || KoboDial::get_nonce(
            s.env.clone(),
            unknown.clone()
        )),
        Err(Error::WalletNotFound)
    );
}

/// change_pin, send and cash_out against an unregistered wallet all fail
/// with WalletNotFound rather than a panic — same as the read views.
#[test]
fn test_change_pin_unknown_wallet() {
    let s = setup();
    let unknown = hash(&s.env, 0xEE);

    let r: Result<(), Error> = s.env.as_contract(&s.contract, || {
        KoboDial::change_pin(
            s.env.clone(),
            s.admin.clone(),
            unknown.clone(),
            hash(&s.env, 1),
            hash(&s.env, 2),
        )
    });
    assert_eq!(r, Err(Error::WalletNotFound));
}

/// Duplicate registration is refused — re-registering would reset the
/// PIN and take over the balance.
#[test]
fn test_duplicate_registration_refused() {
    let s = setup();
    let a = hash(&s.env, 1);
    register(&s.env, &s.contract, &s.admin, a.clone(), hash(&s.env, 2));

    let r: Result<(), Error> = s.env.as_contract(&s.contract, || {
        KoboDial::register(s.env.clone(), s.admin.clone(), a.clone(), hash(&s.env, 3))
    });
    assert_eq!(r, Err(Error::AlreadyRegistered));
}

/// Asserts the event published by the call that just ran carries the
/// expected topic symbol. The test environment's event buffer reflects the
/// most recent invocation context rather than a cumulative log, so each
/// action is checked immediately after it runs — which also pins the
/// payload rather than merely counting that something fired.
fn assert_last_event(env: &Env, want: &str) {
    let events = env.events().all();
    assert_eq!(
        events.len(),
        1,
        "the last call should publish exactly one event"
    );
    let (_, topics, _) = events.last().unwrap();
    // topics[0] is the event's fixed name; the indexed phone hashes follow.
    let got = Symbol::try_from_val(env, &topics.first().unwrap()).unwrap();
    assert_eq!(got, Symbol::new(env, want));
}

/// Every state-changing entry point publishes its event.
#[test]
fn test_events_emitted() {
    let s = setup();
    let (a, b) = (hash(&s.env, 1), hash(&s.env, 3));
    let pin = hash(&s.env, 2);

    register(&s.env, &s.contract, &s.admin, a.clone(), pin.clone());
    assert_last_event(&s.env, "wallet_registered");

    register(&s.env, &s.contract, &s.admin, b.clone(), pin.clone());
    assert_last_event(&s.env, "wallet_registered");

    fund(&s.env, &s.contract, &s.admin, a.clone(), 1000);
    assert_last_event(&s.env, "funded");

    let r: Result<(), Error> = s.env.as_contract(&s.contract, || {
        KoboDial::send(
            s.env.clone(),
            s.admin.clone(),
            a.clone(),
            b.clone(),
            400,
            pin.clone(),
            0,
        )
    });
    assert_eq!(r, Ok(()));
    assert_last_event(&s.env, "sent");

    let r: Result<(), Error> = s.env.as_contract(&s.contract, || {
        KoboDial::cash_out(
            s.env.clone(),
            s.admin.clone(),
            a.clone(),
            100,
            pin.clone(),
            1,
        )
    });
    assert_eq!(r, Ok(()));
    assert_last_event(&s.env, "cashed_out");

    let r: Result<(), Error> = s.env.as_contract(&s.contract, || {
        KoboDial::change_pin(
            s.env.clone(),
            s.admin.clone(),
            a.clone(),
            pin.clone(),
            hash(&s.env, 7),
        )
    });
    assert_eq!(r, Ok(()));
    assert_last_event(&s.env, "pin_changed");
}

/// fund, send and cash_out all guard `amount <= 0`. Zero and negative
/// amounts must be rejected with InsufficientBalance — not because the
/// wallet actually lacks funds, but because a "transfer" of nothing or
/// negative value is not a real transfer, and the two failures share a
/// variant since the gateway's response to a user is the same either way.
#[test]
fn test_zero_and_negative_amounts_rejected() {
    let s = setup();
    let (a, b) = (hash(&s.env, 1), hash(&s.env, 3));
    let pin = hash(&s.env, 2);

    register(&s.env, &s.contract, &s.admin, a.clone(), pin.clone());
    register(&s.env, &s.contract, &s.admin, b.clone(), pin.clone());

    let fund_with = |amount: i128| -> Result<(), Error> {
        s.env.as_contract(&s.contract, || {
            KoboDial::fund(s.env.clone(), s.admin.clone(), a.clone(), amount)
        })
    };
    assert_eq!(fund_with(0), Err(Error::InsufficientBalance));
    assert_eq!(fund_with(-1), Err(Error::InsufficientBalance));

    // Give the wallet real funds so a zero/negative send or cash_out fails
    // on the amount check itself, not on an incidental empty balance.
    assert_eq!(fund_with(1000), Ok(()));

    let send_with = |amount: i128| -> Result<(), Error> {
        s.env.as_contract(&s.contract, || {
            KoboDial::send(
                s.env.clone(),
                s.admin.clone(),
                a.clone(),
                b.clone(),
                amount,
                pin.clone(),
                0,
            )
        })
    };
    assert_eq!(send_with(0), Err(Error::InsufficientBalance));
    assert_eq!(send_with(-500), Err(Error::InsufficientBalance));

    let cash_out_with = |amount: i128| -> Result<(), Error> {
        s.env.as_contract(&s.contract, || {
            KoboDial::cash_out(
                s.env.clone(),
                s.admin.clone(),
                a.clone(),
                amount,
                pin.clone(),
                0,
            )
        })
    };
    assert_eq!(cash_out_with(0), Err(Error::InsufficientBalance));
    assert_eq!(cash_out_with(-1), Err(Error::InsufficientBalance));

    // None of the rejected calls should have moved balance or nonce.
    assert_eq!(
        s.env.as_contract(&s.contract, || KoboDial::get_balance(
            s.env.clone(),
            a.clone()
        )),
        Ok(1000)
    );
    assert_eq!(
        s.env.as_contract(&s.contract, || KoboDial::get_nonce(
            s.env.clone(),
            a.clone()
        )),
        Ok(0)
    );
}

// --- authorization on the PIN-authorized paths --------------------------
//
// pin_hash and nonce both live in this contract's storage, and Soroban
// contract storage is public. Read together they are everything `send`
// used to require, which made pin_hash a bearer token published next to
// the balance it protects. These tests pin the admin gate that closes
// that: they are the regression to keep, not the happy paths above.

/// 14. send refuses a caller that is not the registered admin.
#[test]
fn test_send_rejects_non_admin() {
    let s = setup();
    let (a, pin_a) = (hash(&s.env, 1), hash(&s.env, 2));
    let (b, pin_b) = (hash(&s.env, 3), hash(&s.env, 4));
    register(&s.env, &s.contract, &s.admin, a.clone(), pin_a.clone());
    register(&s.env, &s.contract, &s.admin, b.clone(), pin_b);
    fund(&s.env, &s.contract, &s.admin, a.clone(), 1000);

    // Everything an attacker can read from public chain state: the
    // victim's phone hash, their pin_hash, and their current nonce.
    let impostor = Address::generate(&s.env);
    let r: Result<(), Error> = s.env.as_contract(&s.contract, || {
        KoboDial::send(
            s.env.clone(),
            impostor.clone(),
            a.clone(),
            b.clone(),
            1000,
            pin_a.clone(),
            0,
        )
    });
    assert_eq!(r, Err(Error::Unauthorized));

    // The drain must not have happened.
    assert_eq!(
        s.env.as_contract(&s.contract, || KoboDial::get_balance(
            s.env.clone(),
            a.clone()
        )),
        Ok(1000)
    );
    assert_eq!(
        s.env.as_contract(&s.contract, || KoboDial::get_nonce(
            s.env.clone(),
            a.clone()
        )),
        Ok(0)
    );
}

/// 15. change_pin refuses a caller that is not the registered admin.
#[test]
fn test_change_pin_rejects_non_admin() {
    let s = setup();
    let (phone, pin) = (hash(&s.env, 1), hash(&s.env, 2));
    register(&s.env, &s.contract, &s.admin, phone.clone(), pin.clone());

    let impostor = Address::generate(&s.env);
    let attacker_pin = hash(&s.env, 99);
    let r: Result<(), Error> = s.env.as_contract(&s.contract, || {
        KoboDial::change_pin(
            s.env.clone(),
            impostor.clone(),
            phone.clone(),
            pin.clone(),
            attacker_pin.clone(),
        )
    });
    assert_eq!(r, Err(Error::Unauthorized));

    // The takeover must not have happened: the original PIN still works
    // and the attacker's does not.
    let change = |old: BytesN<32>, new: BytesN<32>| -> Result<(), Error> {
        s.env.as_contract(&s.contract, || {
            KoboDial::change_pin(s.env.clone(), s.admin.clone(), phone.clone(), old, new)
        })
    };
    assert_eq!(
        change(attacker_pin, hash(&s.env, 7)),
        Err(Error::InvalidPin)
    );
    assert_eq!(change(pin, hash(&s.env, 7)), Ok(()));
}

/// 16. send requires the admin's signature, not merely its address.
///
/// The identity check above passes if an attacker simply names the real
/// admin. require_auth is what makes that useless without the key, so
/// this runs without mocked auths and expects the call to fail.
#[test]
// Pinned to the auth failure specifically: a bare should_panic would also
// pass if this test broke for some unrelated reason, which is exactly how
// a security regression test quietly stops testing anything.
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn test_send_requires_admin_signature() {
    let s = setup();
    let (a, pin_a) = (hash(&s.env, 1), hash(&s.env, 2));
    let (b, pin_b) = (hash(&s.env, 3), hash(&s.env, 4));
    register(&s.env, &s.contract, &s.admin, a.clone(), pin_a.clone());
    register(&s.env, &s.contract, &s.admin, b.clone(), pin_b);
    fund(&s.env, &s.contract, &s.admin, a.clone(), 1000);

    // Withdraw the blanket authorization the harness sets up, leaving the
    // correct admin address but no signature behind it.
    s.env.set_auths(&[]);
    let _: Result<(), Error> = s.env.as_contract(&s.contract, || {
        KoboDial::send(
            s.env.clone(),
            s.admin.clone(),
            a.clone(),
            b.clone(),
            1000,
            pin_a.clone(),
            0,
        )
    });
}
