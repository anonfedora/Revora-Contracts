//! Structured Error Coverage Expansion
//!
//! One test per `RevoraError` variant (discriminants 1-30).
//! Each test triggers the exact error and asserts the discriminant is stable.
#![cfg(test)]
#![allow(warnings)]

use crate::{RevoraError, RevoraRevenueShare, RevoraRevenueShareClient, RoundingMode};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger as _},
    Address, Env, String as SdkString,
};

fn client(env: &Env) -> RevoraRevenueShareClient<'_> {
    let id = env.register_contract(None, RevoraRevenueShare);
    RevoraRevenueShareClient::new(env, &id)
}

fn with_offering(env: &Env) -> (RevoraRevenueShareClient<'_>, Address, Address) {
    let c = client(env);
    let issuer = Address::generate(env);
    let token = Address::generate(env);
    c.register_offering(&issuer, &symbol_short!("ns"), &token, &500, &token, &0);
    (c, issuer, token)
}

#[test]
fn sec_error_1_invalid_revenue_share_bps() {
    assert_eq!(RevoraError::InvalidRevenueShareBps as u32, 1);
    let env = Env::default();
    env.mock_all_auths();
    let c = client(&env);
    let issuer = Address::generate(&env);
    let token = Address::generate(&env);
    let r = c.try_register_offering(&issuer, &symbol_short!("ns"), &token, &10_001, &token, &0);
    assert_eq!(r, Err(Ok(RevoraError::InvalidRevenueShareBps)));
}

#[test]
fn sec_error_2_limit_reached() {
    assert_eq!(RevoraError::LimitReached as u32, 2);
    let env = Env::default();
    env.mock_all_auths();
    let c = client(&env);
    let admin = Address::generate(&env);
    c.set_admin(&admin);
    let r = c.try_set_admin(&admin);
    assert_eq!(r, Err(Ok(RevoraError::LimitReached)));
}

#[test]
fn sec_error_3_concentration_limit_exceeded() {
    assert_eq!(RevoraError::ConcentrationLimitExceeded as u32, 3);
    let env = Env::default();
    env.mock_all_auths();
    let (c, issuer, token) = with_offering(&env);
    c.set_concentration_limit(&issuer, &symbol_short!("ns"), &token, &5_000, &true);
    c.report_concentration(&issuer, &symbol_short!("ns"), &token, &6_000);
    let r = c.try_report_revenue(&issuer, &symbol_short!("ns"), &token, &token, &1_000, &1, &false);
    assert_eq!(r, Err(Ok(RevoraError::ConcentrationLimitExceeded)));
}

#[test]
fn sec_error_4_offering_not_found() {
    assert_eq!(RevoraError::OfferingNotFound as u32, 4);
    let env = Env::default();
    env.mock_all_auths();
    let c = client(&env);
    let issuer = Address::generate(&env);
    let token = Address::generate(&env);
    let r = c.try_deposit_revenue(&issuer, &symbol_short!("ns"), &token, &token, &1_000, &1);
    assert_eq!(r, Err(Ok(RevoraError::OfferingNotFound)));
}

#[test]
fn sec_error_5_period_already_deposited() {
    assert_eq!(RevoraError::PeriodAlreadyDeposited as u32, 5);
    let env = Env::default();
    env.mock_all_auths();
    let (c, issuer, token) = with_offering(&env);
    c.test_insert_period(&issuer, &symbol_short!("ns"), &token, &1, &1_000);
    let r = c.try_deposit_revenue(&issuer, &symbol_short!("ns"), &token, &token, &1_000, &1);
    assert_eq!(r, Err(Ok(RevoraError::PeriodAlreadyDeposited)));
}

#[test]
fn sec_error_6_no_pending_claims() {
    assert_eq!(RevoraError::NoPendingClaims as u32, 6);
    let env = Env::default();
    env.mock_all_auths();
    let (c, issuer, token) = with_offering(&env);
    let holder = Address::generate(&env);
    let r = c.try_claim(&holder, &issuer, &symbol_short!("ns"), &token, &0);
    assert_eq!(r, Err(Ok(RevoraError::NoPendingClaims)));
}

#[test]
fn sec_error_7_holder_blacklisted() {
    assert_eq!(RevoraError::HolderBlacklisted as u32, 7);
    let env = Env::default();
    env.mock_all_auths();
    let (c, issuer, token) = with_offering(&env);
    let holder = Address::generate(&env);
    c.set_holder_share(&issuer, &symbol_short!("ns"), &token, &holder, &5_000);
    c.blacklist_add(&issuer, &issuer, &symbol_short!("ns"), &token, &holder);
    let r = c.try_claim(&holder, &issuer, &symbol_short!("ns"), &token, &0);
    assert_eq!(r, Err(Ok(RevoraError::HolderBlacklisted)));
}

#[test]
fn sec_error_8_invalid_share_bps() {
    assert_eq!(RevoraError::InvalidShareBps as u32, 8);
    let env = Env::default();
    env.mock_all_auths();
    let (c, issuer, token) = with_offering(&env);
    let holder = Address::generate(&env);
    let r = c.try_set_holder_share(&issuer, &symbol_short!("ns"), &token, &holder, &10_001);
    assert_eq!(r, Err(Ok(RevoraError::InvalidShareBps)));
}

#[test]
fn sec_error_9_payment_token_mismatch() {
    assert_eq!(RevoraError::PaymentTokenMismatch as u32, 9);
    let env = Env::default();
    env.mock_all_auths();
    let (c, issuer, token) = with_offering(&env);
    let other = Address::generate(&env);
    let r = c.try_deposit_revenue(&issuer, &symbol_short!("ns"), &token, &other, &1_000, &1);
    assert_eq!(r, Err(Ok(RevoraError::PaymentTokenMismatch)));
}

#[test]
fn sec_error_10_contract_frozen() {
    assert_eq!(RevoraError::ContractFrozen as u32, 10);
    let env = Env::default();
    env.mock_all_auths();
    let c = client(&env);
    let admin = Address::generate(&env);
    c.set_admin(&admin);
    c.freeze();
    let issuer = Address::generate(&env);
    let token = Address::generate(&env);
    let r = c.try_register_offering(&issuer, &symbol_short!("ns"), &token, &500, &token, &0);
    assert_eq!(r, Err(Ok(RevoraError::ContractFrozen)));
}

#[test]
fn sec_error_11_claim_delay_not_elapsed() {
    assert_eq!(RevoraError::ClaimDelayNotElapsed as u32, 11);
    let env = Env::default();
    env.mock_all_auths();
    let (c, issuer, token) = with_offering(&env);
    let holder = Address::generate(&env);
    c.set_holder_share(&issuer, &symbol_short!("ns"), &token, &holder, &5_000);
    c.set_claim_delay(&issuer, &symbol_short!("ns"), &token, &86_400);
    c.test_insert_period(&issuer, &symbol_short!("ns"), &token, &1, &1_000);
    let r = c.try_claim(&holder, &issuer, &symbol_short!("ns"), &token, &1);
    assert_eq!(r, Err(Ok(RevoraError::ClaimDelayNotElapsed)));
}

#[test]
fn sec_error_12_snapshot_not_enabled() {
    assert_eq!(RevoraError::SnapshotNotEnabled as u32, 12);
    let env = Env::default();
    env.mock_all_auths();
    let (c, issuer, token) = with_offering(&env);
    let r = c.try_deposit_revenue_with_snapshot(
        &issuer, &symbol_short!("ns"), &token, &token, &1_000, &1, &42,
    );
    assert_eq!(r, Err(Ok(RevoraError::SnapshotNotEnabled)));
}

#[test]
fn sec_error_13_outdated_snapshot() {
    assert_eq!(RevoraError::OutdatedSnapshot as u32, 13);
    let env = Env::default();
    env.mock_all_auths();
    let (c, issuer, token) = with_offering(&env);
    c.set_snapshot_config(&issuer, &symbol_short!("ns"), &token, &true);
    // snapshot_reference=0 <= last_snap=0
    let r = c.try_deposit_revenue_with_snapshot(
        &issuer, &symbol_short!("ns"), &token, &token, &1_000, &1, &0,
    );
    assert_eq!(r, Err(Ok(RevoraError::OutdatedSnapshot)));
}

#[test]
fn sec_error_14_payout_asset_mismatch() {
    assert_eq!(RevoraError::PayoutAssetMismatch as u32, 14);
    let env = Env::default();
    env.mock_all_auths();
    let (c, issuer, token) = with_offering(&env);
    let wrong = Address::generate(&env);
    let r = c.try_report_revenue(
        &issuer, &symbol_short!("ns"), &token, &wrong, &1_000, &1, &false,
    );
    assert_eq!(r, Err(Ok(RevoraError::PayoutAssetMismatch)));
}

#[test]
fn sec_error_15_issuer_transfer_pending() {
    assert_eq!(RevoraError::IssuerTransferPending as u32, 15);
    let env = Env::default();
    env.mock_all_auths();
    let (c, issuer, token) = with_offering(&env);
    let new_issuer = Address::generate(&env);
    c.propose_issuer_transfer(&issuer, &symbol_short!("ns"), &token, &new_issuer);
    let r = c.try_propose_issuer_transfer(&issuer, &symbol_short!("ns"), &token, &new_issuer);
    assert_eq!(r, Err(Ok(RevoraError::IssuerTransferPending)));
}

#[test]
fn sec_error_16_no_transfer_pending() {
    assert_eq!(RevoraError::NoTransferPending as u32, 16);
    let env = Env::default();
    env.mock_all_auths();
    let (c, issuer, token) = with_offering(&env);
    let r = c.try_cancel_issuer_transfer(&issuer, &symbol_short!("ns"), &token);
    assert_eq!(r, Err(Ok(RevoraError::NoTransferPending)));
}

/// Security: typed error (not panic) so callers can distinguish wrong-acceptor
/// from no-pending-transfer.
#[test]
fn sec_error_17_unauthorized_transfer_accept() {
    assert_eq!(RevoraError::UnauthorizedTransferAccept as u32, 17);
    let env = Env::default();
    env.mock_all_auths();
    let (c, issuer, token) = with_offering(&env);
    let new_issuer = Address::generate(&env);
    let wrong = Address::generate(&env);
    c.propose_issuer_transfer(&issuer, &symbol_short!("ns"), &token, &new_issuer);
    let r = c.try_accept_issuer_transfer(&wrong, &issuer, &symbol_short!("ns"), &token);
    assert_eq!(r, Err(Ok(RevoraError::UnauthorizedTransferAccept)));
}

#[test]
fn sec_error_18_metadata_too_large() {
    assert_eq!(RevoraError::MetadataTooLarge as u32, 18);
    let env = Env::default();
    env.mock_all_auths();
    let (c, issuer, token) = with_offering(&env);
    let mut buf = [b'a'; 257];
    buf[0] = b'i'; buf[1] = b'p'; buf[2] = b'f'; buf[3] = b's';
    buf[4] = b':'; buf[5] = b'/'; buf[6] = b'/';
    let meta = SdkString::from_str(&env, core::str::from_utf8(&buf).unwrap());
    let r = c.try_set_offering_metadata(&issuer, &symbol_short!("ns"), &token, &meta);
    assert_eq!(r, Err(Ok(RevoraError::MetadataTooLarge)));
}

#[test]
fn sec_error_19_not_authorized() {
    assert_eq!(RevoraError::NotAuthorized as u32, 19);
    use crate::MetaSetHolderSharePayload;
    let env = Env::default();
    env.mock_all_auths();
    let (c, issuer, token) = with_offering(&env);
    let signer = Address::generate(&env);
    let holder = Address::generate(&env);
    let payload = MetaSetHolderSharePayload {
        issuer,
        namespace: symbol_short!("ns"),
        token,
        holder,
        share_bps: 1_000,
    };
    let fake_sig = soroban_sdk::BytesN::from_array(&env, &[0u8; 64]);
    let r = c.try_meta_set_holder_share(&signer, &payload, &1, &u64::MAX, &fake_sig);
    assert_eq!(r, Err(Ok(RevoraError::NotAuthorized)));
}

/// set_testnet_mode before admin is set returns NotInitialized (not a panic).
#[test]
fn sec_error_20_not_initialized() {
    assert_eq!(RevoraError::NotInitialized as u32, 20);
    let env = Env::default();
    env.mock_all_auths();
    let c = client(&env);
    let r = c.try_set_testnet_mode(&false);
    assert_eq!(r, Err(Ok(RevoraError::NotInitialized)));
}

#[test]
fn sec_error_21_invalid_amount() {
    assert_eq!(RevoraError::InvalidAmount as u32, 21);
    let env = Env::default();
    env.mock_all_auths();
    let (c, issuer, token) = with_offering(&env);
    let r = c.try_report_revenue(&issuer, &symbol_short!("ns"), &token, &token, &-1, &1, &false);
    assert_eq!(r, Err(Ok(RevoraError::InvalidAmount)));
}

#[test]
fn sec_error_22_invalid_period_id() {
    assert_eq!(RevoraError::InvalidPeriodId as u32, 22);
    let env = Env::default();
    env.mock_all_auths();
    let (c, issuer, token) = with_offering(&env);
    let r = c.try_deposit_revenue(&issuer, &symbol_short!("ns"), &token, &token, &1_000, &0);
    assert_eq!(r, Err(Ok(RevoraError::InvalidPeriodId)));
}

#[test]
fn sec_error_23_supply_cap_exceeded() {
    assert_eq!(RevoraError::SupplyCapExceeded as u32, 23);
    let env = Env::default();
    env.mock_all_auths();
    let c = client(&env);
    let issuer = Address::generate(&env);
    let token = Address::generate(&env);
    c.register_offering(&issuer, &symbol_short!("ns"), &token, &500, &token, &500);
    c.test_insert_period(&issuer, &symbol_short!("ns"), &token, &1, &400);
    let r = c.try_deposit_revenue(&issuer, &symbol_short!("ns"), &token, &token, &200, &2);
    assert_eq!(r, Err(Ok(RevoraError::SupplyCapExceeded)));
}

#[test]
fn sec_error_24_metadata_invalid_format() {
    assert_eq!(RevoraError::MetadataInvalidFormat as u32, 24);
    let env = Env::default();
    env.mock_all_auths();
    let (c, issuer, token) = with_offering(&env);
    let meta = SdkString::from_str(&env, "ftp://bad-scheme");
    let r = c.try_set_offering_metadata(&issuer, &symbol_short!("ns"), &token, &meta);
    assert_eq!(r, Err(Ok(RevoraError::MetadataInvalidFormat)));
}

#[test]
fn sec_error_25_reporting_window_closed() {
    assert_eq!(RevoraError::ReportingWindowClosed as u32, 25);
    let env = Env::default();
    env.mock_all_auths();
    let (c, issuer, token) = with_offering(&env);
    c.set_report_window(&issuer, &symbol_short!("ns"), &token, &9_999_999_000, &9_999_999_999);
    let r = c.try_report_revenue(&issuer, &symbol_short!("ns"), &token, &token, &1_000, &1, &false);
    assert_eq!(r, Err(Ok(RevoraError::ReportingWindowClosed)));
}

#[test]
fn sec_error_26_claim_window_closed() {
    assert_eq!(RevoraError::ClaimWindowClosed as u32, 26);
    let env = Env::default();
    env.mock_all_auths();
    let (c, issuer, token) = with_offering(&env);
    let holder = Address::generate(&env);
    c.set_holder_share(&issuer, &symbol_short!("ns"), &token, &holder, &5_000);
    c.test_insert_period(&issuer, &symbol_short!("ns"), &token, &1, &1_000);
    c.set_claim_window(&issuer, &symbol_short!("ns"), &token, &9_999_999_000, &9_999_999_999);
    let r = c.try_claim(&holder, &issuer, &symbol_short!("ns"), &token, &1);
    assert_eq!(r, Err(Ok(RevoraError::ClaimWindowClosed)));
}

#[test]
fn sec_error_27_signature_expired() {
    assert_eq!(RevoraError::SignatureExpired as u32, 27);
    use crate::MetaSetHolderSharePayload;
    let env = Env::default();
    env.mock_all_auths();
    let (c, issuer, token) = with_offering(&env);
    let signer = Address::generate(&env);
    let holder = Address::generate(&env);
    c.set_meta_delegate(&issuer, &symbol_short!("ns"), &token, &signer);
    let payload = MetaSetHolderSharePayload {
        issuer,
        namespace: symbol_short!("ns"),
        token,
        holder,
        share_bps: 1_000,
    };
    let fake_sig = soroban_sdk::BytesN::from_array(&env, &[0u8; 64]);
    env.ledger().with_mut(|l| l.timestamp = 1);
    let r = c.try_meta_set_holder_share(&signer, &payload, &1, &0, &fake_sig);
    assert_eq!(r, Err(Ok(RevoraError::SignatureExpired)));
}

/// Discriminant stability check. Full replay path requires a valid ed25519 sig.
#[test]
fn sec_error_28_signature_replay_discriminant_stable() {
    assert_eq!(RevoraError::SignatureReplay as u32, 28);
}

#[test]
fn sec_error_29_signer_key_not_registered() {
    assert_eq!(RevoraError::SignerKeyNotRegistered as u32, 29);
    use crate::MetaSetHolderSharePayload;
    let env = Env::default();
    env.mock_all_auths();
    let (c, issuer, token) = with_offering(&env);
    let signer = Address::generate(&env);
    let holder = Address::generate(&env);
    c.set_meta_delegate(&issuer, &symbol_short!("ns"), &token, &signer);
    let payload = MetaSetHolderSharePayload {
        issuer,
        namespace: symbol_short!("ns"),
        token,
        holder,
        share_bps: 1_000,
    };
    let fake_sig = soroban_sdk::BytesN::from_array(&env, &[0u8; 64]);
    let r = c.try_meta_set_holder_share(&signer, &payload, &1, &u64::MAX, &fake_sig);
    assert_eq!(r, Err(Ok(RevoraError::SignerKeyNotRegistered)));
}

#[test]
fn sec_error_30_share_sum_exceeded() {
    assert_eq!(RevoraError::ShareSumExceeded as u32, 30);
    let env = Env::default();
    env.mock_all_auths();
    let (c, issuer, token) = with_offering(&env);
    let h1 = Address::generate(&env);
    let h2 = Address::generate(&env);
    c.set_holder_share(&issuer, &symbol_short!("ns"), &token, &h1, &9_000);
    let r = c.try_set_holder_share(&issuer, &symbol_short!("ns"), &token, &h2, &2_000);
    assert_eq!(r, Err(Ok(RevoraError::ShareSumExceeded)));
}

/// All 30 discriminants are distinct and cover the range 1..=30.
#[test]
fn sec_all_discriminants_unique_and_contiguous() {
    let codes: [u32; 30] = [
        RevoraError::InvalidRevenueShareBps as u32,
        RevoraError::LimitReached as u32,
        RevoraError::ConcentrationLimitExceeded as u32,
        RevoraError::OfferingNotFound as u32,
        RevoraError::PeriodAlreadyDeposited as u32,
        RevoraError::NoPendingClaims as u32,
        RevoraError::HolderBlacklisted as u32,
        RevoraError::InvalidShareBps as u32,
        RevoraError::PaymentTokenMismatch as u32,
        RevoraError::ContractFrozen as u32,
        RevoraError::ClaimDelayNotElapsed as u32,
        RevoraError::SnapshotNotEnabled as u32,
        RevoraError::OutdatedSnapshot as u32,
        RevoraError::PayoutAssetMismatch as u32,
        RevoraError::IssuerTransferPending as u32,
        RevoraError::NoTransferPending as u32,
        RevoraError::UnauthorizedTransferAccept as u32,
        RevoraError::MetadataTooLarge as u32,
        RevoraError::NotAuthorized as u32,
        RevoraError::NotInitialized as u32,
        RevoraError::InvalidAmount as u32,
        RevoraError::InvalidPeriodId as u32,
        RevoraError::SupplyCapExceeded as u32,
        RevoraError::MetadataInvalidFormat as u32,
        RevoraError::ReportingWindowClosed as u32,
        RevoraError::ClaimWindowClosed as u32,
        RevoraError::SignatureExpired as u32,
        RevoraError::SignatureReplay as u32,
        RevoraError::SignerKeyNotRegistered as u32,
        RevoraError::ShareSumExceeded as u32,
    ];
    for (i, &code) in codes.iter().enumerate() {
        assert_eq!(code, (i + 1) as u32, "discriminant mismatch at index {}", i);
    }
}
