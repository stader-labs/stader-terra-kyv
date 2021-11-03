use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ValidatorAprResponse};
use crate::state::ValidatorAccounts;
use crate::state::{Config, State, ValidatorMetrics, CONFIG, METRICS_HISTORY, STATE};
use crate::util::{
    compute_apr, decimal_division_in_256, decimal_multiplication_in_256, decimal_summation_in_256,
    uint128_to_decimal,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::Decimal;
use cosmwasm_std::{
    to_binary, Addr, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Order, Response, StakingMsg,
    StdError, StdResult, Storage, Uint128,
};
use cw_storage_plus::{Bound, U64Key};
use std::cmp;
use std::collections::HashMap;
use std::convert::TryInto;
use std::ops::Sub;
use terra_cosmwasm::TerraQuerier;

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
    STATE.save(deps.storage, &state)?;

    let config = Config {
        manager: info.sender.clone(),
        amount_to_stake_per_validator: msg.amount_to_stake_per_validator,
        batch_size: msg.batch_size,
    };
    CONFIG.save(deps.storage, &config)?;

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
    }
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
    }
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

    let total_validators: u64 = validators.len().try_into().unwrap();

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

    let manager = CONFIG.load(deps.storage)?.manager;
    // can only be updated by manager
    if info.sender != manager {
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

pub fn record_validator_metrics(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    timestamp: u64,
) -> Result<Response, ContractError> {
    let manager = CONFIG.load(deps.storage)?.manager;
    // can only be called by manager
    if info.sender != manager {
        return Err(ContractError::Unauthorized {});
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
        let delegation_opt = deps.querier.query_delegation(
            &env.contract.address,
            validator_addr.operator_address.clone(),
        )?;

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
        }

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

    // If validaor is added after the prevous cron run, then there wont be any prev history for this validator
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
    vault_denom: &String,
    exchange_rates_map: &mut HashMap<String, Decimal>,
    querier: &TerraQuerier,
) -> Decimal {
    let mut current_rewards: Decimal = Decimal::zero();
    for coin in rewards {
        // Tries to find the exchange rate in the hashmap,
        // If not present we fetch the exchange rate and add it to the map before calculating reward
        let reward_for_coin =
            get_amount_in_vault_denom(coin, vault_denom, exchange_rates_map, querier);
        if reward_for_coin.is_some() {
            current_rewards = decimal_summation_in_256(reward_for_coin.unwrap(), current_rewards);
        } // If exchange rate is not fetchable then we skip such reward ?
    }
    current_rewards
}

fn get_validators_to_record(
    storage: &mut dyn Storage,
    timestamp: u64,
) -> Result<Vec<ValidatorAccounts>, ContractError> {
    let batch_size = CONFIG.load(storage)?.batch_size;
    let state = STATE.load(storage)?;
    let last_cron_time_opt = state.cron_timestamps.last();
    let validators = state.validators;
    let total_validators: u64 = validators.len().try_into().unwrap();
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
    vault_denom: &String,
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
    vault_denom: &String,
    coin_denom: &String,
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

    let total_validators: u64 = validator_operator_addr.len().try_into().unwrap();

    if to.ge(&total_validators) || from > to {
        return Err(StdError::GenericErr {
            msg: "Invalid indexes!".to_string(),
        });
    }

    let mut start = from;
    let mut res: Vec<ValidatorMetrics> = vec![];
    while start.le(&to) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, Uint128};

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        let msg = InstantiateMsg {
            amount_to_stake_per_validator: Uint128::new(10),
            vault_denom: "luna".to_string(),
            batch_size: 10,
        };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    #[test]
    fn test_record_metrics() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("creator", &[]);
        let env = mock_env();
        let timestamp = 10;

        let _res = record_validator_metrics(deps.as_mut(), env, info, timestamp);
    }

    #[test]
    fn test_get_all_validator_metrics() {
        let deps = mock_dependencies(&[]);
        let validator = Addr::unchecked("valid0001");
        let _res = query_all_validator_metrics(deps.as_ref(), validator);
    }

    #[test]
    fn test_add_validator() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("creator", &[]);
        let validator_opr = Addr::unchecked("valid0001");
        let validator_acc = Addr::unchecked("valid0002").to_string();
        let _res = add_validator(deps.as_mut(), info, validator_opr, validator_acc);
    }
}
