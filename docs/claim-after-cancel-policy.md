# Claim-After-Cancel Policy

## Overview

An issuer may cancel an active offering at any time via `cancel_offering`. Cancellation is a
one-way, irreversible action that stops new revenue from being deposited or reported, while
**guaranteeing that all revenue already deposited before cancellation remains claimable** by
eligible holders.

This document describes the policy, its security model, and integration guidance.

---

## Entrypoints

### `cancel_offering(issuer, namespace, token) → Result<(), RevoraError>`

Cancels an offering. Auth: current issuer.

- Records `env.ledger().timestamp()` as the cancellation timestamp in persistent storage.
- Emits `off_canc` event with the timestamp as payload.
- **Idempotent**: calling on an already-cancelled offering returns `Ok(())` without emitting a
  second event or changing the stored timestamp.

### `get_offering_cancelled_at(issuer, namespace, token) → Option<u64>`

Returns the ledger timestamp at which the offering was cancelled, or `None` if the offering is
still active. Use `.is_some()` to check cancellation status.

---

## Behavioral Contract

| Operation | Active offering | Cancelled offering |
|-----------|----------------|--------------------|
| `deposit_revenue` | ✅ Allowed | ❌ `OfferingCancelled` |
| `report_revenue` | ✅ Allowed | ❌ `OfferingCancelled` |
| `claim` (pre-cancel periods) | ✅ Allowed | ✅ **Allowed** |
| `claim` (post-cancel periods) | N/A | Skipped (defensive guard) |
| `set_holder_share` | ✅ Allowed | ✅ Allowed |
| `blacklist_add/remove` | ✅ Allowed | ✅ Allowed |
| `get_offering_cancelled_at` | Returns `None` | Returns `Some(ts)` |

### Claim-After-Cancel Guarantee

When `claim` is called on a cancelled offering:

1. The cancellation timestamp is read once from storage.
2. The claim loop iterates unclaimed periods in deposit order.
3. Any period whose `PeriodDepositTime ≤ cancelled_at` is processed normally (payout computed
   and accumulated).
4. Any period whose `PeriodDepositTime > cancelled_at` is skipped and its index is advanced
   (defensive guard — in practice this branch is unreachable because `deposit_revenue` already
   blocks post-cancel deposits).
5. The claim delay (`ClaimDelaySecs`) is still enforced per period.
6. Blacklisted holders are still blocked.
7. Zero-share holders still receive `NoPendingClaims`.

Holder funds already transferred to the contract are **never destroyed** by cancellation.

---

## Security Assumptions

| Assumption | Enforcement |
|------------|-------------|
| Only the current issuer can cancel | `current_issuer.require_auth()` inside `cancel_offering` |
| Cancellation timestamp cannot be forged | Taken from `env.ledger().timestamp()`, not a caller parameter |
| Frozen contract blocks cancellation | `require_not_frozen` guard at entry |
| Cancellation is scoped per `(issuer, namespace, token)` | `OfferingCancelledAt(OfferingId)` storage key |
| Pre-cancel funds are never destroyed | `claim` loop only skips, never deletes period revenue |
| Post-cancel deposits are impossible | `do_deposit_revenue` checks `OfferingCancelledAt` before any transfer |

### Trust Boundary

The contract does **not** prevent an issuer from cancelling an offering that still has unclaimed
funds. Issuers should communicate cancellation to holders off-chain so they can claim before any
application-level deadlines. The contract itself imposes no claim deadline after cancellation.

---

## Events

| Topic | Payload | When |
|-------|---------|------|
| `off_canc`, `issuer`, `namespace`, `token` | `cancelled_at: u64` | On first successful `cancel_offering` call |

No event is emitted on idempotent (duplicate) cancel calls.

---

## Error Codes

| Code | Name | Condition |
|------|------|-----------|
| 4 | `OfferingNotFound` | Offering does not exist for `(issuer, namespace, token)` |
| 10 | `ContractFrozen` | Contract is frozen |
| 31 | `OfferingCancelled` | `deposit_revenue` or `report_revenue` called on a cancelled offering |

---

## Integration Patterns

### Off-chain: notify holders before cancellation

```
1. Issuer decides to cancel offering (token, namespace).
2. Off-chain system queries all holders with share_bps > 0.
3. Notify each holder: "Offering X is being cancelled. Claim your revenue before [date]."
4. Issuer calls cancel_offering(issuer, namespace, token).
5. Holders call claim() at their convenience — no deadline enforced on-chain.
```

### Checking cancellation status

```rust
// Returns None if active, Some(timestamp) if cancelled
let cancelled_at = client.get_offering_cancelled_at(&issuer, &ns, &token);
if cancelled_at.is_some() {
    println!("Offering cancelled at ledger time {}", cancelled_at.unwrap());
}
```

### Claiming after cancellation (holder perspective)

```rust
// Works identically to a normal claim — no special handling needed
let payout = client.claim(&holder, &issuer, &ns, &token, &50)?;
```

---

## Storage Impact

One new persistent storage entry per cancelled offering:

- **Key**: `DataKey::OfferingCancelledAt(OfferingId { issuer, namespace, token })`
- **Value**: `u64` (ledger timestamp)
- **Written**: once, on `cancel_offering`
- **Read**: on every `deposit_revenue`, `report_revenue`, and `claim` call for that offering

---

## Test Coverage

All tests are in `src/test.rs` under `mod claim_after_cancel`. Coverage includes:

| Test | What it verifies |
|------|-----------------|
| `cancel_offering_succeeds` | Happy path |
| `cancel_offering_sets_cancelled_at` | Timestamp stored correctly |
| `active_offering_cancelled_at_is_none` | No false positives |
| `cancel_offering_idempotent` | No double-event, timestamp unchanged |
| `cancel_offering_emits_event` | Event topic and payload |
| `cancel_offering_requires_issuer_auth` | Auth enforcement |
| `cancel_nonexistent_offering_fails` | `OfferingNotFound` error |
| `deposit_revenue_blocked_after_cancel` | `OfferingCancelled` error |
| `report_revenue_blocked_after_cancel` | `OfferingCancelled` error |
| `claim_pre_cancel_deposits_succeeds` | Core policy: pre-cancel funds claimable |
| `claim_multiple_pre_cancel_periods_succeeds` | Multiple periods |
| `partial_claim_after_cancel_then_rest` | Incremental claiming |
| `no_pending_claims_after_all_pre_cancel_claimed` | Exhaustion |
| `blacklisted_holder_cannot_claim_after_cancel` | Blacklist still enforced |
| `zero_share_holder_cannot_claim_after_cancel` | Zero-share still blocked |
| `multiple_holders_claim_after_cancel` | Per-holder isolation |
| `cancel_is_scoped_per_offering` | Offering isolation |
| `cancel_is_scoped_per_namespace` | Namespace isolation |
| `cancel_offering_blocked_when_frozen` | Freeze guard |
| `cancel_timestamp_is_ledger_time_not_caller_supplied` | Timestamp integrity |
