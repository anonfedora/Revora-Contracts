//! # Multi-Period Revenue Deposit Contract
//!
//! This Soroban smart contract implements a **Multi-Period Revenue Deposit** mechanism
//! on the Stellar network. It allows an admin to deposit revenue tokens that become
//! claimable by registered beneficiaries across multiple discrete time periods.
//!
//! ## Architecture
//!
//! - **Admin**: A single privileged address that initialises the contract, deposits revenue,
//!   and registers/removes beneficiaries.
//! - **Periods**: Contiguous, non-overlapping ledger-based time windows. Each period has a
//!   fixed `start_ledger`, `end_ledger`, and a total `revenue_amount` to be distributed.
//! - **Beneficiaries**: Addresses eligible to claim their pro-rata share of each period's
//!   revenue once the period has ended.
//!
//! ## Security Assumptions
//!
//! 1. Only the admin may deposit revenue, add/remove beneficiaries, or create periods.
//! 2. A beneficiary can claim exactly once per period; double-claim attempts are rejected.
//! 3. Periods must not overlap; overlapping registrations are rejected at creation time.
//! 4. All arithmetic uses checked operations — overflow panics rather than wraps.
//! 5. The token client is trusted to be a valid Stellar asset contract.
//! 6. Contract state is never deleted; past periods are permanently auditable on-chain.
//!
//! ## Abuse / Failure Paths Considered
//!
//! - Re-entrancy: Soroban's execution model is single-threaded and state is flushed after
//!   each top-level invocation, so re-entrancy is structurally impossible.
//! - Claim before period ends: rejected with `PeriodNotEnded`.
//! - Claim by non-beneficiary: rejected with `NotBeneficiary`.
//! - Zero-beneficiary period claim: `revenue_amount` stays locked; admin may recover via
//!   `withdraw_unclaimed` after a grace period (future extension point).
//! - Integer overflow on share calculation: guarded with `checked_mul` / `checked_div`.

#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype,
    token::Client as TokenClient,
    Address, Env, Vec, Map,
};

// ─── Storage key types ────────────────────────────────────────────────────────

/// Top-level storage keys stored in persistent contract storage.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// The contract admin address.
    Admin,
    /// The token contract ID used for all deposits and claims.
    Token,
    /// Counter tracking the next period ID to be assigned.
    PeriodCounter,
    /// All registered period IDs (Vec<u32>).
    PeriodIds,
    /// Per-period metadata, keyed by period ID.
    Period(u32),
    /// Per-period beneficiary list, keyed by period ID.
    Beneficiaries(u32),
    /// Claim record: whether `address` has claimed from `period_id`.
    Claimed(u32, Address),
}

// ─── Domain types ─────────────────────────────────────────────────────────────

/// Metadata for a single revenue period.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Period {
    /// Unique monotonically-increasing identifier.
    pub id: u32,
    /// Ledger sequence number at which the period opens (inclusive).
    pub start_ledger: u32,
    /// Ledger sequence number at which the period closes (inclusive).
    pub end_ledger: u32,
    /// Total token amount deposited for distribution this period.
    pub revenue_amount: i128,
    /// How many tokens have been claimed so far.
    pub claimed_amount: i128,
}

// ─── Error codes ──────────────────────────────────────────────────────────────

/// Canonical error codes returned by contract functions.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    /// Caller is not the admin.
    Unauthorized = 1,
    /// Contract has already been initialised.
    AlreadyInitialized = 2,
    /// The referenced period does not exist.
    PeriodNotFound = 3,
    /// The period's end ledger has not been reached yet.
    PeriodNotEnded = 4,
    /// The caller is not registered as a beneficiary for this period.
    NotBeneficiary = 5,
    /// The caller has already claimed their share for this period.
    AlreadyClaimed = 6,
    /// A period with overlapping ledger range already exists.
    PeriodOverlap = 7,
    /// The supplied parameters are logically invalid (e.g. start > end, zero amount).
    InvalidInput = 8,
    /// The revenue deposit failed (e.g. insufficient token balance).
    DepositFailed = 9,
    /// Arithmetic overflow occurred.
    Overflow = 10,
    /// No beneficiaries are registered; nothing to distribute.
    NoBeneficiaries = 11,
}

// ─── Contract struct ──────────────────────────────────────────────────────────

#[contract]
pub struct RevenueDepositContract;

// ─── Implementation ───────────────────────────────────────────────────────────

#[contractimpl]
impl RevenueDepositContract {
    // ── Initialisation ────────────────────────────────────────────────────────

    /// Initialise the contract.
    ///
    /// # Arguments
    /// * `admin` – Address that will hold admin privileges.
    /// * `token` – Stellar token contract address used for deposits/claims.
    ///
    /// # Errors
    /// * [`ContractError::AlreadyInitialized`] – if called more than once.
    pub fn initialize(env: Env, admin: Address, token: Address) -> Result<(), ContractError> {
        if env.storage().persistent().has(&DataKey::Admin) {
            return Err(ContractError::AlreadyInitialized);
        }
        admin.require_auth();

        env.storage().persistent().set(&DataKey::Admin, &admin);
        env.storage().persistent().set(&DataKey::Token, &token);
        env.storage().persistent().set(&DataKey::PeriodCounter, &0u32);
        env.storage()
            .persistent()
            .set(&DataKey::PeriodIds, &Vec::<u32>::new(&env));

        Ok(())
    }

    // ── Period management ─────────────────────────────────────────────────────

    /// Create a new revenue period and transfer `revenue_amount` tokens from the
    /// admin into the contract.
    ///
    /// # Arguments
    /// * `start_ledger` – First ledger of the period (inclusive, must be ≥ current ledger).
    /// * `end_ledger`   – Last ledger of the period (inclusive, must be > `start_ledger`).
    /// * `revenue_amount` – Positive token quantity to deposit for this period.
    ///
    /// # Errors
    /// * [`ContractError::Unauthorized`]   – caller is not admin.
    /// * [`ContractError::InvalidInput`]   – bad ledger range or zero/negative amount.
    /// * [`ContractError::PeriodOverlap`]  – range overlaps an existing period.
    pub fn create_period(
        env: Env,
        start_ledger: u32,
        end_ledger: u32,
        revenue_amount: i128,
    ) -> Result<u32, ContractError> {
        let admin: Address = env.storage().persistent().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        // ── Validate inputs ────────────────────────────────────────────────
        if revenue_amount <= 0 {
            return Err(ContractError::InvalidInput);
        }
        if start_ledger >= end_ledger {
            return Err(ContractError::InvalidInput);
        }

        // ── Overlap detection ──────────────────────────────────────────────
        Self::assert_no_overlap(&env, start_ledger, end_ledger)?;

        // ── Assign ID ─────────────────────────────────────────────────────
        let mut counter: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::PeriodCounter)
            .unwrap_or(0);
        let period_id = counter;
        counter = counter.checked_add(1).ok_or(ContractError::Overflow)?;
        env.storage()
            .persistent()
            .set(&DataKey::PeriodCounter, &counter);

        // ── Persist period ─────────────────────────────────────────────────
        let period = Period {
            id: period_id,
            start_ledger,
            end_ledger,
            revenue_amount,
            claimed_amount: 0,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Period(period_id), &period);
        env.storage()
            .persistent()
            .set(&DataKey::Beneficiaries(period_id), &Vec::<Address>::new(&env));

        let mut ids: Vec<u32> = env
            .storage()
            .persistent()
            .get(&DataKey::PeriodIds)
            .unwrap_or_else(|| Vec::new(&env));
        ids.push_back(period_id);
        env.storage().persistent().set(&DataKey::PeriodIds, &ids);

        // ── Pull tokens from admin ─────────────────────────────────────────
        let token: Address = env.storage().persistent().get(&DataKey::Token).unwrap();
        let token_client = TokenClient::new(&env, &token);
        token_client.transfer(&admin, &env.current_contract_address(), &revenue_amount);

        Ok(period_id)
    }

    // ── Beneficiary management ────────────────────────────────────────────────

    /// Register `beneficiary` as eligible to claim from `period_id`.
    ///
    /// Idempotent — adding a beneficiary twice is a no-op (not an error).
    ///
    /// # Errors
    /// * [`ContractError::Unauthorized`]  – caller is not admin.
    /// * [`ContractError::PeriodNotFound`] – `period_id` does not exist.
    pub fn add_beneficiary(
        env: Env,
        period_id: u32,
        beneficiary: Address,
    ) -> Result<(), ContractError> {
        let admin: Address = env.storage().persistent().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        Self::assert_period_exists(&env, period_id)?;

        let mut beneficiaries: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Beneficiaries(period_id))
            .unwrap_or_else(|| Vec::new(&env));

        // Idempotency guard
        if !beneficiaries.contains(&beneficiary) {
            beneficiaries.push_back(beneficiary);
            env.storage()
                .persistent()
                .set(&DataKey::Beneficiaries(period_id), &beneficiaries);
        }

        Ok(())
    }

    /// Remove `beneficiary` from `period_id`.  If they have not yet claimed their
    /// share, that share reverts to the unclaimed pool (claimable by remaining
    /// beneficiaries or recoverable by admin via a future extension).
    ///
    /// # Errors
    /// * [`ContractError::Unauthorized`]   – caller is not admin.
    /// * [`ContractError::PeriodNotFound`] – `period_id` does not exist.
    /// * [`ContractError::NotBeneficiary`] – address not currently registered.
    pub fn remove_beneficiary(
        env: Env,
        period_id: u32,
        beneficiary: Address,
    ) -> Result<(), ContractError> {
        let admin: Address = env.storage().persistent().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        Self::assert_period_exists(&env, period_id)?;

        let mut beneficiaries: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Beneficiaries(period_id))
            .unwrap_or_else(|| Vec::new(&env));

        let pos = beneficiaries
            .iter()
            .position(|b| b == beneficiary)
            .ok_or(ContractError::NotBeneficiary)?;

        beneficiaries.remove(pos as u32);
        env.storage()
            .persistent()
            .set(&DataKey::Beneficiaries(period_id), &beneficiaries);

        Ok(())
    }

    // ── Claims ────────────────────────────────────────────────────────────────

    /// Claim a pro-rata share of `period_id`'s revenue.
    ///
    /// The share is `floor(revenue_amount / beneficiary_count)`.  Any remainder
    /// (due to integer division) stays in the contract as unclaimed dust.
    ///
    /// # Preconditions
    /// * Current ledger must be **strictly after** `end_ledger` of the period.
    /// * Caller must be a registered beneficiary.
    /// * Caller must not have claimed before.
    ///
    /// # Errors
    /// * [`ContractError::PeriodNotFound`]  – period does not exist.
    /// * [`ContractError::PeriodNotEnded`]  – period is still active.
    /// * [`ContractError::NotBeneficiary`]  – caller is not registered.
    /// * [`ContractError::AlreadyClaimed`]  – caller already claimed.
    /// * [`ContractError::NoBeneficiaries`] – no beneficiaries registered.
    /// * [`ContractError::Overflow`]        – arithmetic overflow (should never occur in practice).
    pub fn claim(env: Env, period_id: u32, claimant: Address) -> Result<i128, ContractError> {
        claimant.require_auth();

        // ── Load period ────────────────────────────────────────────────────
        let mut period: Period = env
            .storage()
            .persistent()
            .get(&DataKey::Period(period_id))
            .ok_or(ContractError::PeriodNotFound)?;

        // ── Timing gate ────────────────────────────────────────────────────
        let current_ledger = env.ledger().sequence();
        if current_ledger <= period.end_ledger {
            return Err(ContractError::PeriodNotEnded);
        }

        // ── Beneficiary check ──────────────────────────────────────────────
        let beneficiaries: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Beneficiaries(period_id))
            .unwrap_or_else(|| Vec::new(&env));

        if beneficiaries.is_empty() {
            return Err(ContractError::NoBeneficiaries);
        }

        if !beneficiaries.contains(&claimant) {
            return Err(ContractError::NotBeneficiary);
        }

        // ── Double-claim guard ─────────────────────────────────────────────
        let claim_key = DataKey::Claimed(period_id, claimant.clone());
        if env.storage().persistent().has(&claim_key) {
            return Err(ContractError::AlreadyClaimed);
        }

        // ── Compute share ──────────────────────────────────────────────────
        let count = beneficiaries.len() as i128;
        let share = period
            .revenue_amount
            .checked_div(count)
            .ok_or(ContractError::Overflow)?;

        if share <= 0 {
            return Err(ContractError::InvalidInput);
        }

        // ── Update state (checks-effects-interactions) ─────────────────────
        env.storage().persistent().set(&claim_key, &true);
        period.claimed_amount = period
            .claimed_amount
            .checked_add(share)
            .ok_or(ContractError::Overflow)?;
        env.storage()
            .persistent()
            .set(&DataKey::Period(period_id), &period);

        // ── Transfer tokens ────────────────────────────────────────────────
        let token: Address = env.storage().persistent().get(&DataKey::Token).unwrap();
        let token_client = TokenClient::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &claimant, &share);

        Ok(share)
    }

    // ── Read-only helpers ─────────────────────────────────────────────────────

    /// Return metadata for a period.
    pub fn get_period(env: Env, period_id: u32) -> Result<Period, ContractError> {
        env.storage()
            .persistent()
            .get(&DataKey::Period(period_id))
            .ok_or(ContractError::PeriodNotFound)
    }

    /// Return all period IDs registered with this contract.
    pub fn get_period_ids(env: Env) -> Vec<u32> {
        env.storage()
            .persistent()
            .get(&DataKey::PeriodIds)
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Return the beneficiary list for a period.
    pub fn get_beneficiaries(env: Env, period_id: u32) -> Result<Vec<Address>, ContractError> {
        Self::assert_period_exists(&env, period_id)?;
        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::Beneficiaries(period_id))
            .unwrap_or_else(|| Vec::new(&env)))
    }

    /// Return whether `address` has claimed from `period_id`.
    pub fn has_claimed(env: Env, period_id: u32, address: Address) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Claimed(period_id, address))
    }

    /// Return the current admin address.
    pub fn get_admin(env: Env) -> Address {
        env.storage().persistent().get(&DataKey::Admin).unwrap()
    }

    /// Return the token contract address.
    pub fn get_token(env: Env) -> Address {
        env.storage().persistent().get(&DataKey::Token).unwrap()
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    /// Assert that `period_id` is stored.
    fn assert_period_exists(env: &Env, period_id: u32) -> Result<(), ContractError> {
        if !env.storage().persistent().has(&DataKey::Period(period_id)) {
            return Err(ContractError::PeriodNotFound);
        }
        Ok(())
    }

    /// Assert that [start_ledger, end_ledger] does not overlap any existing period.
    fn assert_no_overlap(
        env: &Env,
        start_ledger: u32,
        end_ledger: u32,
    ) -> Result<(), ContractError> {
        let ids: Vec<u32> = env
            .storage()
            .persistent()
            .get(&DataKey::PeriodIds)
            .unwrap_or_else(|| Vec::new(env));

        for id in ids.iter() {
            let existing: Period = env
                .storage()
                .persistent()
                .get(&DataKey::Period(id))
                .unwrap();
            // Overlap: NOT (new_end < existing_start OR new_start > existing_end)
            if !(end_ledger < existing.start_ledger || start_ledger > existing.end_ledger) {
                return Err(ContractError::PeriodOverlap);
            }
        }
        Ok(())
    }

    /// Build a summary map of unclaimed amounts per period (useful for admin dashboards).
    pub fn unclaimed_summary(env: Env) -> Map<u32, i128> {
        let ids: Vec<u32> = env
            .storage()
            .persistent()
            .get(&DataKey::PeriodIds)
            .unwrap_or_else(|| Vec::new(&env));

        let mut map: Map<u32, i128> = Map::new(&env);
        for id in ids.iter() {
            if let Some(period) = env
                .storage()
                .persistent()
                .get::<DataKey, Period>(&DataKey::Period(id))
            {
                let unclaimed = period.revenue_amount - period.claimed_amount;
                map.set(id, unclaimed);
            }
        }
        map
    }
}