use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _, Ledger as _},
    Address, Env, IntoVal,
};

use crate::vesting::{RevoraVesting, RevoraVestingClient, VESTING_EVENT_SCHEMA_VERSION};

fn setup(env: &Env) -> (RevoraVestingClient<'_>, Address, Address, Address) {
    let contract_id = env.register_contract(None, RevoraVesting);
    let client = RevoraVestingClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let beneficiary = Address::generate(env);
    let token_id = env.register_stellar_asset_contract(admin.clone());
    (client, admin, beneficiary, token_id)
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
fn event_schema_version_is_stable() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, _beneficiary, _token_id) = setup(&env);
    client.initialize_vesting(&admin);

    assert_eq!(client.get_event_schema_version(), VESTING_EVENT_SCHEMA_VERSION);
    assert_eq!(client.get_event_schema_version(), 1);
}

#[test]
fn create_schedule_emits_legacy_and_v1_events() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, beneficiary, token_id) = setup(&env);
    let contract_id = client.address.clone();
    client.initialize_vesting(&admin);

    let idx =
        client.create_schedule(&admin, &beneficiary, &token_id, &1_000_000, &1000, &250, &2000);
    assert_eq!(idx, 0);

    let events = env.events().all();
    let legacy = (
        contract_id.clone(),
        (symbol_short!("vest_crt"), admin.clone(), beneficiary.clone()).into_val(&env),
        (token_id.clone(), 1_000_000_i128, 1000_u64, 1250_u64, 3000_u64, 0_u32).into_val(&env),
    );
    let v1 = (
        contract_id,
        (symbol_short!("vst_crt1"), admin, beneficiary).into_val(&env),
        (
            VESTING_EVENT_SCHEMA_VERSION,
            token_id,
            1_000_000_i128,
            1000_u64,
            1250_u64,
            3000_u64,
            0_u32,
        )
            .into_val(&env),
    );

    assert!(events.contains(&legacy));
    assert!(events.contains(&v1));
}
