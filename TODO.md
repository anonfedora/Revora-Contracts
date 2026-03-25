# TODO: Period Ordering Invariants Implementation
Current Working Directory: c:/Users/user/Desktop/Revora-Contracts

## Approved Plan Steps (Completed: [ ])
### 1. [x] Create TODO.md (tracking progress)
### 2. [ ] Update src/lib.rs:
   - Add DataKey::LastPeriodId(OfferingId): u64
   - report_revenue: require period_id > LastPeriodId -> Err(InvalidPeriodId); set LastPeriodId
   - deposit_revenue: same check; require period_id > 0
   - Update RevoraError doc
### 3. [ ] Update src/test.rs:
   - Test sequential deposits succeed
   - Test duplicate/non-increasing fails (Err::InvalidPeriodId)
   - Test sparse fails (e.g., deposit 1 then 3)
   - Fuzz non-monotonic sequences (reject)
### 4. [ ] Create docs/period-ordering-invariants.md (docs)
### 5. [ ] Validate:
   - cargo check
   - cargo test
   - cargo clippy
### 6. [ ] Git branch: feature/contracts-028-period-ordering-invariants
### 7. [ ] Git commit changes
### 8. [ ] attempt_completion

Next step: Edit src/lib.rs
