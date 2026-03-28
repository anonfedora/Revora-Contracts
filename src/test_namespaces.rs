#![cfg(test)]

use crate::{RevoraRevenueShare, RevoraRevenueShareClient};
use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env};

fn make_client(env: &Env) -> RevoraRevenueShareClient {
    let id = env.register_contract(None, RevoraRevenueShare);
    RevoraRevenueShareClient::new(env, &id)
}

#[test]
fn test_namespace_isolation() {
    let env = Env::default();
    env.mock_all_auths();

    let client = make_client(&env);

    let issuer_a = Address::generate(&env);
    let issuer_b = Address::generate(&env);
    let token = Address::generate(&env); // Same token for both!
    let ns_1 = symbol_short!("ns1");
    let ns_2 = symbol_short!("ns2");

    // Issuer A registers in ns1
    client.register_offering(&issuer_a, &ns_1, &token, &1000, &token, &0);
    // Issuer B registers in ns2 with SAME token
    client.register_offering(&issuer_b, &ns_2, &token, &2000, &token, &0);

    // Set holder shares differently
    let holder = Address::generate(&env);
    client.set_holder_share(&issuer_a, &ns_1, &token, &holder, &500);
    client.set_holder_share(&issuer_b, &ns_2, &token, &holder, &1500);

    // Verify they are isolated
    assert_eq!(client.get_holder_share(&issuer_a, &ns_1, &token, &holder), 500);
    assert_eq!(client.get_holder_share(&issuer_b, &ns_2, &token, &holder), 1500);

    // We need to manage the token (mint some to the issuer)
    // Actually, in mock_all_auths, the transfer will succeed if we don't check balances?
    // No, soroban-sdk mock_all_auths doesn't mock balances.
    // But we are using the `token` Address directly. We should probably use a proper token client.

    // For simplicity in this isolation test, let's just check metadata/config which are simple set/get
    client.set_claim_delay(&issuer_a, &ns_1, &token, &3600);
    client.set_claim_delay(&issuer_b, &ns_2, &token, &7200);

    assert_eq!(client.get_claim_delay(&issuer_a, &ns_1, &token), 3600);
    assert_eq!(client.get_claim_delay(&issuer_b, &ns_2, &token), 7200);
}

#[test]
fn test_same_issuer_different_namespaces() {
    let env = Env::default();
    env.mock_all_auths();

    let client = make_client(&env);

    let issuer = Address::generate(&env);
    let token = Address::generate(&env);
    let ns_1 = symbol_short!("prod");
    let ns_2 = symbol_short!("stg");

    client.register_offering(&issuer, &ns_1, &token, &1000, &token, &0);
    client.register_offering(&issuer, &ns_2, &token, &2000, &token, &0);

    client.set_snapshot_config(&issuer, &ns_1, &token, &true);
    client.set_snapshot_config(&issuer, &ns_2, &token, &false);

    assert!(client.get_snapshot_config(&issuer, &ns_1, &token));
    assert!(!client.get_snapshot_config(&issuer, &ns_2, &token));
}

#[test]
fn test_cross_namespace_blacklist_isolation() {
    let env = Env::default();
    env.mock_all_auths();
    let client = make_client(&env);

    let issuer = Address::generate(&env);
    let token = Address::generate(&env);
    let ns_1 = symbol_short!("ns1");
    let ns_2 = symbol_short!("ns2");
    let investor = Address::generate(&env);

    client.register_offering(&issuer, &ns_1, &token, &1000, &token, &0);
    client.register_offering(&issuer, &ns_2, &token, &1000, &token, &0);

    // Blacklist in NS 1
    client.blacklist_add(&issuer, &issuer, &ns_1, &token, &investor);
    
    // Verify isolated
    assert!(client.is_blacklisted(&issuer, &ns_1, &token, &investor));
    assert!(!client.is_blacklisted(&issuer, &ns_2, &token, &investor));
    
    assert_eq!(client.get_blacklist(&issuer, &ns_1, &token).len(), 1);
    assert_eq!(client.get_blacklist(&issuer, &ns_2, &token).len(), 0);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")] // OfferingNotFound
fn test_unregistered_namespace_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let client = make_client(&env);

    let issuer = Address::generate(&env);
    let token = Address::generate(&env);
    let ns_ghost = symbol_short!("ghost");

    // Attempt to set delay on non-existent offering
    client.set_claim_delay(&issuer, &ns_ghost, &token, &3600);
}

#[test]
fn test_unauthorized_issuer_access_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let client = make_client(&env);

    let issuer_real = Address::generate(&env);
    let issuer_attacker = Address::generate(&env);
    let token = Address::generate(&env);
    let ns_1 = symbol_short!("ns1");

    client.register_offering(&issuer_real, &ns_1, &token, &1000, &token, &0);

    // Attacker tries to blacklist for real issuer's offering
    // Note: mock_all_auths will allow the call to reach the contract, 
    // but the contract should check that issuer_attacker is not current_issuer.
    
    let res = client.try_blacklist_add(&issuer_attacker, &issuer_real, &ns_1, &token, &Address::generate(&env));
    
    // Should fail with NotAuthorized (#10) or OfferingNotFound (if we strictly check issuer in ID)
    // Actually our implementation returns NotAuthorized if issuer matches but caller doesn't, 
    // but here the issuer_real in the ID matches the real one, but the caller is attacker.
    assert!(res.is_err());
}

#[test]
fn test_transfer_maintains_namespace_isolation() {
    let env = Env::default();
    env.mock_all_auths();
    let client = make_client(&env);

    let issuer_a = Address::generate(&env);
    let issuer_b = Address::generate(&env);
    let token_1 = Address::generate(&env);
    let ns_1 = symbol_short!("ns1");

    client.register_offering(&issuer_a, &ns_1, &token_1, &1000, &token_1, &0);
    client.set_claim_delay(&issuer_a, &ns_1, &token_1, &3600);

    // Transfer to Issuer B
    client.propose_issuer_transfer(&issuer_a, &ns_1, &token_1, &issuer_b);
    client.accept_issuer_transfer(&issuer_a, &ns_1, &token_1);

    // Verify config preserved
    assert_eq!(client.get_claim_delay(&issuer_a, &ns_1, &token_1), 3600);

    // Verify Issuer B now has control (e.g. can change delay)
    client.set_claim_delay(&issuer_b, &ns_1, &token_1, &7200);
    assert_eq!(client.get_claim_delay(&issuer_a, &ns_1, &token_1), 7200);

    // Verify Issuer A NO LONGER has control
    let res = client.try_set_claim_delay(&issuer_a, &ns_1, &token_1, &9999);
    assert!(res.is_err());
}
