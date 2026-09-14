---
name: Feature request
about: Suggest something for KoboDial
labels: enhancement
---

**The USSD problem you're trying to solve**

What a user is trying to do on a feature phone, and what stops them
today. Start from the flow — "an agent needs to reverse a cash-in they
keyed wrong" — rather than the mechanism.

**What you'd like**

The shape you imagine: a new entry point, a change to an existing one,
a new event for the gateway to index.

**How it stays authorized**

KoboDial's whole design is that the relayer can relay but never decide.
If your feature moves funds, say how the user authorizes it — which PIN
check, which nonce — and what happens when the relayer submits
something the user did not approve.

**Scope note**

Multi-token support, interest/yield and admin fee logic are deliberately
out of MVP scope. They are welcome as discussion, but will not be
merged into the MVP contract.
