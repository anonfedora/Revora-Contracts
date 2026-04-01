# Per-Offering Emergency Pause

## Overview
The Per-Offering Emergency Pause mechanism allows authorized roles (Admin, Safety, Issuer) to halt all state-mutating operations for a specific offering without affecting the rest of the contract. This granular control is essential for managing individual offering risks or responding to suspicious activities localized to a single issuance.

## Security Roles and Authorizations
The following roles are authorized to pause or unpause an offering:
- **Global Admin**: Full control to pause/unpause ANY offering.
- **Safety Role**: Dedicated emergency role (configured during initialization) authorized to pause/unpause any offering.
- **Current Issuer**: The current authorized issuer of the specific offering may pause it at any time.

## Protected Entrypoints
When an offering is paused, the following state-mutating functions will return `RevoraError::OfferingPaused` (code `31`):
- `do_deposit_revenue`
- `report_revenue`
- `blacklist_add` / `blacklist_remove`
- `whitelist_add` / `whitelist_remove`
- `set_concentration_limit` / `report_concentration`
- `set_rounding_mode`
- `set_investment_constraints`
- `set_min_revenue_threshold`
- `set_snapshot_config`
- `set_holder_share`
- `set_meta_delegate`
- `meta_set_holder_share`
- `meta__approve_revenue_report`
- `claim`
- `set_report_window` / `set_claim_window` / `set_claim_delay`
- `set_offering_metadata`

## Implicit Assumptions & Security Notes
1. **Flash-Loan Resistance**: Pause checks are performed at the beginning of each state-mutating call. Even within the same transaction, if an offering is paused, all subsequent mutating calls will fail.
2. **Read-Only Access**: View functions (e.g., `get_offering`, `get_holder_share`, `get_claimable`) remain operational during a pause to allow users to verify their state.
3. **Issuer Autonomy**: Allowing issuers to pause their own offerings ensures they can act faster than a global admin if they detect an issue with their specific offering's off-chain reporting.
4. **State Persistence**: The pause state is stored in persistent storage under `DataKey::PausedOffering(OfferingId)` and survives contract upgrades and Ledger ttl extensions.

## Storage Layout
```rust
pub enum DataKey {
    // ... other keys ...
    /// Per-offering pause flag; when true, state-mutating ops for that offering are disabled.
    PausedOffering(OfferingId),
}
```

## Error Codes
- `RevoraError::OfferingPaused` (31): Returned when an operation is attempted on a paused offering.
