---
name: Bug report
about: Something in the contract behaves wrong
labels: bug
---

**What happened**

A clear description of the incorrect behaviour.

**What you expected**

What the contract should have done instead.

**How to reproduce**

1. The entry point and arguments (hashes, amounts, nonce — never real
   phone numbers or PINs, and never a real PIN hash from production).
2. The wallet's state beforehand: balance and nonce, from `get_balance`
   and `get_nonce`.
3. What you saw: the error variant returned, or the resulting state.

**Environment**

- Network: testnet / futurenet / local quickstart
- Contract ID, if deployed
- `soroban-sdk` version and `stellar` CLI version

**Does it move money?**

If this lets a caller move funds they should not be able to move, or
lets an authorization be replayed, please say so plainly at the top and
do not post a working reproduction publicly — see SECURITY.md.
