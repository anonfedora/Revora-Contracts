# Token Vesting Core

## Overview
The Token Vesting Core capability is a production-grade standalone primitive for vesting token distributions to team members, advisors, and other stakeholders securely. 

## Features
- **Multiple Schedules:** Admin can create multiple vesting schedules per beneficiary.
- **Vesting Mechanism:** Supports linear vesting over time and a hard cliff.
- **Cancellations:** Admin can cleanly cancel a schedule; only the unvested portion is forfeit.
- **Claim Processing:** Beneficiaries independently claim vested tokens.

## Architecture & Security Assumptions
1. **Admin Control**: Only an initialized admin address can create or cancel schedules.
2. **Deterministic Computation**: Vested amounts are computed mathematically on-the-fly (`start_time`, `end_time`, `cliff_time`) and use saturating arithmetic to prevent underflow/overflow.
3. **Immutability of the Past**: Canceling a schedule does not impact already claimed or vested amounts that have accrued up until the cancellation threshold.
4. **Zero-Trust Claims**: Beneficiaries securely claim what is theirs over the timeline without admin intervention being strictly required, up to the mathematically provable vested amount.

## Edge Cases Mitigated
- Zero duration handling and inverted cliff bounds handling.
- Claiming prior to a cliff returning absolute zero safely.
- Safe division and saturating multiplication to avoid panic traps under network stress.
