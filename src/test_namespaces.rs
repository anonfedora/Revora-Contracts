#![cfg(test)]

use soroban_sdk::{
    symbol_short,
    testutils::Address as _,
    Address, Env,
};
use crate::{
    RevoraRevenueShare, RevoraRevenueShareClient,
};

fn make_client(env: &Env) -> RevoraRevenueShareClient<'_> {
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

    assert_eq!(client.get_snapshot_config(&issuer, &ns_1, &token), true);
    assert_eq!(client.get_snapshot_config(&issuer, &ns_2, &token), false);
}
