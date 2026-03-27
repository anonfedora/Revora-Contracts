use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Env,
};

use crate::vesting::{RevoraVesting, RevoraVestingClient};

fn setup(env: &Env) -> (RevoraVestingClient<'_>, Address, Address, Address) {
    let contract_id = env.register_contract(None, RevoraVesting);
    let client = RevoraVestingClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let beneficiary = Address::generate(env);
    let token_id = env.register_stellar_asset_contract(admin.clone());
    (client, admin, beneficiary, token_id)
}

fn mint_tokens(env: &Env, payment_token: &Address, recipient: &Address, amount: &i128) {
    soroban_sdk::token::StellarAssetClient::new(env, payment_token).mint(recipient, amount);
}

fn balance(env: &Env, payment_token: &Address, who: &Address) -> i128 {
    soroban_sdk::token::Client::new(env, payment_token).balance(who)
}

#[test]
fn initialize_sets_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, _b, _t) = setup(&env);
    client.initialize_vesting(&admin);
}

#[test]
fn create_schedule_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, beneficiary, token_id) = setup(&env);
    client.initialize_vesting(&admin);

    let total = 1_000_000_i128;
    let start = 1000_u64;
    let cliff = 500_u64;
    let duration = 2000_u64;

    let idx =
        client.create_schedule(&admin, &beneficiary, &token_id, &total, &start, &cliff, &duration);
    assert_eq!(idx, 0);

    let schedule = client.get_schedule(&admin, &0);
    assert_eq!(schedule.beneficiary, beneficiary);
    assert_eq!(schedule.total_amount, total);
    assert_eq!(schedule.claimed_amount, 0);
    assert_eq!(schedule.start_time, start);
    assert_eq!(schedule.cliff_time, start + cliff);
    assert_eq!(schedule.end_time, start + duration);
    assert!(!schedule.cancelled);
}

#[test]
fn get_claimable_before_cliff_is_zero() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, beneficiary, token_id) = setup(&env);
    client.initialize_vesting(&admin);

    let total = 1_000_000_i128;
    let start = 1000_u64;
    let cliff = 500_u64;
    let duration = 2000_u64;
    client.create_schedule(&admin, &beneficiary, &token_id, &total, &start, &cliff, &duration);

    env.ledger().with_mut(|l| l.timestamp = start + 100);
    let claimable = client.get_claimable_vesting(&admin, &0);
    assert_eq!(claimable, 0);
}

#[test]
fn cancel_schedule() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, beneficiary, token_id) = setup(&env);
    client.initialize_vesting(&admin);
    client.create_schedule(&admin, &beneficiary, &token_id, &1_000_000, &1000, &100, &2000);

    client.cancel_schedule(&admin, &beneficiary, &0);
    let schedule = client.get_schedule(&admin, &0);
    assert!(schedule.cancelled);
}

#[test]
fn multiple_schedules_same_beneficiary() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, beneficiary, token_id) = setup(&env);
    client.initialize_vesting(&admin);

    client.create_schedule(&admin, &beneficiary, &token_id, &100, &1000, &0, &1000);
    client.create_schedule(&admin, &beneficiary, &token_id, &200, &2000, &0, &1000);
    assert_eq!(client.get_schedule_count(&admin), 2);
}

#[test]
fn zero_duration_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, beneficiary, token_id) = setup(&env);
    client.initialize_vesting(&admin);
    let r = client.try_create_schedule(&admin, &beneficiary, &token_id, &1000, &1000, &0, &0);
    assert!(r.is_err());
}

#[test]
fn cliff_longer_than_duration_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, beneficiary, token_id) = setup(&env);
    client.initialize_vesting(&admin);
    let r = client.try_create_schedule(&admin, &beneficiary, &token_id, &1000, &1000, &2000, &1000);
    assert!(r.is_err());
}

#[test]
fn partial_claim_happy_path_and_history() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, beneficiary, token_id) = setup(&env);
    client.initialize_vesting(&admin);

    let total = 1_000_000_i128;
    let start = 1_000_u64;
    let cliff = 200_u64;
    let duration = 1_000_u64;
    let idx =
        client.create_schedule(&admin, &beneficiary, &token_id, &total, &start, &cliff, &duration);

    // Move to halfway between cliff and end → vested = total * 0.5
    let cliff_time = start + cliff;
    let end_time = start + duration;
    let mid_time = cliff_time + (end_time - cliff_time) / 2;
    env.ledger().with_mut(|l| l.timestamp = mid_time);

    // Fund contract to enable payouts
    let contract_addr = client.address.clone();
    let claimable = client.get_claimable_vesting(&admin, &idx);
    assert!(claimable > 0);
    mint_tokens(&env, &token_id, &contract_addr, &claimable);

    // Beneficiary initial balance
    let bal_before = balance(&env, &token_id, &beneficiary);

    // Partial claim: half of claimable
    let partial = claimable / 2;
    let claimed = client.claim_vesting_partial(&beneficiary, &admin, &idx, &partial);
    assert_eq!(claimed, partial);

    // Check balances
    let bal_after = balance(&env, &token_id, &beneficiary);
    assert_eq!(bal_after - bal_before, partial);

    // Check schedule claimed updated
    let schedule = client.get_schedule(&admin, &idx);
    assert_eq!(schedule.claimed_amount, partial);

    // History count and record
    let cnt = client.get_partial_claim_count(&admin, &idx);
    assert_eq!(cnt, 1);
    let rec = client.get_partial_claim_record(&admin, &idx, &0).expect("record 0");
    assert_eq!(rec.1, partial);
    assert!(rec.0 >= mid_time);
}

#[test]
fn partial_claim_zero_amount_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, beneficiary, token_id) = setup(&env);
    client.initialize_vesting(&admin);
    client.create_schedule(&admin, &beneficiary, &token_id, &1_000_000, &1_000, &100, &1_000);
    env.ledger().with_mut(|l| l.timestamp = 2_000);
    let r = client.try_claim_vesting_partial(&beneficiary, &admin, &0, &0);
    assert!(r.is_err());
}

#[test]
fn partial_claim_exceeds_claimable_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, beneficiary, token_id) = setup(&env);
    client.initialize_vesting(&admin);
    client.create_schedule(&admin, &beneficiary, &token_id, &1_000_000, &1_000, &100, &1_000);
    env.ledger().with_mut(|l| l.timestamp = 2_000);
    let claimable = client.get_claimable_vesting(&admin, &0);
    // Fund exactly claimable
    let contract_addr = client.address.clone();
    mint_tokens(&env, &token_id, &contract_addr, &claimable);
    let r = client.try_claim_vesting_partial(&beneficiary, &admin, &0, &(claimable + 1));
    assert!(r.is_err());
}

#[test]
fn partial_claim_before_cliff_nothing_to_claim() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, beneficiary, token_id) = setup(&env);
    client.initialize_vesting(&admin);
    client.create_schedule(&admin, &beneficiary, &token_id, &1_000_000, &1_000, &300, &1_000);
    env.ledger().with_mut(|l| l.timestamp = 1_100); // before cliff (1_300)
    let r = client.try_claim_vesting_partial(&beneficiary, &admin, &0, &1);
    assert!(r.is_err());
}
