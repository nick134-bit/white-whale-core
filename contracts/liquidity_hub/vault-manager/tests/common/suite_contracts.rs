use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

/// Creates the whale lair contract
pub fn whale_lair_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        whale_lair::contract::execute,
        whale_lair::contract::instantiate,
        whale_lair::contract::query,
    )
    .with_migrate(whale_lair::contract::migrate);

    Box::new(contract)
}

/// Creates the fee collector contract
pub fn fee_collector_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        fee_collector::contract::execute,
        fee_collector::contract::instantiate,
        fee_collector::contract::query,
    )
    .with_reply(fee_collector::contract::reply)
    .with_migrate(fee_collector::contract::migrate);

    Box::new(contract)
}

/// Creates a cw20 token contract
pub fn cw20_token_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        terraswap_token::contract::execute,
        terraswap_token::contract::instantiate,
        terraswap_token::contract::query,
    );

    Box::new(contract)
}

/// Creates a vault manager contract
pub fn vault_manager_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        vault_manager::contract::execute,
        vault_manager::contract::instantiate,
        vault_manager::contract::query,
    )
    .with_migrate(vault_manager::contract::migrate);

    Box::new(contract)
}

/// Creates a pair contract
pub fn pair_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        terraswap_pair::contract::execute,
        terraswap_pair::contract::instantiate,
        terraswap_pair::contract::query,
    )
    .with_reply(terraswap_pair::contract::reply)
    .with_migrate(terraswap_pair::contract::migrate);

    Box::new(contract)
}
