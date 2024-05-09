use crate::epoch_manager::epoch_manager::Epoch as EpochV2;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    to_json_binary, Addr, Coin, CosmosMsg, Decimal, StdResult, Timestamp, Uint128, WasmMsg,
};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

#[cw_serde]
pub struct Config {
    /// Pool Manager contract address for swapping
    pub pool_manager_addr: Addr,
    /// Epoch Manager contract address
    pub epoch_manager_addr: Addr,
    /// Distribution denom for the rewards
    pub distribution_denom: String,
    /// Unbonding period in nanoseconds. The time that needs to pass before an unbonded position can
    /// be withdrawn
    pub unbonding_period: u64,
    /// A fraction that controls the effect of time on the weight of a bond. If the growth rate is set
    /// to zero, time will have no impact on the weight.
    pub growth_rate: Decimal,
    /// Denom of the asset to be bonded. Can't only be set at instantiation.
    pub bonding_assets: Vec<String>,
    /// Grace period the maximum age of a epoch bucket before it's considered expired and fees
    /// are forwarded from it
    pub grace_period: u64,
}

#[cw_serde]
#[derive(Default)]
pub struct Epoch {
    // Epoch identifier
    pub id: u64,
    // Epoch start time
    pub start_time: Timestamp,
    // Initial fees to be distributed in this epoch.
    pub total: Vec<Coin>,
    // Fees left to be claimed on this epoch. These available fees are forwarded when the epoch expires.
    pub available: Vec<Coin>,
    // Fees that were claimed on this epoch. For keeping record on the total fees claimed.
    pub claimed: Vec<Coin>,
    // Global index taken at the time of Epoch Creation
    pub global_index: GlobalIndex,
}

#[cw_serde]
pub struct Bond {
    /// The amount of bonded tokens.
    pub asset: Coin,
    /// The epoch id at which the Bond was created.
    pub created_at_epoch: u64,
    /// The epoch id at which the bond was last time updated.
    pub updated_last: u64,
    /// The weight of the bond at the given block height.
    pub weight: Uint128,
}

impl Default for Bond {
    fn default() -> Self {
        Self {
            asset: Coin {
                denom: String::new(),
                amount: Uint128::zero(),
            },
            created_at_epoch: Default::default(),
            updated_last: Default::default(),
            weight: Uint128::zero(),
        }
    }
}

#[cw_serde]
#[derive(Default)]
pub struct GlobalIndex {
    /// The total amount of tokens bonded in the contract.
    pub bonded_amount: Uint128,
    /// Assets that are bonded in the contract.
    pub bonded_assets: Vec<Coin>,
    /// The epoch id at which the total bond was updated.
    pub last_updated: u64,
    /// The total weight of the bond at the given block height.
    pub weight: Uint128,
}

#[cw_serde]
pub struct InstantiateMsg {
    /// Denom to be swapped to and rewarded
    pub distribution_denom: String,
    /// Unbonding period in epochs. The time (in epochs) that needs to pass before an unbonded position can
    /// be withdrawn
    pub unbonding_period: u64,
    /// Weight grow rate. Needs to be between 0 and 1.
    pub growth_rate: Decimal,
    /// [String] denoms of the assets that can be bonded.
    pub bonding_assets: Vec<String>,
    /// Grace period the maximum age of a epoch bucket before it's considered expired and fees
    /// are forwarded from it
    pub grace_period: u64,
    /// The epoch manager contract
    pub epoch_manager_addr: String,
}

#[cw_serde]
pub struct EpochChangedHookMsg {
    pub current_epoch: EpochV2,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// Bonds the specified [Asset].
    Bond,
    /// Unbonds the specified [Asset].
    Unbond {
        /// The asset to unbond.
        asset: Coin,
    },
    /// Sends withdrawable unbonded tokens to the user.
    Withdraw {
        /// The denom to withdraw.
        denom: String,
    },
    /// Updates the [Config] of the contract.
    UpdateConfig {
        /// The new epoch manager address.
        epoch_manager_addr: Option<String>,
        /// The new pool manager address.
        pool_manager_addr: Option<String>,
        /// The unbonding period.
        unbonding_period: Option<u64>,
        /// The new growth rate.
        growth_rate: Option<Decimal>,
    },
    /// Claims the available rewards
    Claim,

    /// Fills the contract with new rewards.
    FillRewards,

    /// Creates a new bucket for the rewards flowing from this time on, i.e. to be distributed in
    /// the upcoming epoch. Also, forwards the expiring epoch (only 21 epochs are live at a given moment)
    EpochChangedHook {
        /// The current epoch, the one that was newly created.
        current_epoch: EpochV2,
    },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the [Config] of te contract.
    #[returns(Config)]
    Config,

    /// Returns the amount of assets that have been bonded by the specified address.
    #[returns(BondedResponse)]
    Bonded {
        /// The address to check for bonded assets. If none is provided, all bonded assets in the
        /// contract are returned.
        address: Option<String>,
    },

    /// Returns the amount of tokens of the given denom that are been unbonded by the specified address.
    /// Allows pagination with start_after and limit.
    #[returns(UnbondingResponse)]
    Unbonding {
        /// The address to check for unbonding assets.
        address: String,
        /// The denom to check for unbonding assets.
        denom: String,
        /// The amount of unbonding assets to skip. Allows pagination.
        start_after: Option<u64>,
        /// The maximum amount of unbonding assets to return.
        limit: Option<u8>,
    },

    /// Returns the amount of unbonding tokens of the given denom for the specified address that can
    /// be withdrawn, i.e. that have passed the unbonding period.
    #[returns(WithdrawableResponse)]
    Withdrawable {
        /// The address to check for withdrawable assets.
        address: String,
        /// The denom to check for withdrawable assets.
        denom: String,
    },

    /// Returns the weight of the address.
    #[returns(BondingWeightResponse)]
    Weight {
        /// The address to check for weight.
        address: String,
        /// The timestamp to check for weight. If none is provided, the current block time is used.
        epoch_id: Option<u64>,
        /// The global index to check for weight. If none is provided, the current global index is used.
        global_index: Option<GlobalIndex>,
    },

    /// Returns the global index of the contract.
    #[returns(GlobalIndex)]
    GlobalIndex,

    /// Returns the [Epoch]s that can be claimed by an address.
    #[returns(ClaimableEpochsResponse)]
    Claimable {
        /// The address to check for claimable epochs. If none is provided, all possible epochs
        /// stored in the contract that can potentially be claimed are returned.
        address: Option<String>,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

/// Response for the Bonded query
#[cw_serde]
pub struct BondedResponse {
    /// The total amount of bonded tokens by the address. Bear in mind the bonded assets are
    /// considered to be equal for this purpose.
    pub total_bonded: Uint128,
    /// The total amount of bonded assets by the address.
    pub bonded_assets: Vec<Coin>,
    /// If Some, the epoch id at which the user/address bonded first time. None is used when this
    /// Response is used to check the bonded assets in the contract.
    pub first_bonded_epoch_id: Option<u64>,
}

/// Response for the Unbonding query
#[cw_serde]
pub struct UnbondingResponse {
    /// The total amount of unbonded tokens by the address.
    pub total_amount: Uint128,
    /// The total amount of unbonded assets by the address.
    pub unbonding_requests: Vec<Bond>,
}

/// Response for the Withdrawable query
#[cw_serde]
pub struct WithdrawableResponse {
    /// The total amount of withdrawable assets by the address.
    pub withdrawable_amount: Uint128,
}

/// Response for the Weight query.
#[cw_serde]
pub struct BondingWeightResponse {
    /// The weight of the address.
    pub address: String,
    /// The weight of the address at the given timestamp.
    pub weight: Uint128,
    /// The global weight of the contract.
    pub global_weight: Uint128,
    /// The share the address has of the rewards at the particular timestamp.
    pub share: Decimal,
    /// The epoch id at which the weight was calculated.
    pub epoch_id: u64,
}

/// Creates a message to fill rewards on the whale lair contract.
pub fn fill_rewards_msg(contract_addr: String, assets: Vec<Coin>) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr,
        msg: to_json_binary(&ExecuteMsg::FillRewards)?,
        funds: assets,
    }))
}

#[cw_serde]
pub struct ClaimableEpochsResponse {
    /// The epochs that can be claimed by the address.
    pub epochs: Vec<Epoch>,
}
