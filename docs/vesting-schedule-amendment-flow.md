# Vesting Schedule Amendment Flow

## Overview
The Vesting Schedule Amendment Flow allows the contract administrator to modify the parameters of an existing vesting schedule. This is a critical administrative feature for handling changes in team roles, performance-based adjustments, or error corrections in initial schedule setups.

## Key Features
- **Total Amount Adjustment**: Increase or decrease the total amount of tokens in the schedule.
- **Timeline Refactoring**: Update start time, cliff duration, and total duration.
- **Safety Guards**: Prevents reducing the total amount below what has already been claimed.
- **Status Validation**: Only active (non-cancelled) schedules can be amended.

## Security Assumptions and Rules
1. **Authorized Access**: Only the address initialized as `Admin` can call `amend_schedule`.
2. **Accounting Integrity**: The contract enforces `new_total_amount >= claimed_amount`. This ensures that even if a schedule is reduced, the tokens already claimed by the beneficiary remain accounted for and the schedule doesn't enter an invalid state.
3. **Parameter Validity**:
    - `new_duration_secs > 0`: Prevents division-by-zero errors in vesting calculations.
    - `new_cliff_duration_secs <= new_duration_secs`: Ensures the cliff occurs within the vesting period.
4. **Immutability of Cancelled Schedules**: Once a schedule is cancelled, it cannot be amended. This prevents "reviving" a forfeit schedule through parameter manipulation.

## Implementation Details
The `amend_schedule` function updates the `VestingSchedule` struct in persistent storage. After amendment, any subsequent calls to `get_claimable_vesting` or `claim_vesting` will use the updated parameters for linear calculation.

### Event Emission
Every successful amendment emits a `vest_amd` event containing:
- `admin`: The authorized caller.
- `beneficiary`: The recipient of the vesting.
- `schedule_index`: The specific schedule modified.
- `new_total_amount`, `new_start_time`, `new_cliff_time`, `new_end_time`.

## Example Flow
1. Admin creates a schedule for 1000 tokens over 1 year.
2. After 6 months, the beneficiary has claimed 500 tokens.
3. Admin decides to increase the total to 2000 tokens and extend the duration to 2 years.
4. Admin calls `amend_schedule` with the new parameters.
5. The beneficiary can now continue claiming based on the new 2000-token, 2-year linear curve, minus the 500 tokens already claimed.

## Technical Errors
- `AmendmentNotAllowed`: Thrown if attempting to amend a cancelled schedule.
- `InvalidAmount`: Thrown if `new_total_amount < claimed_amount`.
- `InvalidDuration` / `InvalidCliff`: Thrown if timing parameters are logically inconsistent.
