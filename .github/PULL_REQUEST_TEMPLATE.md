## What this changes and why

<!-- The problem this solves, not just a restatement of the diff. -->

## Checklist

- [ ] `make check` passes locally (fmt, clippy, tests, wasm build)
- [ ] If this touches an entry point that moves funds or checks the
      PIN/nonce, I added a test for the failure path, not just success
- [ ] If this adds a new failure mode, it returns a typed `Error`
      variant rather than panicking
- [ ] No phone numbers, PINs, or PIN hashes appear anywhere in this
      diff — hashes are computed off-chain by the gateway
