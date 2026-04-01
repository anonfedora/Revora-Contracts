# Compute Share Overflow Protection

## Summary

`compute_share` is used to derive holder payouts from `(amount, share_bps)`.
This hardening removes silent overflow-to-zero behavior and replaces it with an overflow-resistant decomposition that is deterministic and bounded.

## Threat Model

Potential abuse and failure modes addressed:

- Arithmetic overflow in `amount * bps` for large `i128` amounts.
- Inconsistent rounding behavior at boundary values.
- Accidental over-distribution due to intermediate overflow artifacts.

Non-goals:

- Changing payout policy semantics.
- Changing authorization boundaries.
- Expanding scope beyond contract-side arithmetic safety.

## Security Assumptions

- `revenue_share_bps` is expected to be in `[0, 10_000]`.
- Values above `10_000` are treated as invalid and return `0`.
- Revenue reporting paths are expected to be non-negative, but the helper remains total for signed `i128` and enforces output bounds for both signs.

## Implementation Strategy

Instead of computing:

- `share = (amount * bps) / 10_000`

the function computes using decomposition:

- `amount = q * 10_000 + r`
- `share = q * bps + (r * bps) / 10_000`

Properties:

- `r` is bounded to `(-10_000, 10_000)`, so `r * bps` is always safe in `i128`.
- The result is clamped to `[min(0, amount), max(0, amount)]`.
- Rounding behavior remains deterministic for both modes:
  - `Truncation`
  - `RoundHalfUp`

## Deterministic Test Coverage

The test suite now includes explicit overflow-protection cases:

- `compute_share_max_amount_full_bps_is_exact`
- `compute_share_max_amount_half_bps_rounding_is_deterministic`
- `compute_share_min_amount_full_bps_is_exact`
- `compute_share_extreme_inputs_remain_bounded`

These tests validate:

- Exactness at full share (`10_000 bps`) for `i128::MAX` and `i128::MIN`.
- Stable rounding at large odd values.
- Bound invariants under extreme signed inputs.

## Review Notes

- No auth logic was changed.
- No storage schema was changed.
- No event schema was changed.
- The change is localized to arithmetic safety and corresponding tests.
