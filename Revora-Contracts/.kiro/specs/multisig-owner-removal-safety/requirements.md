# Requirements Document

## Introduction

This feature hardens the multisig owner removal flow in the Revora Contracts Soroban/Rust smart contract.
The current `RemoveOwner` proposal action already guards against dropping below the threshold, but several
security-critical edge cases are unaddressed: removing a non-existent owner, removing the last owner,
removing an owner who has a pending (unexecuted) proposal, and ensuring the threshold invariant is
enforced atomically at execution time. This feature adds production-grade safety checks, deterministic
event emission, and comprehensive test coverage for all abuse and failure paths.

## Glossary

- **Multisig**: The on-chain multi-signature admin system that requires `threshold` out of `N` owners to
  approve a proposal before it can be executed.
- **Owner**: An `Address` registered in `MultisigOwners` storage that is authorized to propose and
  approve multisig actions.
- **Proposal**: A pending administrative action stored under `MultisigProposal(id)` that accumulates
  approvals until the threshold is met and `execute_action` is called.
- **Threshold**: The minimum number of distinct owner approvals required to execute a proposal, stored
  under `MultisigThreshold`.
- **RemoveOwner**: The `ProposalAction::RemoveOwner(Address)` variant that, when executed, removes the
  specified address from the owners list.
- **Contract**: The `RevoraRevenueShare` Soroban smart contract deployed on Stellar.
- **Executor**: Any caller of `execute_action`; no auth is required for execution (threshold acts as
  the authorization gate).
- **Proposer**: An owner who calls `propose_action`; their approval is automatically counted.

---

## Requirements

### Requirement 1: Owner Existence Validation on Removal

**User Story:** As a multisig owner, I want removal proposals to fail at execution if the target address
is not currently an owner, so that stale or duplicate proposals do not silently succeed.

#### Acceptance Criteria

1. WHEN `execute_action` is called for a `RemoveOwner(addr)` proposal AND `addr` is not present in the
   current `MultisigOwners` list, THEN THE Contract SHALL return `RevoraError::NotAuthorized`.
2. WHEN `execute_action` is called for a `RemoveOwner(addr)` proposal AND `addr` is present in the
   current `MultisigOwners` list, THE Contract SHALL remove `addr` from the owners list and persist the
   updated list.
3. THE Contract SHALL perform the existence check against the owners list at execution time, not at
   proposal creation time, to account for concurrent proposals that may have already removed the target.

---

### Requirement 2: Threshold Invariant Enforcement

**User Story:** As a multisig owner, I want the contract to prevent any removal that would leave fewer
owners than the current threshold, so that the multisig cannot be rendered permanently inoperable.

#### Acceptance Criteria

1. WHEN `execute_action` is called for a `RemoveOwner(addr)` proposal AND the resulting owner count
   after removal would be strictly less than the current threshold, THEN THE Contract SHALL return
   `RevoraError::LimitReached`.
2. WHEN `execute_action` is called for a `RemoveOwner(addr)` proposal AND the resulting owner count
   after removal equals the current threshold, THE Contract SHALL execute the removal successfully.
3. THE Contract SHALL evaluate the threshold invariant using the owner count after the removal, not
   before, to ensure the check is accurate.
4. IF the `MultisigOwners` list is empty after removal (i.e., the last owner is removed), THEN THE
   Contract SHALL return `RevoraError::LimitReached` regardless of the threshold value.

---

### Requirement 3: Self-Removal Safety

**User Story:** As a multisig owner, I want the contract to allow an owner to propose their own removal
only when the remaining quorum can still operate, so that an owner can voluntarily exit without bricking
the multisig.

#### Acceptance Criteria

1. WHEN a `RemoveOwner(addr)` proposal is proposed by `addr` itself AND the resulting owner count after
   removal would be less than the current threshold, THEN THE Contract SHALL return `RevoraError::LimitReached`
   at execution time.
2. WHEN a `RemoveOwner(addr)` proposal is proposed by `addr` itself AND the resulting owner count after
   removal is greater than or equal to the current threshold, THE Contract SHALL execute the removal
   successfully.
3. THE Contract SHALL NOT prevent an owner from proposing their own removal; the safety check occurs
   only at execution time.

---

### Requirement 4: Duplicate Removal Proposal Safety

**User Story:** As a multisig owner, I want a second removal proposal targeting the same address to fail
at execution if a prior proposal already removed that address, so that replayed or stale proposals
cannot corrupt the owner set.

#### Acceptance Criteria

1. WHEN two `RemoveOwner(addr)` proposals are created for the same `addr` AND the first proposal is
   executed successfully, THEN THE Contract SHALL return `RevoraError::NotAuthorized` when the second
   proposal is executed.
2. THE Contract SHALL NOT prevent creation or approval of a second removal proposal for the same address;
   the safety check occurs only at execution time.

---

### Requirement 5: Event Emission on Owner Removal

**User Story:** As an off-chain indexer, I want a deterministic event emitted whenever an owner is
successfully removed, so that I can maintain an accurate off-chain replica of the owner set.

#### Acceptance Criteria

1. WHEN `execute_action` successfully removes an owner via `RemoveOwner(addr)`, THE Contract SHALL emit
   an event with topic `prop_exe` and data containing the proposal ID.
2. THE Contract SHALL NOT emit the removal event if the removal fails (e.g., owner not found, threshold
   violated).
3. WHEN `execute_action` successfully removes an owner, THE Contract SHALL emit the event after the
   updated owners list has been persisted to storage.

---

### Requirement 6: Read-Only Owner Set Queries

**User Story:** As a developer or off-chain tool, I want to query the current owner list and threshold
at any time, so that I can verify the multisig state without submitting a transaction.

#### Acceptance Criteria

1. THE Contract SHALL expose `get_multisig_owners` returning the current `Vec<Address>` of owners, or
   an empty `Vec` if multisig is not initialized.
2. THE Contract SHALL expose `get_multisig_threshold` returning `Some(u32)` when initialized, or `None`
   when not initialized.
3. WHEN an owner is removed via a successfully executed `RemoveOwner` proposal, THE `get_multisig_owners`
   query SHALL reflect the updated list in the same ledger the proposal was executed.

---

### Requirement 7: Auth Boundary — Proposal Creation and Approval

**User Story:** As a security auditor, I want all state-mutating multisig operations to require explicit
owner authorization, so that no unauthorized party can manipulate the proposal lifecycle.

#### Acceptance Criteria

1. WHEN `propose_action` is called by an address that is not in `MultisigOwners`, THEN THE Contract
   SHALL return `RevoraError::LimitReached`.
2. WHEN `approve_action` is called by an address that is not in `MultisigOwners`, THEN THE Contract
   SHALL return `RevoraError::LimitReached`.
3. THE Contract SHALL call `require_auth()` on the proposer address before any state mutation in
   `propose_action`.
4. THE Contract SHALL call `require_auth()` on the approver address before any state mutation in
   `approve_action`.
5. THE `execute_action` function SHALL NOT require auth from any specific address; the threshold
   approval count acts as the authorization gate.

---

### Requirement 8: Proposal Lifecycle Integrity

**User Story:** As a multisig owner, I want executed proposals to be permanently marked as executed so
that they cannot be re-executed, and I want the proposal state to be queryable at any time.

#### Acceptance Criteria

1. WHEN `execute_action` is called on a proposal whose `executed` field is `true`, THEN THE Contract
   SHALL return `RevoraError::LimitReached`.
2. WHEN `execute_action` successfully executes a proposal, THE Contract SHALL set the proposal's
   `executed` field to `true` and persist it before returning.
3. WHEN `get_proposal` is called with a valid proposal ID, THE Contract SHALL return `Some(Proposal)`
   with the current state including the `executed` flag.
4. WHEN `get_proposal` is called with an ID that has never been created, THE Contract SHALL return
   `None`.

---

### Requirement 9: Threshold-Owner Count Consistency After Removal

**User Story:** As a multisig owner, I want the threshold to remain valid (≤ owner count) after any
removal, so that the multisig is always operable by the remaining owners.

#### Acceptance Criteria

1. FOR ALL valid states where `RemoveOwner` executes successfully, THE Contract SHALL maintain the
   invariant: `threshold ≤ len(MultisigOwners)`.
2. FOR ALL valid states where `RemoveOwner` executes successfully, THE Contract SHALL maintain the
   invariant: `len(MultisigOwners) ≥ 1`.
3. THE Contract SHALL NOT automatically adjust the threshold when an owner is removed; threshold
   adjustment requires a separate `SetThreshold` proposal.
