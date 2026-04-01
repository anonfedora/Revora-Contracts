# Multisig Owner Removal Safety

## Overview

This document describes the safety guarantees enforced by the `RemoveOwner` execution path in the
`RevoraRevenueShare` Soroban smart contract. All checks are evaluated atomically at execution time.

---

## Guards in `execute_action` — `RemoveOwner` branch

Guards are evaluated in strict order. The first failing guard short-circuits and returns an error
without mutating any state.

| Order | Guard                                                            | Error returned            |
| ----- | ---------------------------------------------------------------- | ------------------------- |
| 1     | Proposal exists in storage                                       | panic (storage invariant) |
| 2     | `proposal.executed == false`                                     | `LimitReached`            |
| 3     | `proposal.approvals.len() >= threshold`                          | `LimitReached`            |
| 4     | `addr` is present in current `MultisigOwners` list               | `NotAuthorized`           |
| 5     | `owners.len() - 1 >= threshold` (post-removal count ≥ threshold) | `LimitReached`            |

After all guards pass, the contract:

1. Removes `addr` from the owners list
2. Persists the updated owners list to `DataKey::MultisigOwners`
3. Sets `proposal.executed = true` and persists the proposal
4. Emits a `prop_exe` event with the proposal ID as data

---

## Security Assumptions

- **Execution-time checks only.** Both the existence check (guard 4) and the threshold invariant
  (guard 5) are evaluated when `execute_action` is called, not when the proposal is created. This
  correctly handles concurrent proposals: if two proposals target the same owner and the first
  executes successfully, the second will fail with `NotAuthorized`.

- **No threshold auto-adjustment.** Removing an owner never changes the threshold. If the remaining
  owner count equals the threshold after removal, the multisig remains operable (all remaining owners
  must agree). To lower the threshold, a separate `SetThreshold` proposal is required.

- **Last-owner protection.** Removing the sole owner always fails because the post-removal count
  would be 0, which is less than any valid threshold (≥ 1).

- **Self-removal is allowed at proposal time.** An owner may propose their own removal. The safety
  check is deferred to execution time, where the threshold invariant is enforced.

- **`execute_action` requires no auth.** Any caller may trigger execution once the threshold approval
  count is met. The threshold acts as the authorization gate.

---

## Post-Removal Invariants

After any successful `RemoveOwner` execution, the following invariants hold:

```
threshold ≤ len(MultisigOwners)
len(MultisigOwners) ≥ 1
```

---

## Read-Only Query Functions

### `get_multisig_owners(env: Env) -> Vec<Address>`

Returns the current list of multisig owners. Returns an empty `Vec` if the multisig has not been
initialized.

```rust
let owners = client.get_multisig_owners();
```

### `get_multisig_threshold(env: Env) -> Option<u32>`

Returns `Some(threshold)` if the multisig is initialized, `None` otherwise.

```rust
let threshold = client.get_multisig_threshold(); // Some(2) or None
```

Both functions are read-only and require no authorization.

---

## Error Reference

| Scenario                                                   | Error           |
| ---------------------------------------------------------- | --------------- |
| `RemoveOwner(addr)` where `addr` is not in the owners list | `NotAuthorized` |
| `RemoveOwner(addr)` where post-removal count < threshold   | `LimitReached`  |
| `RemoveOwner(addr)` where `addr` is the sole owner         | `LimitReached`  |
| `execute_action` on an already-executed proposal           | `LimitReached`  |
| `execute_action` with insufficient approvals               | `LimitReached`  |
| `propose_action` or `approve_action` by a non-owner        | `LimitReached`  |

---

## Off-Chain Usage Example

Query the current multisig state before submitting a removal proposal:

```typescript
// Using stellar-sdk or soroban-client
const owners = await contract.get_multisig_owners();
const threshold = await contract.get_multisig_threshold();

console.log(`Owners: ${owners.length}, Threshold: ${threshold}`);
// Safe to remove if owners.length - 1 >= threshold
if (owners.length - 1 >= threshold) {
  await contract.propose_action({
    proposer,
    action: { RemoveOwner: targetAddress },
  });
}
```

---

## Related

- `init_multisig` — initializes owners and threshold (one-time)
- `propose_action` — creates a proposal; proposer's vote is auto-counted
- `approve_action` — adds an owner's approval to a pending proposal
- `execute_action` — executes a proposal once threshold approvals are met
- `get_proposal` — returns `Some(Proposal)` or `None` for a given proposal ID
