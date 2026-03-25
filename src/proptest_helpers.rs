use soroban_sdk::{Address, Env, Symbol, Vec};
use crate::RevoraRevenueShareClient;
use proptest::prelude::*;

// Common proptest strategies for Revora contract testing
pub fn any_offering_id(env: &Env) -> impl Strategy<Value = (Address, Symbol, Address)> {
    (
        any::<Address>(),
        ".*".prop_map(|s: String| Symbol::short(&env, &s.chars().take(4).collect::<String>()[..])),
        any::<Address>(),
    )
}

pub fn any_positive_amount() -> impl Strategy<Value = i128> {
    1i128..=100_000_000
}

pub fn strictly_increasing_periods(len: usize) -> impl Strategy<Value = Vec<u64>> {
    vec![0u64.prop_map(|_| 1u64); len-1].prop_map(|mut v| {
        let mut last = 1u64;
        for i in 0..v.len() {
            last += 1 + (i as u64);
            v[i] = last;
        }
        v
    })
}

// Operations for random sequence generation
#[derive(Debug, Clone)]
pub enum TestOperation {
    RegisterOffering((Address, Symbol, Address, u32, Address)),
    ReportRevenue((Address, Symbol, Address, Address, i128, u64, bool)),
    DepositRevenue((Address, Symbol, Address, Address, i128, u64)),
    SetHolderShare((Address, Symbol, Address, Address, u32)),
    BlacklistAdd((Address, Symbol, Address, Address)),
}

pub fn any_test_operation(env: &Env) -> impl Strategy<Value = TestOperation> {
    prop_oneof![
        any_offering_id(env).prop_map(|(i, ns, t, bps, pa)| TestOperation::RegisterOffering((i, ns, t, bps, pa))),
        10usize.prop_flat_map(|_| (any_offering_id(env), any_positive_amount(), any::<u64>(), any::<bool>()).prop_map(|((i,ns,t),amt,pid,ovr)| TestOperation::ReportRevenue((i,ns,t,pa,amt,pid,ovr)))),
        // Add more...
    ]
}

