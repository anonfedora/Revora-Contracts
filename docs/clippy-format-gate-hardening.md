# Clippy/Format Gate Hardening

## Summary

This hardening introduces a per-offering quality gate that can enforce a recent successful `cargo fmt` and `cargo clippy` attestation before critical revenue write paths are allowed.

The gate is intentionally scoped to contracts logic and does not depend on external CI services at runtime.

## New Contract Capabilities

### New errors

- `GatePolicyInvalid` (31): invalid policy input.
- `GateAttestationExpired` (32): attestation is stale or timestamp-invalid.
- `GateCheckFailed` (33): missing attestation or failed gate flags.

### New storage

- `ClippyFormatGateConfig(OfferingId)`:
  - `enforce: bool`
  - `max_attestation_age_secs: u64`
- `ClippyFormatGateAttestation(OfferingId)`:
  - `attested_at: u64`
  - `format_ok: bool`
  - `clippy_ok: bool`
  - `artifact_hash: BytesN<32>`

### New events

- `gate_cfg`: emitted when gate policy is configured.
- `gate_att`: emitted when gate attestation is recorded.

### New methods

- `set_clippy_format_gate(caller, issuer, namespace, token, enforce, max_attestation_age_secs)`
- `get_clippy_format_gate(issuer, namespace, token)`
- `attest_clippy_format_gate(caller, issuer, namespace, token, format_ok, clippy_ok, artifact_hash)`
- `attest_clippy_format_gate(caller, issuer, namespace, token, attestation_input)`
- `get_clippy_format_attestation(issuer, namespace, token)`

Where `attestation_input` is:

- `format_ok: bool`
- `clippy_ok: bool`
- `artifact_hash: BytesN<32>`

## Enforcement Behavior

When a gate policy exists and `enforce = true`, the contract now requires a fresh green attestation for:

- `report_revenue`
- `deposit_revenue`
- `deposit_revenue_with_snapshot` (through shared deposit path)

A valid attestation must satisfy all of the following:

- `format_ok = true`
- `clippy_ok = true`
- `now >= attested_at`
- `now - attested_at <= max_attestation_age_secs`

If any check fails, the call returns a gate error and state does not mutate.

## Security Assumptions

- The attesting caller (issuer or admin) is trusted to submit truthful results.
- `artifact_hash` binds attestation to an off-chain build artifact and should be produced by a trusted CI workflow.
- Ledger timestamp is assumed monotonic enough for freshness checks.
- This mechanism is an application-layer guardrail, not a cryptographic proof of CI execution.

## Abuse/Failure Paths Covered

- Unauthorized policy update blocked.
- Invalid policy windows rejected (`max_attestation_age_secs = 0`, or too large).
- Missing attestation blocks revenue writes under enforced mode.
- Failed attestation flags block writes.
- Expired attestation blocks writes deterministically.
- Fresh green attestation allows writes.

## Operational Guidance

- Keep policy windows short enough to reduce stale attestation risk.
- Rotate attestation on every release candidate.
- Store CI artifact references and hash derivation details in release records.
- Use admin override carefully and monitor emitted gate events.
