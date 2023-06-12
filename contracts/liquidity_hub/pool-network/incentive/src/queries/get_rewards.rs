use cosmwasm_std::{Decimal256, Deps, Uint128, Uint256};

use white_whale::pool_network::{asset::Asset, incentive::RewardsResponse};

use crate::error::ContractError;
use crate::helpers;
use crate::state::{EpochId, ADDRESS_WEIGHT_HISTORY, GLOBAL_WEIGHT_SNAPSHOT, LAST_CLAIMED_EPOCH};

#[allow(unused_assignments)]
/// Gets the rewards for the given address. Returns a [RewardsResponse] struct.
pub fn get_rewards(deps: Deps, address: String) -> Result<RewardsResponse, ContractError> {
    let address = deps.api.addr_validate(&address)?;
    let current_epoch = helpers::get_current_epoch(deps)?;
    let last_claimed_epoch = LAST_CLAIMED_EPOCH.may_load(deps.storage, &address)?;

    // Check if the user ever claimed before
    if let Some(last_claimed_epoch) = last_claimed_epoch {
        // if the last claimed epoch is the same as the current epoch, then there is nothing to claim
        if current_epoch == last_claimed_epoch {
            return Ok(RewardsResponse { rewards: vec![] });
        }
    }

    println!("current_epoch: {:?}", current_epoch);
    println!("last_claimed_epoch: {:?}", last_claimed_epoch);

    let flows = helpers::get_available_flows(deps, &current_epoch)?;

    let mut last_epoch_user_weight_update: EpochId = 0u64;
    let mut last_user_weight_seen: Uint128 = Uint128::zero();
    let mut rewards = vec![];
    println!("flows: {:?}", flows);
    for flow in flows.iter() {
        // check if flow already ended and if everything has been claimed for that flow.
        if current_epoch > flow.end_epoch && flow.claimed_amount == flow.flow_asset.amount {
            // if so, skip flow.
            continue;
        }

        // reset last_epoch_user_weight_update and last_user_weight_seen
        last_epoch_user_weight_update = 0u64;
        last_user_weight_seen = Uint128::zero();

        // check what is the earliest available weight for the user
        let earliest_available_weight_for_user =
            helpers::get_earliest_available_weight_snapshot_for_user(deps, &&address)?;

        if !earliest_available_weight_for_user.is_empty() {
            (last_epoch_user_weight_update, last_user_weight_seen) =
                earliest_available_weight_for_user[0];
        }

        println!(
            "earliest_available_weight_for_user: {:?}",
            earliest_available_weight_for_user
        );

        let first_claimable_epoch = if let Some(last_claimed_epoch) = last_claimed_epoch {
            // start claiming from the last claimed epoch + 1
            last_claimed_epoch + 1u64
        } else {
            // the user never claimed before
            if flow.start_epoch > last_epoch_user_weight_update {
                // it means the user locked tokens before the flow started. Start from there just to get
                // the ADDRESS_WEIGHT_HISTORY right
                last_epoch_user_weight_update
            } else {
                // it means the user locked tokens after the flow started, and last_epoch_user_weight_update has a value
                flow.start_epoch
            }
        };

        println!("first_claimable_epoch: {:?}", first_claimable_epoch);

        let mut flow_emitted_tokens = flow.emitted_tokens.clone();
        let mut total_reward = Uint128::zero();

        for epoch_id in first_claimable_epoch..=current_epoch {
            // check if the flow is active in this epoch
            if epoch_id < flow.start_epoch {
                // the flow is not active yet, skip
                continue;
            } else if epoch_id >= flow.end_epoch {
                // this flow has finished
                // todo maybe we should make end_epoch inclusive?
                break;
            }

            // calculate emissions per epoch
            let emitted_tokens = if flow_emitted_tokens.is_empty() {
                // if the emitted_tokens map is empty, it means that this is the first time we
                // are calculating an emission for this flow, return zero
                Uint128::zero()
            } else {
                // otherwise we want to return the last emission, since this is used in the formula
                // default to zero if the emission is not found, i.e. for cases when someone is claiming
                // the very first epoch for the flow after someone else and there's already an
                // emission stored in the map. So defaulting to zero emulates the case when the if
                // statement above is true.
                let previous_emission = *flow_emitted_tokens
                    .get(&(epoch_id - 1u64))
                    .unwrap_or(&Uint128::zero());

                previous_emission
            };

            println!("emitted_tokens: {:?}", emitted_tokens);

            // emission = (total_tokens - emitted_tokens_at_epoch) / (flow_start + flow_duration - epoch) = (total_tokens - emitted_tokens_at_epoch) / (flow_end - epoch)
            let emission_per_epoch = flow
                .flow_asset
                .amount
                .saturating_sub(emitted_tokens)
                .checked_div(Uint128::from(flow.end_epoch - epoch_id))?;

            println!("emission_per_epoch: {:?}", emission_per_epoch);

            // record the emitted tokens for this epoch if it hasn't been recorded before.
            // emitted tokens for this epoch is the total emitted tokens in previous epoch + the ones
            // that where emitted in this epoch
            if flow_emitted_tokens.get(&epoch_id).is_none() {
                flow_emitted_tokens
                    .insert(epoch_id, emission_per_epoch.checked_add(emitted_tokens)?);
            }

            // get user weight for this epoch
            let user_weight_at_epoch =
                ADDRESS_WEIGHT_HISTORY.may_load(deps.storage, (&address.clone(), epoch_id))?;

            // this is done this way because we don't save the weight for every single epoch for the user,
            // but rather keep a registry on when it changes. So we need to check if the user has a weight
            // registered for this epoch, and if not, use the last one that was recorded since it means
            // it hasn't changed since then.
            let user_weight = if let Some(user_weight_at_epoch) = user_weight_at_epoch {
                (last_epoch_user_weight_update, last_user_weight_seen) =
                    (epoch_id, user_weight_at_epoch);
                user_weight_at_epoch
            } else if last_epoch_user_weight_update != 0u64
                && last_epoch_user_weight_update <= epoch_id
            {
                // we have a weight for this user for a previous epoch, but not this one, so we use
                // the last we saw as that's the current one
                last_user_weight_seen
            } else {
                // we don't have a weight for this user for this epoch, or any previous epoch, so we
                // skip this epoch. i.e. the user's weight is 0 for this epoch
                continue;
            };

            // get global weight for this epoch
            let global_weight_at_epoch = GLOBAL_WEIGHT_SNAPSHOT
                .may_load(deps.storage, epoch_id)?
                .unwrap_or_default();

            if global_weight_at_epoch == Uint128::zero() {
                // Nothing to compute here as the global weight is 0, we don't want to divide by 0
                continue;
            }

            // calculate user share for this epoch
            let user_share_at_epoch = Decimal256::from_ratio(user_weight, global_weight_at_epoch);
            let user_reward_at_epoch: Uint128 =
                (Uint256::from_uint128(emission_per_epoch) * user_share_at_epoch).try_into()?;

            // sanity check for user_reward_at_epoch
            if user_reward_at_epoch > emission_per_epoch
                || user_reward_at_epoch.checked_add(flow.claimed_amount)? > flow.flow_asset.amount
            {
                return Err(ContractError::InvalidReward {});
            }

            total_reward += user_reward_at_epoch;
        }

        rewards.push(Asset {
            amount: total_reward,
            info: flow.flow_asset.info.clone(),
        });
    }

    rewards.retain(|asset| asset.amount > Uint128::zero());

    Ok(RewardsResponse { rewards })
}
