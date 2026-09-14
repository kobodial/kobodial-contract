# Security policy

KoboDial holds user balances and is operated on behalf of users who
cannot inspect it. Please treat findings accordingly.

## Reporting

Report anything that moves funds without the user's authorization, or
that lets an authorization be used more than once, **privately** — use
GitHub's "Report a vulnerability" on this repository rather than opening
a public issue. Include the entry point, the arguments, and the wallet
state before and after.

## The trust model, stated plainly

This is the part worth understanding before reporting, because some of
it looks like a vulnerability and is in fact the design:

- **The relayer (admin) is trusted for liveness, not for authorization.**
  It can refuse to submit a user's transaction, delay it, or stop
  serving entirely. It cannot move a user's funds: `send` and `cash_out`
  require the user's PIN hash and the wallet's exact current nonce, both
  checked on-chain.
- **The relayer can censor.** There is no on-chain path for a user to
  act without it, because a feature-phone user has no key to sign with.
  This is a real limitation of the model, not an oversight.
- **`fund` is admin-only and unbacked on-chain in the MVP.** It credits a
  balance to reflect cash an agent physically received. A compromised
  admin can credit balances it did not receive cash for. Settlement
  against a real token is future scope.
- **PIN entropy is small.** A 4-digit PIN has 10,000 possibilities, and
  the contract does not rate-limit attempts — the gateway must, and a
  deployment that does not rate-limit PIN attempts off-chain is
  misconfigured. On-chain lockout is tracked as future scope.
- **`pin_hash` is supplied by the gateway.** The contract compares the
  hash it is given against the hash it stored; it cannot verify that the
  gateway hashed what the user actually typed. The gateway is trusted to
  hash honestly — a compromised gateway can authorize as any user whose
  PIN hash it has seen.

## Out of scope

- The gateway, the USSD channel, and the SIM/telecom layer.
- Deployments that skip off-chain PIN rate limiting.
- PIN brute force against the on-chain contract by the admin itself,
  which the trust model above already grants.
