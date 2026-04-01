# Multisig Duplicate-Approval Guards

## Overview

The `approve_action` entrypoint enforces that each multisig owner may approve a
given proposal **at most once**. A second call by the same owner returns
`RevoraError::AlreadyApproved` (error code 32) and leaves contract state
unchanged.

## Security Rationale

Threshold enforcement in `execute_action` counts the length of
`proposal.approvals`. If duplicate entries were permitted, a single owner could
call `approve_action` N times to inflate the count and satisfy an N-of-M
threshold alone — effectively bypassing the multisig requirement entirely.

The guard ensures `proposal.approvals` is a **set**: each owner address appears
at most once. This is the minimal invariant required for threshold enforcement to
be meaningful.

## Implementation

```rust
// In approve_action — runs after auth and owner checks, before list mutation.
for i in 0..proposal.approvals.len() {
    if proposal.approvals.get(i).unwrap() == approver {
        return Err(RevoraError::AlreadyApproved);
    }
}
proposal.approvals.push_back(approver.clone());
```

**Complexity:** O(M) where M is the number of registered owners. Because M is
fixed at `init_multisig` time and realistic multisig sizes are small (≤ 10),
this is safe for on-chain execution.

**No state mutation on rejection:** When `AlreadyApproved` is returned, the
proposal is not written back to storage and no event is emitted.

## Error Code

| Code | Name | Meaning |
|------|------|---------|
| 32 | `AlreadyApproved` | The caller has already approved this proposal. |

Use `try_approve_action` to receive this as a `Result` rather than a host panic.

## Check Order in `approve_action`

1. `approver.require_auth()` — auth failure panics (host-level).
2. `require_multisig_owner` — returns `LimitReached` if not an owner.
3. Proposal lookup — returns `OfferingNotFound` if proposal does not exist.
4. Executed check — returns `LimitReached` if already executed.
5. **Duplicate guard** — returns `AlreadyApproved` if already in approval list.
6. Append + persist + emit `prop_app` event.

## Security Assumptions

- **Owner list is authoritative.** Only addresses in `MultisigOwners` can call
  `approve_action`. The owner list is set at `init_multisig` and modified only
  via executed `AddOwner`/`RemoveOwner` proposals.
- **Approval list is append-only.** Entries are never removed from
  `proposal.approvals`; the duplicate guard is the only write gate.
- **Threshold is validated at execution time.** `execute_action` reads the
  current threshold from storage; a `SetThreshold` proposal that raises the
  threshold will retroactively require more approvals on any pending proposals
  that have not yet been executed.
- **No time-lock or expiry.** Proposals do not expire. A stale proposal can be
  executed at any time once threshold is met. For production deployments,
  consider adding an expiry timestamp.

## Abuse Paths and Mitigations

| Abuse path | Mitigation |
|------------|------------|
| Owner submits `approve_action` twice to inflate count | Duplicate guard returns `AlreadyApproved`; count unchanged |
| Owner proposes then immediately approves again | Auto-approval on `propose_action` counts as the first; second call rejected |
| Non-owner calls `approve_action` | `require_multisig_owner` returns `LimitReached` before guard runs |
| Approving an executed proposal | Executed check returns `LimitReached` before guard runs |
| Approving a non-existent proposal | Proposal lookup returns `OfferingNotFound` before guard runs |
| Duplicate approvals across independent proposals | Each proposal has its own `approvals` list; isolation is per-proposal |

## Test Coverage

All paths are covered in `src/test.rs` under the multisig section:

| Test | What it validates |
|------|-------------------|
| `multisig_duplicate_approval_is_idempotent` | Proposer's auto-approval + second call → `AlreadyApproved`; count stays 1 |
| `multisig_duplicate_approval_proposer_returns_already_approved` | Same as above, explicit error assertion |
| `multisig_duplicate_approval_second_owner_returns_already_approved` | Non-proposer owner double-approves → `AlreadyApproved` |
| `multisig_duplicate_approval_all_owners_approve_once_each` | All 3 owners approve once; count = 3, no duplicates |
| `multisig_duplicate_approval_emits_no_event_on_rejection` | Rejected duplicate emits no `prop_app` event |
| `multisig_duplicate_approval_on_executed_proposal_returns_limit_reached` | Executed check fires before duplicate guard |
| `multisig_duplicate_approval_cannot_satisfy_threshold` | Single owner cannot reach threshold=2 via double-approval |
| `multisig_duplicate_approval_nonexistent_proposal_returns_not_found` | Proposal lookup fires before duplicate guard |
| `multisig_duplicate_approval_independent_proposals_isolated` | Duplicate on p1 does not affect p2 approval count |
