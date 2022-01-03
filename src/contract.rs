use crate::conversion_utils;
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, OffChainTimestamps, OffChainValidators, QueryMsg, ValidatorAprResponse};
use crate::state::{Config, State, ValidatorMetrics, CONFIG, METRICS_HISTORY, STATE, OFF_CHAIN_TIMESTAMPS, OffChainValidatorMetrics, OFF_CHAIN_VALIDATOR_IDX_MAPPING, OFF_CHAIN_STATE_FOR_VALIDATOR, OffchainTimestampMetaData, OFF_CHAIN_TIMESTAMP_META_DATA, OffChainState, OFF_CHAIN_STATE};
use crate::state::{ValidatorAccounts};
use crate::util::{
    compute_apr, decimal_division_in_256, decimal_multiplication_in_256, decimal_summation_in_256,
    uint128_to_decimal,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Order, Response, StakingMsg,
    StdError, StdResult, Storage, Uint128,
};
use cosmwasm_std::{BankMsg, Decimal};
use cw_storage_plus::{Bound, U16Key, U64Key};
use std::cmp;
use std::cmp::min;
use std::collections::HashMap;
use std::ops::Sub;
use terra_cosmwasm::{TerraMsgWrapper, TerraQuerier};

//todo[PD]: consider splitting into validator + test, metrics + test modules, for modularity.

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        vault_denom: msg.vault_denom.clone(),
        validators: vec![],
        cron_timestamps: vec![],
        validator_index_for_next_cron: 0,
    };
    let config = Config {
        manager: info.sender.clone(),
        amount_to_stake_per_validator: msg.amount_to_stake_per_validator,
        batch_size: msg.batch_size,
    };

    STATE.save(deps.storage, &state)?;
    CONFIG.save(deps.storage, &config)?;

    // offchain publishing related states
    // let off_chain_state = OffChainState {
    //     next_validator_idx: 0,
    // };

    // OFF_CHAIN_STATE.save(deps.storage, &off_chain_state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("manager", info.sender)
        .add_attribute("time", _env.block.time.seconds().to_string())
        .add_attribute(
            "amount_to_stake_per_validator",
            msg.amount_to_stake_per_validator,
        )
        .add_attribute("vault_denom", msg.vault_denom))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(
    _deps: DepsMut,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    CONFIG.update(_deps.storage, |mut conf| -> StdResult<_> {
        conf.manager = _msg.manager_address.clone();
        Ok(conf)
    })?;

    // offchain publishing related states
    // let off_chain_state = OffChainState {
    //     next_validator_idx: 0,
    // };

    // OFF_CHAIN_STATE.save(_deps.storage, &off_chain_state)?;

    Ok(Response::new()
        .add_attribute("method", "update_config")
        .add_attribute("new_manager", _msg.manager_address.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetState {} => to_binary(&query_state(deps)?),
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
        QueryMsg::GetAllTimestamps {} => to_binary(&query_timestamps(deps)?),
        QueryMsg::GetAllAprsByInterval {
            timestamp1,
            timestamp2,
            from,
            to,
        } => to_binary(&query_validators_aprs_by_interval(
            deps, timestamp1, timestamp2, from, to,
        )?),
        QueryMsg::GetAprByValidator {
            timestamp1,
            timestamp2,
            addr,
        } => to_binary(&query_validator_apr(deps, timestamp1, timestamp2, addr)?),
        QueryMsg::GetAllValidatorMetrics { addr } => {
            to_binary(&query_all_validator_metrics(deps, addr)?)
        }
        QueryMsg::GetValidatorMetricsByTimestamp { addr, timestamp } => to_binary(
            &query_validator_metrics_by_timestamp(deps, addr, timestamp)?,
        ),
        QueryMsg::GetValidatorsMetricsByTimestamp {
            timestamp,
            from,
            to,
        } => to_binary(&query_validators_metrics_by_timestamp(
            deps, timestamp, from, to,
        )?),
        QueryMsg::GetValidatorMetricsBtwTimestamps {
            addr,
            timestamp1,
            timestamp2,
        } => to_binary(&query_all_validator_metrics_btw_timestamps(
            deps, addr, timestamp1, timestamp2,
        )?),

        QueryMsg::GetOffChainValidatorMetrics {
            timestamp,
            validator_addr,
        } => to_binary(&get_off_chain_metrics(deps, timestamp, validator_addr)?),

        QueryMsg::GetOffChainMetricsTimestamps {} => {
            to_binary(&get_off_chain_metrics_timestamps(deps)?)
        }
        QueryMsg::GetOffChainState {} => to_binary(&get_off_chain_state(deps)?),
        QueryMsg::GetOffChainTimestampMetaData { timestamp } => {
            to_binary(&get_off_chain_timestamp_meta_data(deps, timestamp)?)
        }
        QueryMsg::GetOffChainValidators {} => to_binary(&get_off_chain_validators(deps)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::RecordMetrics { timestamp } => {
            record_validator_metrics(deps, env, info, timestamp)
        }
        ExecuteMsg::AddValidator {
            validator_opr_addr,
            account_addr,
        } => add_validator(deps, info, validator_opr_addr, account_addr),
        ExecuteMsg::UpdateConfig { batch_size } => update_config(deps, info, batch_size),
        ExecuteMsg::RemoveValidator {
            validator_oper_addr,
        } => remove_validator(deps, info, validator_oper_addr),
        ExecuteMsg::WithdrawFunds { amount } => withdraw_funds(deps, info, amount),

        ExecuteMsg::DeleteMetricsForTimestamp {
            timestamp,
            validator_idx,
            validator_ct,
        } => delete_metrics_for_timestamp(
            deps,
            info,
            timestamp,
            validator_idx as usize,
            validator_ct as usize,
        ),

        ExecuteMsg::DeleteMetricsForValidator {
            validator_opr_addr,
            timestamp_idx,
            timestamp_ct,
        } => delete_metrics_for_validator(
            deps,
            info,
            validator_opr_addr,
            timestamp_idx as usize,
            timestamp_ct as usize,
        ),

        ExecuteMsg::RemoveTimestamp { timestamp } => remove_timestamp(deps, info, timestamp),
        ExecuteMsg::RemoveOffChainMetricsForTimestamp {
            timestamp,
            no_of_validators_to_remove,
        } => remove_off_chain_metrics_for_timestamp(
            deps,
            info,
            timestamp,
            no_of_validators_to_remove,
        ),

        ExecuteMsg::OffChainAddValidator { oper_addr } => {
            add_off_chain_validator(deps, info, oper_addr)
        }

        ExecuteMsg::OffChainRecordTimestampMetaData {
            timestamp,
            timestamp_meta_data,
        } => save_off_chain_details(deps, info, timestamp, timestamp_meta_data),

        ExecuteMsg::OffChainAddValidatorMetricsForTimestamp {
            timestamp,
            validator_metrics,
        } => add_off_chain_validator_metrics(deps, info, timestamp, validator_metrics),
    }
}
//

//todo; write test for this
//note: this is also inefficient, potentially O(T), due to array deletion. not sure if
// cosmwasm library optimizes this.
// old code
fn remove_timestamp(
    deps: DepsMut,
    info: MessageInfo,
    timestamp: u64,
) -> Result<Response, ContractError> {
    // can only be called by manager
    if !sender_is_manager(&deps, &info) {
        return Err(ContractError::Unauthorized {});
    }

    let mut state = STATE.load(deps.storage)?;
    let existing_timestamps_length = state.cron_timestamps.len();

    let new_timestamps = state
        .cron_timestamps
        .into_iter()
        .filter(|state_tstamp| state_tstamp.ne(&timestamp))
        .collect::<Vec<u64>>();

    let mut timestamp_existed = false;

    if new_timestamps.len() < existing_timestamps_length {
        state.cron_timestamps = new_timestamps;
        STATE.save(deps.storage, &state)?;
        timestamp_existed = true;
    }

    Ok(Response::new()
        .add_attribute("method", "delete_timestamp")
        .add_attribute("timestamp_removed", timestamp.to_string())
        .add_attribute(
            "message",
            if timestamp_existed {
                "timestamp successfully removed"
            } else {
                "timestamp didn't exist"
            },
        ))
}

fn sender_is_manager(deps: &DepsMut, info: &MessageInfo) -> bool {
    let config = CONFIG.load(deps.storage).unwrap();
    info.sender == config.manager
}

fn delete_metrics_for_timestamp(
    deps: DepsMut,
    info: MessageInfo,
    timestamp: u64,
    validator_start: usize,
    validator_ct: usize,
) -> Result<Response, ContractError> {
    if !sender_is_manager(&deps, &info) {
        return Err(ContractError::Unauthorized {});
    }
    //for every validator, in range, remove the metrics
    let state = STATE.load(deps.storage)?;
    if state.validators.len().le(&validator_start) {
        return Err(ContractError::ValidatorOutOfRange {});
    }

    let validator_end = min(validator_start + validator_ct, state.validators.len());

    state.validators[validator_start..validator_end]
        .iter()
        .for_each(|validator| {
            METRICS_HISTORY.remove(
                deps.storage,
                (&validator.operator_address, U64Key::from(timestamp)),
            );
        });

    Ok(Response::new()
        .add_attribute("method", "delete_metrics_for_timestamp")
        .add_attribute("deleted_timestamp", timestamp.to_string())
        .add_attribute(
            "validators_left",
            (state.validators.len() - validator_end).to_string(),
        )
        .add_attribute("next_validator_idx", validator_end.to_string()))
}

fn query_timestamps(deps: Deps) -> StdResult<Vec<u64>> {
    Ok(STATE.load(deps.storage)?.cron_timestamps)
}

fn query_validator_apr(
    deps: Deps,
    timestamp1: u64,
    timestamp2: u64,
    addr: Addr,
) -> StdResult<ValidatorAprResponse> {
    if timestamp1.ge(&timestamp2) {
        return Err(StdError::GenericErr {
            msg: "timestamp1 cannot be greater than or equal to timestamp2".to_string(),
        });
    }

    let h1 = METRICS_HISTORY.load(deps.storage, (&addr, U64Key::new(timestamp1)))?;

    let h2 = METRICS_HISTORY.load(deps.storage, (&addr, U64Key::new(timestamp2)))?;

    Ok(ValidatorAprResponse {
        addr,
        apr: compute_apr(&h1, &h2, timestamp2 - timestamp1)?,
    })
}

fn query_validators_aprs_by_interval(
    deps: Deps,
    timestamp1: u64,
    timestamp2: u64,
    from: u64,
    to: u64,
) -> StdResult<Vec<ValidatorAprResponse>> {
    if timestamp1.ge(&timestamp2) {
        return Err(StdError::GenericErr {
            msg: "timestamp1 cannot be greater than or equal to timestamp2".to_string(),
        });
    }
    let validators = STATE.load(deps.storage)?.validators;

    let total_validators: u64 = validators.len() as u64;

    if to.ge(&total_validators) || from > to {
        return Err(StdError::GenericErr {
            msg: "Invalid indexes!".to_string(),
        });
    }

    let t1 = U64Key::new(timestamp1);
    let t2 = U64Key::new(timestamp2);

    let mut response: Vec<ValidatorAprResponse> = vec![];
    let mut start = from;

    while start.le(&to) {
        let validator_addr = &validators[start as usize];
        let h1_opt =
            METRICS_HISTORY.may_load(deps.storage, (&validator_addr.operator_address, t1.clone()));
        let h2_opt =
            METRICS_HISTORY.may_load(deps.storage, (&validator_addr.operator_address, t2.clone()));
        if let (Ok(Some(h1)), Ok(Some(h2))) = (h1_opt, h2_opt) {
            let apr = compute_apr(&h1, &h2, timestamp2 - timestamp1)?;
            response.push(ValidatorAprResponse {
                addr: h2.operator_addr,
                apr,
            });
        };
        start += 1;
    }

    Ok(response)
}

fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    batch_size: u64,
) -> Result<Response, ContractError> {
    if batch_size == 0 {
        return Err(ContractError::BatchSizeCannotBeZero {});
    }

    if !sender_is_manager(&deps, &info) {
        return Err(ContractError::Unauthorized {});
    }

    CONFIG.update(deps.storage, |mut conf| -> StdResult<_> {
        conf.batch_size = batch_size;
        Ok(conf)
    })?;

    Ok(Response::new()
        .add_attribute("method", "update_config")
        .add_attribute("new_batch_size", batch_size.to_string()))
}

fn add_validator(
    deps: DepsMut,
    info: MessageInfo,
    validator_addr: Addr,
    wallet_addrress: String, // account address of the validator
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;

    let vault_denom = state.vault_denom;
    let amount_to_stake_per_validator = config.amount_to_stake_per_validator;

    // can only be called by manager
    if info.sender != config.manager {
        return Err(ContractError::Unauthorized {});
    }

    // check if the validator exists in the blockchain
    if deps
        .querier
        .query_validator(validator_addr.clone())?
        .is_none()
    {
        return Err(ContractError::ValidatorDoesNotExist {});
    }

    // Validator should not be already recorded
    if state
        .validators
        .iter()
        .any(|addr| addr.operator_address.eq(&validator_addr))
    {
        return Err(ContractError::ValidatorAlreadyExists {});
    }

    let funds = info.funds.first();
    if funds.is_none() {
        return Err(ContractError::NoFundsFound {});
    }

    if funds.unwrap().amount.lt(&amount_to_stake_per_validator) {
        return Err(ContractError::InsufficientFunds {});
    }

    let msg = StakingMsg::Delegate {
        validator: validator_addr.to_string(),
        amount: Coin {
            denom: vault_denom,
            amount: amount_to_stake_per_validator,
        },
    };

    // since deps is borrowed as mutable below, borrowing it immutably here
    let validator_account_addr = deps.api.addr_validate(&wallet_addrress).unwrap();

    STATE.update(deps.storage, |mut s: State| -> StdResult<_> {
        let current_validator = ValidatorAccounts {
            operator_address: validator_addr.clone(),
            account_address: validator_account_addr,
        };
        s.validators.push(current_validator);
        Ok(s)
    })?;

    Ok(Response::new()
        .add_messages(vec![msg])
        .add_attribute("method", "add_validator"))
}

fn remove_validator(
    deps: DepsMut,
    info: MessageInfo,
    val_opr_addr: Addr,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;

    let vault_denom = state.vault_denom.clone();
    let amount_to_stake_per_validator = config.amount_to_stake_per_validator;

    // can only be called by manager
    if info.sender != config.manager {
        return Err(ContractError::Unauthorized {});
    }

    let val_len = state.validators.len();

    /* note: this seems unoptimized, primarily due to equality op being required.
       with a change to state can be optimized, i.e. if validators could be queried by validator address
    */
    let other_validators = state
        .validators
        .into_iter()
        .filter(|addr| !addr.operator_address.eq(&val_opr_addr))
        .collect::<Vec<ValidatorAccounts>>();
    if other_validators.len().eq(&val_len) {
        return Err(ContractError::ValidatorDoesNotExist {});
    }

    let msg = StakingMsg::Undelegate {
        validator: val_opr_addr.to_string(),
        amount: Coin {
            denom: vault_denom,
            amount: amount_to_stake_per_validator,
        },
    };

    state.validators = other_validators;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_messages(vec![msg])
        .add_attribute("method", "remove_validator"))
}

fn delete_metrics_for_validator(
    deps: DepsMut,
    info: MessageInfo,
    val_address: Addr,
    timestamp_start: usize,
    timestamp_count: usize,
) -> Result<Response, ContractError> {
    if !sender_is_manager(&deps, &info) {
        return Err(ContractError::Unauthorized {});
    }

    let state = STATE.load(deps.storage)?;
    if state.cron_timestamps.len().le(&timestamp_start) {
        return Err(ContractError::TimestampOutOfRange {});
    }

    let timestamp_end = min(
        timestamp_start + timestamp_count,
        state.cron_timestamps.len(),
    );

    query_timestamps(deps.as_ref()).unwrap()[timestamp_start..timestamp_end]
        .iter()
        .for_each(|timestamp| {
            METRICS_HISTORY.remove(deps.storage, (&val_address, U64Key::from(*timestamp)));
        });

    Ok(Response::new()
        .add_attribute("method", "remove_metrics_for_validator")
        .add_attribute(
            "msg",
            "All metrics are removed for the given validator and timestamp range",
        )
        .add_attribute(
            "timestamps_left",
            (state.cron_timestamps.len() - timestamp_end).to_string(),
        ))
}

// Anyone can call this but funds will only be sent back to manager.
fn withdraw_funds(
    deps: DepsMut,
    _info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let state = STATE.load(deps.storage)?;

    if amount.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }
    Ok(Response::new().add_message(BankMsg::Send {
        to_address: config.manager.to_string(),
        amount: vec![Coin::new(amount.u128(), state.vault_denom)],
    }))
}

pub fn record_validator_metrics(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    timestamp: u64,
) -> Result<Response, ContractError> {
    // can only be called by manager
    if !sender_is_manager(&deps, &info) {
        return Err(ContractError::Unauthorized {});
    }

    if get_last_recorded_timestamp(&deps) > timestamp {
        return Err(ContractError::TimestampWithinExistingRange {});
    }

    let validators_to_record = get_validators_to_record(deps.storage, timestamp)?;

    if validators_to_record.is_empty() {
        return Ok(Response::new()
            .add_attribute("method", "record_validator_metrics")
            .add_attribute("msg", "All validators are recorded for the given cron time")
            .add_attribute("validators_left", "0"));
    }

    let current_validators_metrics =
        compute_current_metrics(&deps, env, &validators_to_record, timestamp)?;

    let t = U64Key::new(timestamp);
    for metric in current_validators_metrics {
        METRICS_HISTORY.save(deps.storage, (&metric.operator_addr, t.clone()), &metric)?;
    }

    Ok(Response::new()
        .add_attribute("method", "record_validator_metrics")
        .add_attribute(
            "msg",
            format!(
                "Updated {} validators for the given time",
                validators_to_record.len()
            ),
        ))
}

fn get_last_recorded_timestamp(deps: &DepsMut) -> u64 {
    let state = STATE.load(deps.storage).unwrap();
    *state.cron_timestamps.last().unwrap_or(&(0_u64))
}

fn compute_current_metrics(
    deps: &DepsMut,
    env: Env,
    validators: &[ValidatorAccounts],
    timestamp: u64,
) -> Result<Vec<ValidatorMetrics>, ContractError> {
    let state = STATE.load(deps.storage)?;
    let vault_denom = state.vault_denom;
    let last_cron_time_opt = state.cron_timestamps.last();

    let mut exchange_rates_map: HashMap<String, Decimal> = HashMap::new();
    exchange_rates_map.insert(vault_denom.clone(), Decimal::one());
    let querier = TerraQuerier::new(&deps.querier);

    let mut current_metrics: Vec<ValidatorMetrics> = vec![];

    for validator_addr in validators {
        // could be extracted into another function? computes metric for validator
        let delegation_opt = deps.querier.query_delegation(
            &env.contract.address,
            validator_addr.operator_address.clone(),
        )?;

        // when would this happen?
        if delegation_opt.is_none() {
            return Err(ContractError::NoDelegationFound {
                manager: env.contract.address,
                validator: validator_addr.operator_address.clone(),
            });
        }

        let validator_opt = deps
            .querier
            .query_validator(validator_addr.operator_address.clone())?;
        // if suddenly validators drop out of the validator set, either due to jailing or some other mishap.
        if validator_opt.is_none() {
            continue;
        } // Handle this edge case when validator previous timestamp does not match state.cron_timings.last() entry.

        let validator = validator_opt.unwrap();
        let delegation = delegation_opt.unwrap();

        let (rewards_diff, previous_rewards) = get_diff_in_rewards_from_last_cron(
            deps,
            &&(validator_addr.operator_address),
            last_cron_time_opt,
            delegation.accumulated_rewards.clone(),
        )?;

        let current_rewards_diff = get_total_rewards_in_vault_denom(
            &rewards_diff,
            &vault_denom,
            &mut exchange_rates_map,
            &querier,
        );

        // This is the new Delegated amount after slashing Ex: (10 => 9.8 etc.,)
        let current_delegated_amount = delegation.amount.amount;

        let self_delegation_opt = deps.querier.query_delegation(
            validator_addr.account_address.clone(), //This is the Account Address
            validator_addr.operator_address.clone(), //This is the Operator Address
        );

        // This is the self_delegation amount (delegation by validator)
        let self_delegation_amount = match self_delegation_opt {
            Ok(delegation_result) => delegation_result.unwrap().amount.amount,
            Err(_) => Uint128::new(0),
        };

        // this stores current_slashing pointer value
        // if there are no prev metric then it's default value is 1.0
        let mut current_slashing_pointer = Decimal::one();

        // this is a vector of all the metrics of current validator
        let vector_delegation_change_ratio =
            query_all_validator_metrics(deps.as_ref(), validator_addr.operator_address.clone())
                .unwrap();

        // current_slashing_pointer=(current_delegated_amount/prev_delegated_amount)*prev_slashing_pointer
        if !vector_delegation_change_ratio.is_empty() {
            let delegation_change_ratio = &vector_delegation_change_ratio.last().unwrap().1;
            current_slashing_pointer = decimal_division_in_256(
                uint128_to_decimal(current_delegated_amount),
                uint128_to_decimal(delegation_change_ratio.delegated_amount),
            );
            current_slashing_pointer = decimal_multiplication_in_256(
                current_slashing_pointer,
                delegation_change_ratio.slashing_pointer,
            );
        }

        current_metrics.push(ValidatorMetrics {
            operator_addr: validator_addr.operator_address.clone(),
            rewards: decimal_summation_in_256(current_rewards_diff, previous_rewards),
            delegated_amount: current_delegated_amount,
            self_delegated_amount: self_delegation_amount,
            slashing_pointer: current_slashing_pointer,
            commission: validator.commission,
            max_commission: validator.max_commission,
            rewards_in_coins: delegation.accumulated_rewards.clone(),
            timestamp,
        });
    }
    Ok(current_metrics)
}

fn get_diff_in_rewards_from_last_cron(
    deps: &DepsMut,
    validator_addr: &&Addr,
    last_cron_time_opt: Option<&u64>,
    current_accumulated_rewards: Vec<Coin>,
    // Return is Tuple of (Vec<Coin> = Diff in rewards,  Decimal = Previous cron rewards)
) -> Result<(Vec<Coin>, Decimal), ContractError> {
    // If this is the very first cron we simply return current_accumulated_rewards as the diff
    if last_cron_time_opt.is_none() {
        return Ok((current_accumulated_rewards, Decimal::zero()));
    }

    let last_cron_time = last_cron_time_opt.unwrap();
    let previous_metrics_opt =
        METRICS_HISTORY.may_load(deps.storage, (validator_addr, U64Key::new(*last_cron_time)))?;

    // If validator is added after the previous cron run, then there wont be any prev history for this validator
    if previous_metrics_opt.is_none() {
        return Ok((current_accumulated_rewards, Decimal::zero()));
    }

    let previous_metrics = previous_metrics_opt.unwrap();

    let mut previous_rewards_map: HashMap<String, Uint128> = HashMap::new();
    for reward in previous_metrics.rewards_in_coins {
        previous_rewards_map.insert(reward.denom, reward.amount);
    }

    let diff_in_rewards = current_accumulated_rewards
        .into_iter()
        .map(|reward| {
            // Find matching denom reward in the previous run
            let prev_reward_opt = previous_rewards_map.get(&reward.denom);
            match prev_reward_opt {
                Some(prev_reward) => Coin {
                    denom: reward.denom,
                    amount: reward.amount.sub(prev_reward), // Subtract the previous denom reward amount with the current
                },
                None => reward, // Last rewards vec does not contain this particular denom reward
            }
        })
        .collect();
    Ok((diff_in_rewards, previous_metrics.rewards))
}

fn get_total_rewards_in_vault_denom(
    rewards: &[Coin],
    vault_denom: &str,
    exchange_rates_map: &mut HashMap<String, Decimal>,
    querier: &TerraQuerier,
) -> Decimal {
    let mut current_rewards: Decimal = Decimal::zero();
    for coin in rewards {
        // Tries to find the exchange rate in the hashmap,
        // If not present we fetch the exchange rate and add it to the map before calculating reward
        let reward_for_coin =
            get_amount_in_vault_denom(coin, vault_denom, exchange_rates_map, querier);
        if let Some(converted_from_vault) = reward_for_coin {
            current_rewards = decimal_summation_in_256(converted_from_vault, current_rewards);
        } // If exchange rate is not fetchable then we skip such reward ?
    }
    current_rewards
}

fn get_validators_to_record(
    storage: &mut dyn Storage,
    timestamp: u64,
) -> Result<Vec<ValidatorAccounts>, ContractError> {
    // validators are fetched batch wise on index after the last index processed in last cron
    let batch_size = CONFIG.load(storage)?.batch_size;
    let state = STATE.load(storage)?;
    let last_cron_time_opt = state.cron_timestamps.last();
    let validators = state.validators;
    let total_validators: u64 = validators.len() as u64;
    let mut validator_index_for_next_cron = state.validator_index_for_next_cron;

    // If the Cron time is completely New (Update State)
    if last_cron_time_opt.is_none() || !last_cron_time_opt.unwrap().eq(&timestamp) {
        STATE.update(storage, |mut s| -> StdResult<_> {
            s.cron_timestamps.push(timestamp);
            s.validator_index_for_next_cron = 0;
            Ok(s)
        })?;

        validator_index_for_next_cron = 0
    }

    if validator_index_for_next_cron.ge(&total_validators) {
        return Ok(vec![]);
    }

    // Examples
    // len = 12, batch_size = 5
    // (=0 <5) 5 (=5 <10) 10 (=10 <12) Final(=12 is >=12 (total_validators)) => return Ok(vec![]);
    // len = 2, batch_size = 1
    // (=0 <1) 1 (=1 <2) Final(=2 is >= 2 (total_validators)) => return Ok(vec![]);
    let start = validator_index_for_next_cron;
    let end = cmp::min(start + batch_size, total_validators);

    let validators_batch: Vec<ValidatorAccounts> =
        validators[(start as usize)..(end as usize)].to_vec();
    // let validators_acc_batch = validators_acc[(start as usize)..(end as usize)].to_vec();

    STATE.update(storage, |mut s| -> StdResult<_> {
        s.validator_index_for_next_cron = end;
        Ok(s)
    })?;

    Ok(validators_batch)
}

fn get_amount_in_vault_denom(
    coin: &Coin,
    vault_denom: &str,
    exchange_rates_map: &mut HashMap<String, Decimal>, // Try to bring it outside (As we are mutating a func param)
    querier: &TerraQuerier,
) -> Option<Decimal> {
    if exchange_rates_map.contains_key(&coin.denom) {
        let exchange_rate = exchange_rates_map.get(&coin.denom).unwrap();
        Some(convert_amount_to_valut_denom(coin, *exchange_rate))
    } else {
        let rate_opt = query_exchange_rate(querier, vault_denom, &coin.denom);
        rate_opt?;
        let exchange_rate = rate_opt.unwrap();
        exchange_rates_map.insert(coin.denom.clone(), exchange_rate);
        Some(convert_amount_to_valut_denom(coin, exchange_rate))
    }
}

fn convert_amount_to_valut_denom(coin: &Coin, exchange_rate: Decimal) -> Decimal {
    let amount = uint128_to_decimal(coin.amount);

    decimal_multiplication_in_256(amount, exchange_rate)
}

fn query_exchange_rate(
    querier: &TerraQuerier,
    vault_denom: &str,
    coin_denom: &str,
) -> Option<Decimal> {
    let result = querier.query_exchange_rates(coin_denom, vec![vault_denom]);
    if result.is_err() {
        return None;
    }
    let exchange_rate = result
        .unwrap()
        .exchange_rates
        .first()
        .unwrap()
        .exchange_rate;

    Some(exchange_rate)
}

fn query_state(deps: Deps) -> StdResult<State> {
    let state = STATE.load(deps.storage)?;
    Ok(state)
}

fn query_config(deps: Deps) -> StdResult<Config> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

fn query_all_validator_metrics(
    deps: Deps,
    addr: Addr,
) -> StdResult<Vec<(Vec<u8>, ValidatorMetrics)>> {
    METRICS_HISTORY
        .prefix(&addr)
        .range(deps.storage, None, None, Order::Ascending)
        .collect()
}

fn query_all_validator_metrics_btw_timestamps(
    deps: Deps,
    addr: Addr,
    timestamp1: u64,
    timestamp2: u64,
) -> StdResult<Vec<(Vec<u8>, ValidatorMetrics)>> {
    if timestamp1.ge(&timestamp2) {
        return Err(StdError::GenericErr {
            msg: "timestamp1 cannot be greater than or equal to timestamp2".to_string(),
        });
    }
    let from = Some(Bound::Inclusive(U64Key::new(timestamp1).into()));
    let to = Some(Bound::Inclusive(U64Key::new(timestamp2).into()));

    METRICS_HISTORY
        .prefix(&addr)
        .range(deps.storage, from, to, Order::Ascending)
        .collect()
}

fn query_validator_metrics_by_timestamp(
    deps: Deps,
    addr: Addr,
    timestamp: u64,
) -> StdResult<ValidatorMetrics> {
    METRICS_HISTORY.load(deps.storage, (&addr, U64Key::new(timestamp)))
}

fn query_validators_metrics_by_timestamp(
    deps: Deps,
    timestamp: u64,
    from: u64,
    to: u64,
) -> StdResult<Vec<ValidatorMetrics>> {
    let validators = STATE.load(deps.storage)?.validators;
    let mut validator_operator_addr = vec![];
    for validator_addr_info in validators {
        validator_operator_addr.push(validator_addr_info.operator_address.clone());
    }

    let total_validators: u64 = validator_operator_addr.len() as u64;

    if to.ge(&total_validators) || from > to {
        return Err(StdError::GenericErr {
            msg: "Invalid indexes!".to_string(),
        });
    }

    let mut start = from;
    let mut res: Vec<ValidatorMetrics> = vec![];
    while start.le(&to) {
        // note: this change will help with missing validator / timestamps due to deletion.
        // let fetch_metric_result = METRICS_HISTORY.load(
        //     deps.storage,
        //     (&validator_operator_addr[start as usize], U64Key::new(timestamp)));
        // //note: this change is required, as deletion is now a possibility,
        // // and this validators metrics might not exist for this timestamp
        // if let Ok(metric) = fetch_metric_result {
        //     res.push(metric);
        // }
        res.push(METRICS_HISTORY.load(
            deps.storage,
            (
                &validator_operator_addr[start as usize],
                U64Key::new(timestamp),
            ),
        )?);
        start += 1;
    }
    Ok(res)
}

// off chain code

fn save_off_chain_details(
    deps: DepsMut,
    info: MessageInfo,
    timestamp: u64,
    details: OffchainTimestampMetaData,
) -> Result<Response, ContractError> {
    if !sender_is_manager(&deps, &info) {
        return Err(ContractError::Unauthorized {});
    }

    let timestamp_meta_data =
        OFF_CHAIN_TIMESTAMP_META_DATA.may_load(deps.storage, U64Key::from(timestamp));

    if timestamp_meta_data.unwrap().is_some() {
        return Err(ContractError::OffChainDetailsAlreadyRecorded);
    }

    OFF_CHAIN_TIMESTAMP_META_DATA.save(deps.storage, U64Key::from(timestamp), &details)?;
    OFF_CHAIN_TIMESTAMPS.save(deps.storage, U64Key::from(timestamp), &true);

    return Ok(Response::new()
        .add_attribute("method", "save_off_chain_details")
        .add_attribute("status", "Ok"));
}

fn add_off_chain_validator(
    deps: DepsMut,
    info: MessageInfo,
    validator_addr: Addr,
) -> Result<Response, ContractError> {
    if !sender_is_manager(&deps, &info) {
        return Err(ContractError::Unauthorized {});
    }

    let off_chain_state = OFF_CHAIN_STATE.load(deps.storage)?;
    let optional_validator =
        OFF_CHAIN_VALIDATOR_IDX_MAPPING.may_load(deps.storage, &validator_addr)?;

    if optional_validator.is_some() {
        return Err(ContractError::ValidatorAlreadyExists {});
    }
    let next_validator_idx = off_chain_state.next_validator_idx;
    OFF_CHAIN_VALIDATOR_IDX_MAPPING.save(deps.storage, &validator_addr, &next_validator_idx);

    OFF_CHAIN_STATE.update(deps.storage, |mut s| -> StdResult<_> {
        s.next_validator_idx = next_validator_idx + 1;
        Ok(s)
    });

    Ok(Response::new()
        .add_attribute("validator_idx", next_validator_idx.to_string())
        .add_attribute("validator_addr", validator_addr))
}

fn remove_off_chain_metrics_for_timestamp(
    deps: DepsMut,
    info: MessageInfo,
    timestamp: u64,
    no_of_validators_to_remove: u16,
) -> Result<Response, ContractError> {
    let mut timestamp_existed = true;
    let mut timestamp_removed = false;
    let mut validators_removed = 0;

    if !sender_is_manager(&deps, &info) {
        return Err(ContractError::Unauthorized {});
    }

    if !OFF_CHAIN_TIMESTAMPS.has(deps.storage, U64Key::from(timestamp)) {
        timestamp_existed = false;
    } else {
        let validator_idxs_to_remove: Vec<u16> = OFF_CHAIN_STATE_FOR_VALIDATOR
            .prefix(U64Key::from(timestamp))
            .range(deps.storage, Option::None, Option::None, Order::Ascending)
            .take(no_of_validators_to_remove as usize)
            .map(|item| return item.unwrap().1.validator_idx)
            .collect();

        validators_removed = validator_idxs_to_remove.len();

        validator_idxs_to_remove.into_iter().for_each(|idx| {
            OFF_CHAIN_STATE_FOR_VALIDATOR
                .remove(deps.storage, (U64Key::from(timestamp), U16Key::from(idx)))
        });

        if (validators_removed as u16) < (no_of_validators_to_remove as u16) {
            timestamp_removed = true;
            OFF_CHAIN_TIMESTAMPS.remove(deps.storage, U64Key::from(timestamp));
            OFF_CHAIN_TIMESTAMP_META_DATA.remove(deps.storage, U64Key::from(timestamp));
        }
    }

    Ok(Response::new()
        .add_attribute("method", "remove_off_chain_metrics_for_timestamp")
        .add_attribute(
            "timestamp removed : ",
            if timestamp_existed && timestamp_removed {
                "timestamp successfully removed"
            } else if timestamp_existed && !timestamp_removed {
                "timestamp not fully removed, need to run more paginated requests"
            } else {
                "timestamp fully removed"
            },
        )
        .add_attribute(
            "validator metrics removed : ",
            validators_removed.to_string(),
        ))
}

fn add_off_chain_validator_metrics(
    deps: DepsMut,
    info: MessageInfo,
    timestamp: u64,
    metrics_to_be_added: Vec<OffChainValidatorMetrics>,
) -> Result<Response, ContractError> {
    if !sender_is_manager(&deps, &info) {
        return Err(ContractError::Unauthorized {});
    }

    for validator_metric in metrics_to_be_added {
        let validator_idx = OFF_CHAIN_VALIDATOR_IDX_MAPPING
            .load(deps.storage, &validator_metric.opr_address)
            .unwrap();

        if off_chain_metrics_exists(&deps, timestamp, validator_idx) {
            return Err(ContractError::OffChainMetricsAlreadyRecorded);
        }

        OFF_CHAIN_STATE_FOR_VALIDATOR.save(
            deps.storage,
            (U64Key::from(timestamp), U16Key::from(validator_idx)),
            &validator_metric,
        );
    }

    Ok(Response::new()
        .add_attribute("method", "add_off_chain_validator_metrics")
        .add_attribute("status", "successful"))
}

fn off_chain_metrics_exists(deps: &DepsMut, timestamp: u64, validator_idx: u16) -> bool {
    OFF_CHAIN_STATE_FOR_VALIDATOR
        .may_load(
            deps.storage,
            (U64Key::from(timestamp), U16Key::from(validator_idx)),
        )
        .unwrap()
        .is_some()
}

fn get_off_chain_metrics_timestamps(deps: Deps) -> StdResult<OffChainTimestamps> {
    let keys: Vec<Vec<u8>> = OFF_CHAIN_TIMESTAMPS
        .keys(deps.storage, Option::None, Option::None, Order::Ascending)
        .collect();

    let off_chain_timestamps: Vec<u64> = keys
        .into_iter()
        .map(|item| conversion_utils::u64_from_vec_u8(item))
        .collect();

    Ok(OffChainTimestamps {
        timestamps: off_chain_timestamps,
    })
}

fn get_off_chain_metrics(
    deps: Deps,
    timestamp: u64,
    validator_addr: Addr,
) -> StdResult<OffChainValidatorMetrics> {
    let validator_idx = OFF_CHAIN_VALIDATOR_IDX_MAPPING.load(deps.storage, &validator_addr)?;
    let keys: Vec<Vec<u8>> = OFF_CHAIN_STATE_FOR_VALIDATOR
        .keys(deps.storage, Option::None, Option::None, Order::Ascending)
        .collect();
    let off_chain_state = OFF_CHAIN_STATE_FOR_VALIDATOR.load(
        deps.storage,
        (U64Key::from(timestamp), U16Key::from(validator_idx)),
    )?;
    Ok(off_chain_state)
}

fn get_off_chain_validators(deps: Deps) -> StdResult<OffChainValidators> {
    let keys: Vec<Vec<u8>> = OFF_CHAIN_VALIDATOR_IDX_MAPPING
        .keys(deps.storage, Option::None, Option::None, Order::Ascending)
        .collect();

    let off_chain_validator_addresses: Vec<Addr> = keys
        .into_iter()
        .map(|item| conversion_utils::addr_from_vec_u8(item))
        .collect();

    Ok(OffChainValidators {
        validator_addresses: off_chain_validator_addresses,
    })
}

fn get_off_chain_timestamp_meta_data(
    deps: Deps,
    timestamp: u64,
) -> StdResult<OffchainTimestampMetaData> {
    let timestamp_meta_data =
        OFF_CHAIN_TIMESTAMP_META_DATA.load(deps.storage, U64Key::from(timestamp))?;
    Ok(timestamp_meta_data)
}

fn get_off_chain_state(deps: Deps) -> StdResult<OffChainState> {
    let off_chain_state = OFF_CHAIN_STATE.load(deps.storage)?;
    Ok(off_chain_state)
}


// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::constants::OFF_CHAIN_METRICS_FOR_VALIDATOR;
//     use crate::msg::{ConversionRatio, OffChainValidatorMetrics};
//     use cosmwasm_std::testing::{
//         mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
//     };
//     use cosmwasm_std::{coins, OwnedDeps, Uint128};
//
//     const TEST_VALIDATOR_OPR_ADDR: &str = "valid0001";
//     const TEST_VALIDATOR_ACC_ADDR: &str = "validacc001";
//     const TEST_VALIDATOR_OPR_ADDR_2: &str = "valid0002";
//     const TEST_VALIDATOR_ACC_ADDR_2: &str = "validacc002";
//     const TEST_OWNER_ADDR: &str = "owner001";
//
//     const TEST_DENOM: &str = "luna";
//     const TEST_TIMESTAMP_1: u64 = 1638180000000;
//     const TEST_TIMESTAMP_2: u64 = 1638190000000;
//
//     fn instantiate_test_contract() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
//         let mut dependencies = mock_dependencies(&[]);
//         let env = mock_env();
//         let coin = Coin::new(1000, TEST_DENOM);
//         let msg_info = MessageInfo {
//             sender: Addr::unchecked(TEST_OWNER_ADDR),
//             funds: vec![coin],
//         };
//         let instantiate_msg = InstantiateMsg {
//             vault_denom: TEST_DENOM.to_string(),
//             amount_to_stake_per_validator: Uint128::new(10),
//             batch_size: 10,
//         };
//         instantiate(dependencies.as_mut(), env, msg_info, instantiate_msg).unwrap();
//         dependencies
//     }
//
//     #[test]
//     fn test_instantiate() {
//         let mut deps = mock_dependencies(&coins(2, TEST_DENOM));
//
//         let msg = InstantiateMsg {
//             amount_to_stake_per_validator: Uint128::new(10),
//             vault_denom: TEST_DENOM.to_string(),
//             batch_size: 10,
//         };
//         let info = mock_info("creator", &coins(2, TEST_DENOM));
//         let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
//     }
//
//     #[test]
//     fn test_record_metrics() {
//         let mut deps = initiate_test_validators_and_metrics();
//         let info = get_test_msg_info();
//         let env = mock_env();
//         let timestamp = 10;
//
//         let _res = record_validator_metrics(deps.as_mut(), env, info, timestamp);
//     }
//
//     #[test]
//     fn test_get_all_validator_metrics() {
//         let deps = mock_dependencies(&[]);
//         let validator = Addr::unchecked(TEST_VALIDATOR_OPR_ADDR);
//         let _res = query_all_validator_metrics(deps.as_ref(), validator);
//     }
//
//     #[test]
//     fn test_add_validator() {
//         let mut deps = mock_dependencies(&[]);
//         let info = mock_info("creator", &[]);
//         let validator_opr = Addr::unchecked(TEST_VALIDATOR_OPR_ADDR);
//         let validator_acc = Addr::unchecked(TEST_VALIDATOR_ACC_ADDR).to_string();
//         let _res = add_validator(deps.as_mut(), info, validator_opr, validator_acc);
//     }
//
//     #[test]
//     fn test_delete_metrics_for_validator() {
//         //initiate state
//         let mut dependencies = initiate_test_validators_and_metrics();
//
//         let state = STATE.load(&dependencies.storage).unwrap();
//         let metrics = METRICS_HISTORY.load(
//             &dependencies.storage,
//             (
//                 &Addr::unchecked(TEST_VALIDATOR_OPR_ADDR),
//                 U64Key::from(TEST_TIMESTAMP_1),
//             ),
//         );
//
//         // check if state has the timestamp
//         assert_eq!(2, state.cron_timestamps.len());
//         // check if metrics response is giving data
//         assert!(metrics.is_ok());
//         // check if remove validator removes both validator and metrics
//         let result = delete_metrics_for_validator(
//             dependencies.as_mut(),
//             get_test_msg_info(),
//             Addr::unchecked(TEST_VALIDATOR_OPR_ADDR),
//             0,
//             1,
//         );
//
//         let timestamps_left = result
//             .unwrap()
//             .attributes
//             .into_iter()
//             .find(|item| item.key.eq("timestamps_left"))
//             .unwrap()
//             .value;
//
//         assert_eq!(timestamps_left, "1");
//     }
//
//     //tests if the metrics and timestamp exists, and upon deletion of timestamp both go away
//     #[test]
//     fn test_delete_metrics_for_timestamp() {
//         //initiate state
//         let mut dependencies = initiate_test_validators_and_metrics();
//
//         let state = STATE.load(&dependencies.storage).unwrap();
//         let metrics = METRICS_HISTORY.load(
//             &dependencies.storage,
//             (
//                 &Addr::unchecked(TEST_VALIDATOR_OPR_ADDR),
//                 U64Key::from(TEST_TIMESTAMP_1),
//             ),
//         );
//
//         // check if state has the timestamp
//         assert_eq!(2, state.cron_timestamps.len());
//         // check if metrics response is giving data
//         assert!(metrics.is_ok());
//
//         // delete one validator from this timestamp metrics
//         let result = delete_metrics_for_timestamp(
//             dependencies.as_mut(),
//             get_test_msg_info(),
//             TEST_TIMESTAMP_1,
//             0,
//             1,
//         );
//
//         // check if result is okay
//         assert!(result.as_ref().ok().is_some());
//         let validators_left = result
//             .unwrap()
//             .attributes
//             .into_iter()
//             .find(|attribute| attribute.key.eq("validators_left"))
//             .unwrap()
//             .value;
//
//         // assert that one validator is still left for the timestamp
//         assert_eq!(validators_left, "1");
//
//         // delete second validator from this timestamp metrics
//         let result = delete_metrics_for_timestamp(
//             dependencies.as_mut(),
//             get_test_msg_info(),
//             TEST_TIMESTAMP_1,
//             1,
//             1,
//         );
//
//         // check if result is okay
//         assert!(result.as_ref().ok().is_some());
//         let validators_left = result
//             .unwrap()
//             .attributes
//             .into_iter()
//             .find(|attribute| attribute.key.eq("validators_left"))
//             .unwrap()
//             .value;
//
//         // assert that all validators are deleted for this timestamp
//         assert_eq!(validators_left, "0");
//
//         let metrics = METRICS_HISTORY.load(
//             &dependencies.storage,
//             (
//                 &Addr::unchecked(TEST_VALIDATOR_OPR_ADDR),
//                 U64Key::from(TEST_TIMESTAMP_1),
//             ),
//         );
//
//         // check if metrics response is giving error, due to this metric not being available
//         assert!(metrics.is_err());
//     }
//
//     #[test]
//     fn test_delete_timestamp() {
//         let mut dependencies = initiate_test_validators_and_metrics();
//         delete_metrics_for_timestamp(
//             dependencies.as_mut(),
//             get_test_msg_info(),
//             TEST_TIMESTAMP_1,
//             0,
//             2,
//         );
//
//         let result = remove_timestamp(dependencies.as_mut(), get_test_msg_info(), TEST_TIMESTAMP_1);
//     }
//
//     fn initiate_test_validators_and_metrics() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
//         let mut dependencies = instantiate_test_contract();
//
//         // initiate state to have two timestamp metrics for two validators
//         let _msg = get_test_msg_info();
//
//         // initiate state
//         let _ = STATE.update(dependencies.as_mut().storage, |mut s| -> StdResult<_> {
//             s.validators = get_test_validators();
//             s.cron_timestamps = vec![TEST_TIMESTAMP_1, TEST_TIMESTAMP_2];
//             Ok(s)
//         });
//
//         // initiate metrics
//         let _ = METRICS_HISTORY.save(
//             dependencies.as_mut().storage,
//             (
//                 &Addr::unchecked(TEST_VALIDATOR_OPR_ADDR),
//                 U64Key::from(TEST_TIMESTAMP_1),
//             ),
//             &get_test_metrics(TEST_VALIDATOR_OPR_ADDR, TEST_TIMESTAMP_1),
//         );
//         dependencies
//     }
//
//     fn get_test_msg_info() -> MessageInfo {
//         MessageInfo {
//             sender: Addr::unchecked(TEST_OWNER_ADDR),
//             funds: vec![Coin::new(1000, TEST_DENOM)],
//         }
//     }
//
//     fn get_test_metrics(validator_addr: &str, timestamp: u64) -> ValidatorMetrics {
//         ValidatorMetrics {
//             operator_addr: Addr::unchecked(validator_addr),
//             rewards: Default::default(),
//             delegated_amount: Default::default(),
//             self_delegated_amount: Default::default(),
//             slashing_pointer: Default::default(),
//             commission: Default::default(),
//             max_commission: Default::default(),
//             timestamp: timestamp,
//             rewards_in_coins: vec![],
//         }
//     }
//
//     fn get_test_validators() -> Vec<ValidatorAccounts> {
//         vec![
//             ValidatorAccounts {
//                 operator_address: Addr::unchecked(TEST_VALIDATOR_OPR_ADDR),
//                 account_address: Addr::unchecked(TEST_VALIDATOR_ACC_ADDR),
//             },
//             ValidatorAccounts {
//                 operator_address: Addr::unchecked(TEST_VALIDATOR_OPR_ADDR_2),
//                 account_address: Addr::unchecked(TEST_VALIDATOR_ACC_ADDR_2),
//             },
//         ]
//     }
//
//     #[test]
//     fn test_create_state_and_increment() {
//         let mut dependencies = instantiate_test_contract();
//         let off_chain_state = OFF_CHAIN_STATE.load(dependencies.as_mut().storage);
//
//         assert!(off_chain_state.is_ok());
//
//         add_off_chain_validator(
//             dependencies.as_mut(),
//             get_test_msg_info(),
//             Addr::unchecked(TEST_VALIDATOR_OPR_ADDR),
//         );
//
//         let updated_state = get_off_chain_state(dependencies.as_ref()).unwrap();
//
//         assert_eq!(1, updated_state.next_validator_idx);
//     }
//
//     #[test]
//     fn test_off_chain_timestamp_meta_data() {
//         let mut dependencies = instantiate_test_contract();
//         let timestamp = get_test_timestamp_0();
//         let before_data = get_off_chain_timestamp_meta_data(dependencies.as_ref(), timestamp);
//
//         assert!(before_data.is_err());
//
//         let saved_data = save_off_chain_details(
//             dependencies.as_mut(),
//             get_test_msg_info(),
//             timestamp,
//             get_test_off_chain_timestamp_meta_data(),
//         );
//
//         assert!(saved_data.is_ok());
//
//         let after_data = get_off_chain_timestamp_meta_data(dependencies.as_ref(), timestamp);
//
//         assert!(after_data.is_ok());
//         assert_eq!(after_data.unwrap().timestamp, timestamp);
//     }
//
//     #[test]
//     fn test_add_off_chain_validator_metrics() {
//         let mut dependencies = instantiate_test_contract();
//
//         add_off_chain_validator(
//             dependencies.as_mut(),
//             get_test_msg_info(),
//             Addr::unchecked(TEST_VALIDATOR_OPR_ADDR),
//         );
//
//         let off_chain_metrics_result = get_off_chain_metrics(
//             dependencies.as_ref(),
//             get_test_timestamp_0(),
//             Addr::unchecked(TEST_VALIDATOR_OPR_ADDR),
//         );
//
//         assert!(off_chain_metrics_result.is_err());
//
//         let saved_details = save_off_chain_details(
//             dependencies.as_mut(),
//             get_test_msg_info(),
//             get_test_timestamp_0(),
//             get_test_off_chain_timestamp_meta_data(),
//         );
//
//         assert!(saved_details.is_ok());
//
//         let test_metric = get_test_off_chain_validator_metric();
//
//         let saved_data = add_off_chain_validator_metrics(
//             dependencies.as_mut(),
//             get_test_msg_info(),
//             get_test_timestamp_0(),
//             vec![test_metric.clone()],
//         );
//
//         assert!(saved_data.is_ok());
//
//         let after_data = get_off_chain_metrics(
//             dependencies.as_ref(),
//             get_test_timestamp_0(),
//             test_metric.opr_address,
//         );
//
//         assert!(after_data.is_ok());
//     }
//
//     #[test]
//     fn test_get_off_chain_timestamps() {
//         let mut dependencies = instantiate_test_contract();
//
//         let mut test_timestamp = 100000;
//         let mut test_timestamp_meta_data = get_test_off_chain_timestamp_meta_data();
//
//         test_timestamp_meta_data.timestamp = test_timestamp;
//
//         let saved_details = save_off_chain_details(
//             dependencies.as_mut(),
//             get_test_msg_info(),
//             test_timestamp,
//             get_test_off_chain_timestamp_meta_data(),
//         );
//
//         assert!(saved_details.is_ok());
//
//         let timestamps = get_off_chain_metrics_timestamps(dependencies.as_ref());
//
//         assert!(timestamps.is_ok());
//     }
//
//     #[test]
//     fn test_delete_off_chain_metrics_timestamp() {
//         let mut dependencies = instantiate_test_contract();
//
//         let mut test_timestamp = 100000;
//         let mut test_timestamp_meta_data = get_test_off_chain_timestamp_meta_data();
//
//         test_timestamp_meta_data.timestamp = test_timestamp;
//
//         let add_validator = add_off_chain_validator(
//             dependencies.as_mut(),
//             get_test_msg_info(),
//             Addr::unchecked(TEST_VALIDATOR_OPR_ADDR),
//         );
//
//         assert!(add_validator.is_ok());
//
//         let saved_details = save_off_chain_details(
//             dependencies.as_mut(),
//             get_test_msg_info(),
//             test_timestamp,
//             get_test_off_chain_timestamp_meta_data(),
//         );
//
//         assert!(saved_details.is_ok());
//
//         let mut test_metric = get_test_off_chain_validator_metric().clone();
//
//         let saved_data = add_off_chain_validator_metrics(
//             dependencies.as_mut(),
//             get_test_msg_info(),
//             test_timestamp,
//             vec![test_metric],
//         );
//         let delete_off_chain_timestamp = remove_off_chain_metrics_for_timestamp(
//             dependencies.as_mut(),
//             get_test_msg_info(),
//             test_timestamp,
//             10,
//         );
//
//         assert!(delete_off_chain_timestamp.is_ok());
//     }
//
//     #[test]
//     fn test_get_off_chain_validators() {
//         let mut dependencies = instantiate_test_contract();
//
//         add_off_chain_validator(
//             dependencies.as_mut(),
//             get_test_msg_info(),
//             Addr::unchecked(TEST_VALIDATOR_OPR_ADDR),
//         );
//
//         let get_off_chain_validators = get_off_chain_validators(dependencies.as_ref());
//
//         assert!(get_off_chain_validators.is_ok());
//
//         assert_eq!(
//             get_off_chain_validators.unwrap().validator_addresses.len(),
//             1
//         );
//     }
//
//     fn get_test_off_chain_timestamp_meta_data() -> OffchainTimestampMetaData {
//         OffchainTimestampMetaData {
//             timestamp: get_test_timestamp_0(),
//             conversion_ratios_to_luna: vec![
//                 (ConversionRatio {
//                     denomination: "".to_string(),
//                     multiplier: "156.00".to_string(),
//                 }),
//             ],
//         }
//     }
//
//     fn get_test_timestamp_0() -> u64 {
//         0
//     }
//
//     fn get_test_off_chain_validator_metric() -> OffChainValidatorMetrics {
//         OffChainValidatorMetrics {
//             validator_idx: 0,
//             opr_address: Addr::unchecked(TEST_VALIDATOR_OPR_ADDR),
//             apr: "110.00".to_string(),
//         }
//     }
// }
