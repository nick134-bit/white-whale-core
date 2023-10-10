use std::string::ToString;

use cosmwasm_std::{Deps, Order, StdResult, Storage, Uint128};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, Map, UniqueIndex};

use white_whale::pool_network::asset::AssetInfo;
use white_whale::vault_manager::{Config, Vault};

use crate::ContractError;

// A bool representing if a flashloan is being performed or not
pub const ONGOING_FLASHLOAN: Item<bool> = Item::new("ongoing_flashloan");

// Stores the balances of all assets in the contract before a flashloan, to compare after the messages are executed
pub const TEMP_BALANCES: Map<&[u8], Uint128> = Map::new("temp_balances");

// Contract's config
pub const CONFIG: Item<Config> = Item::new("config");

// pub const VAULTS: Map<&[u8], Vault> = Map::new("vaults");
pub const VAULTS: IndexedMap<&[u8], Vault, VaultIndexes> = IndexedMap::new(
    "vaults",
    VaultIndexes {
        lp_asset: UniqueIndex::new(|v| v.lp_asset.to_string(), "vaults__lp_asset"),
    },
);

pub struct VaultIndexes<'a> {
    pub lp_asset: UniqueIndex<'a, String, Vault, &'a [u8]>,
}

impl<'a> IndexList<Vault> for VaultIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Vault>> + '_> {
        let v: Vec<&dyn Index<Vault>> = vec![&self.lp_asset];
        Box::new(v.into_iter())
    }
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub fn read_vaults(
    storage: &dyn Storage,
    start_after: Option<Vec<u8>>,
    limit: Option<u32>,
) -> StdResult<Vec<Vault>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after).map(Bound::ExclusiveRaw);

    VAULTS
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, vault) = item?;

            Ok(vault)
        })
        .collect()
}

/// Calculates the item at which to start the range
fn calc_range_start(start_after: Option<Vec<u8>>) -> Option<Vec<u8>> {
    start_after.map(|asset_info| {
        let mut v = asset_info;
        v.push(1);
        v
    })
}

/// Gets the vault given an lp asset as [AssetInfo]
pub fn get_vault(deps: &Deps, lp_asset: &AssetInfo) -> Result<Vault, ContractError> {
    Ok(VAULTS
        .idx
        .lp_asset
        .item(deps.storage, lp_asset.to_string())?
        .map_or_else(|| Err(ContractError::NonExistentVault {}), Ok)?
        .1)
}
