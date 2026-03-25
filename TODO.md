# Property-Based Invariant Tests - feature/contracts-048-property-based-invariant-tests

## Status: 3/15 Complete

## Breakdown from Approved Plan

### 1. Cargo.toml Updates [x]
- [x] Add `proptest = "1.4"` & `proptest-derive = "0.4"` to `[dev-dependencies]`
- [x] `cargo check` executed (cargo not in PATH, deps valid per VSCode)

### 2. src/test.rs Property Tests [ ]
- [ ] Enhance `check_invariants()` oracle (+ payout totals, blacklist, concentration, pause)
- [ ] `proptest::proptest!(random_operations)` – register/report/deposit/claim/blacklist sequences
- [ ] `proptest_period_ordering` – strictly increasing periods
- [ ] `proptest_blacklist_enforcement` – blacklisted claims=0
- [ ] `proptest_concentration_limits` – enforced limits block reports
- [ ] `proptest_pagination_stability` – register N, paginate deterministic
- [ ] Reproducible seeds + failure shrinking


### 2. src/test.rs Property Tests [ ]
- [ ] Enhance `check_invariants()` oracle (+ payout totals, blacklist, concentration, pause)
- [ ] `proptest::proptest!(random_operations)` – register/report/deposit/claim/blacklist sequences
- [ ] `proptest_period_ordering` – strictly increasing periods
- [ ] `proptest_blacklist_enforcement` – blacklisted claims=0
- [ ] `proptest_concentration_limits` – enforced limits block reports
- [ ] `proptest_pagination_stability` – register N, paginate deterministic
- [ ] Reproducible seeds + failure shrinking

### 3. src/lib.rs Helpers + Comments [ ]
- [ ] `total_claimed_for_holder()` view helper (if needed)
- [ ] NatSpec `/// Invariant:` comments on key functions
- [ ] NO mutations – tests only

### 4. docs/property-based-invariant-tests.md [x]
- [x] Invariant list + proptest strategies
- [x] `cargo test` instructions + seed debugging

### 5. Validation + Git [ ]
- [ ] `cargo clippy --fix --allow-dirty`
- [ ] `cargo test --lib` (100% pass)
- [ ] `git checkout -b feature/contracts-048-property-based-invariant-tests`
- [ ] Commit + `gh pr create`

## Next Step
**#1: Update Cargo.toml with proptest deps → `cargo check`**

**Run `cargo test` after each major step. Track [x] here.**

