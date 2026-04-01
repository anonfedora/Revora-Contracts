# Share Sum Invariant Checks

## Overview

Every offering maintains a **running aggregate** of all holder share allocations in basis points (bps). The invariant is:

```
sum(HolderShare[offering][h] for all h) ≤ 10 000
```

10 000 bps = 100 %. The contract enforces this on every write to `set_holder_share` and `meta_set_holder_share`. A write that would push the aggregate above the ceiling is rejected with `RevoraError::ShareSumExceeded` (code 30) before any state is mutated.

---

## Motivation

Without an aggregate cap, an issuer could accidentally (or maliciously) allocate more than 100 % of revenue to holders. This would cause the contract to attempt to transfer more tokens than were deposited, leading to either a failed transfer or, in a buggy implementation, an under-funded payout pool. The invariant prevents this class of error entirely at the write layer.

---

## Storage

| Key | Type | Description |
|-----|------|-------------|
| `DataKey::TotalShareBps(OfferingId)` | `u32` | Running sum of all holder `share_bps` for the offering. Absent = 0. |

The key is scoped to `OfferingId { issuer, namespace, token }`, so two offerings never share a counter.

---

## Affected Entrypoints

| Entrypoint | Invariant check |
|------------|-----------------|
| `set_holder_share` | Reads old share for holder, computes delta, rejects if `old_sum - old_share + new_share > 10 000`. |
| `meta_set_holder_share` | Delegates to `set_holder_share_internal`; same check applies. |

Read-only entrypoints (`get_holder_share`, `get_total_share_bps`, `simulate_distribution`, `claim`) are not affected.

---

## Algorithm

```
old_share  = HolderShare[offering][holder]  // 0 if unset
old_sum    = TotalShareBps[offering]        // 0 if unset
new_sum    = old_sum - old_share + new_share

if new_sum > 10_000:
    return Err(ShareSumExceeded)            // no state mutation

HolderShare[offering][holder] = new_share
TotalShareBps[offering]       = new_sum
```

Saturating arithmetic is used for the subtraction (`saturating_sub`) to guard against any hypothetical state corruption where `old_share > old_sum`.

---

## Events

Two events are emitted on every successful `set_holder_share` call:

| Event symbol | Topics | Payload | When |
|---|---|---|---|
| `share_set` | `(issuer, namespace, token)` | `(holder, share_bps)` | Per-holder share stored. |
| `share_sum` | `(issuer, namespace, token)` | `(previous_total_bps: u32, new_total_bps: u32)` | Aggregate updated. |

The `share_sum` event lets off-chain indexers track the aggregate without re-reading all holder shares.

---

## Public Query

```rust
pub fn get_total_share_bps(
    env: Env,
    issuer: Address,
    namespace: Symbol,
    token: Address,
) -> u32
```

Returns the current aggregate for an offering. Returns 0 if no shares have been set. This is a read-only view; it cannot be manipulated directly.

---

## Error Code

| Code | Name | Meaning |
|------|------|---------|
| 30 | `ShareSumExceeded` | The requested share would push the per-offering aggregate above 10 000 bps. Reduce another holder's share first. |

---

## Security Assumptions

1. **Single write path.** `TotalShareBps` is only written by `set_holder_share_internal`. All public entrypoints that modify holder shares call this helper, so there is no bypass.

2. **Atomic check-then-write.** The invariant check and the storage writes happen in the same Soroban transaction. There is no TOCTOU window.

3. **Issuer auth required.** `set_holder_share` requires `issuer.require_auth()`. An attacker cannot set shares for an offering they do not control.

4. **Scoped per offering.** The aggregate counter is keyed by `OfferingId { issuer, namespace, token }`. Shares in one offering cannot affect the counter of another.

5. **No on-chain enforcement of minimum sum.** The contract does not require the sum to equal exactly 10 000. An issuer may allocate less than 100 % (e.g., to retain a platform fee portion off-chain). This is intentional.

6. **Testnet mode does not bypass this check.** Unlike `revenue_share_bps` validation, the share-sum invariant is always enforced regardless of testnet mode.

---

## Abuse / Failure Paths

| Scenario | Outcome |
|----------|---------|
| Issuer sets one holder to 10 000 bps, then tries to add a second holder | `ShareSumExceeded` on the second call; first holder's share is unchanged. |
| Issuer sets holder A to 9 000, then tries to set holder B to 1 001 | `ShareSumExceeded`; aggregate stays at 9 000. |
| Issuer updates holder A from 5 000 → 3 000, then sets holder B to 7 000 | Accepted; aggregate = 10 000. |
| Issuer sets holder A to 0 (removes contribution) | Accepted; aggregate decreases by the old value. |
| Two offerings for the same issuer/token in different namespaces | Each has its own counter; they are fully isolated. |

---

## Integration Notes

- Before calling `set_holder_share`, integrators should call `get_total_share_bps` to check available headroom: `headroom = 10_000 - get_total_share_bps(...)`.
- When redistributing shares (e.g., after a token transfer), reduce outgoing holders first, then increase incoming holders, to avoid transient `ShareSumExceeded` errors.
- The `simulate_distribution` entrypoint does not enforce the invariant (it is read-only and accepts arbitrary `holder_shares` inputs). Use it for previews only.

---

## Test Coverage

Tests are in `src/test.rs` under the `// ── Share-sum invariant tests ──` section:

| Test | What it verifies |
|------|-----------------|
| `share_sum_starts_at_zero` | Initial state is 0. |
| `share_sum_reflects_single_holder` | Single holder sets aggregate correctly. |
| `share_sum_accumulates_across_holders` | Multiple holders sum correctly. |
| `share_sum_at_ceiling_is_accepted` | Exactly 10 000 is accepted. |
| `share_sum_rejects_overflow_by_one` | 10 001 is rejected with `ShareSumExceeded`. |
| `share_sum_unchanged_after_rejected_write` | Failed write does not mutate state. |
| `share_sum_delta_on_update` | Update uses delta, not replacement. |
| `share_sum_reduce_then_add_succeeds` | Reducing one holder makes room for another. |
| `share_sum_zero_share_removes_contribution` | Setting share to 0 decrements aggregate. |
| `share_sum_is_scoped_per_offering` | Two offerings have independent counters. |
| `share_sum_event_emitted_on_set` | `share_sum` event is emitted on every write. |
| `share_sum_abuse_second_holder_after_full_allocation` | All overflow attempts are blocked. |
