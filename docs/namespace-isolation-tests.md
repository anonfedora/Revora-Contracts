# Namespace Isolation in Revora

Revora supports multi-tenant deployments through the use of **Namespaces**. This document details the security assumptions, implementation details, and verification strategy for namespace isolation.

## Security Overview

The goal of namespace isolation is to ensure that different issuers or the same issuer across different logical groups (namespaces) cannot access or modify each other's data.

### Key Concepts

- **TenantId**: Composed of `(issuer: Address, namespace: Symbol)`. Used to group offerings.
- **OfferingId**: Composed of `(issuer: Address, namespace: Symbol, token: Address)`. Used for per-offering storage keys.

## Isolation Mechanism

Isolation is achieved at the storage level by incorporating the `namespace` and the original `issuer` address into all persistent storage keys.

### Storage Keys

All data related to an offering is keyed by its `OfferingId`:
- Blacklists and Whitelists
- Concentration limits and reported values
- Revenue reports and period indices
- Configuration (rounding modes, thresholds, windows)

Because Soroban storage keys are XDR-encoded, the unique combination of `(Issuer, Namespace, Token)` ensures distinct storage locations with no possibility of collision.

### Authorization & Tenure

1. **Existence Verification**: Most operations now explicitly verify that an offering exists under the provided `(Issuer, Namespace, Token)` triple before allowing mutations.
2. **Current Issuer Check**: The contract tracks the current owner of an offering. Authorization (`require_auth`) is enforced against the **current issuer**, even if the offering was originally created by a different address in the same namespace.
3. **Admin Override**: The system admin has authority across all namespaces for emergency actions (e.g., managing blacklists), as defined in the contract security policy.

## Security Assumptions

- **Namespace Uniqueness**: Issuers are responsible for managing their own namespaces. Different issuers using the same namespace name (e.g., "default") are still isolated because their underlying `Address` is different.
- **Issuer Trust**: It is assumed that the `issuer` address is secure. Namespace isolation protects against cross-issuer leakage, not against a compromised issuer account.
- **Storage Integrity**: We rely on the Soroban host environment to ensure that distinct storage keys do not collide.

## Verification Strategy

Namespace isolation is verified through comprehensive unit tests in `src/test_namespaces.rs` and `src/test.rs`.

### Test Scenarios

- **Isolated Storage**: Verifying that registering identical tokens in different namespaces results in separate revenue pools and blacklists.
- **Unauthorized Cross-Access**: Attempting to modify a namespace's state using an unauthorized issuer address.
- **Transfer Integrity**: Ensuring that transferring an offering to a new issuer maintains its namespace-scoped data while updating owner-based access controls.
- **Ghost State Prevention**: Ensuring that operations on non-existent namespaces/offerings fail with `OfferingNotFound`.
