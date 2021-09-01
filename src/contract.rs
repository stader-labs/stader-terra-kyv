use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ValidatorAprResponse};
use crate::state::{State, ValidatorMetrics, ValidatorUpdateTimings, METRICS_HISTORY, STATE};
use cosmwasm_bignumber::Decimal256;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, StakingMsg, StdError,
    StdResult, Storage,
};
use cosmwasm_std::{Decimal, FullDelegation, Uint128};
use cw_storage_plus::U64Key;
use std::collections::HashMap;
use terra_cosmwasm::TerraQuerier;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        last_epoch_cron_time: _env.block.time.seconds(), // Can be just 0 as well
        manager: info.sender.clone(),
        vault_denom: msg.vault_denom.clone(),
        amount_to_stake_per_validator: msg.amount_to_stake_per_validator,
        validator_update_timings: vec![],
        max_records_to_update_per_run: msg.max_records_to_update_per_run,
    };

    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("manager", info.sender)
        .add_attribute(
            "last_epoch_cron_time",
            _env.block.time.seconds().to_string(),
        )
        .add_attribute(
            "amount_to_stake_per_validator",
            msg.amount_to_stake_per_validator,
        )
        .add_attribute("vault_denom", msg.vault_denom.clone().to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::RecordMetrics { timestamp } => record_validator_metrics(deps, env, timestamp),
        ExecuteMsg::AddValidator { addr } => add_validator(deps, env, info, addr),
        ExecuteMsg::UpdateRecordsToUpdatePerRun { no } => {
            update_records_to_update_per_run(deps, no)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCurrentState {} => to_binary(&query_state(deps)?),
        QueryMsg::GetHistoryByTime { timestamp } => {
            to_binary(&query_validator_history(deps, timestamp)?)
        }
        QueryMsg::GetAllAprsByInteral {
            timestamp1,
            timestamp2,
        } => to_binary(&query_all_validators_aprs(deps, timestamp1, timestamp2)?),
        QueryMsg::GetAprByValidator {
            timestamp1,
            timestamp2,
            addr,
        } => to_binary(&query_validator_apr(deps, timestamp1, timestamp2, addr)?),
    }
}

fn query_validator_apr(
    deps: Deps,
    timestamp1: u64,
    timestamp2: u64,
    addr: Addr,
) -> StdResult<ValidatorAprResponse> {
    if timestamp1.eq(&timestamp2) {
        return Err(StdError::GenericErr {
            msg: "timestamp1 and timestamp2 cannot be the same".to_string(),
        });
    }

    let h1_opt = METRICS_HISTORY
        .load(deps.storage, U64Key::new(timestamp1))?
        .into_iter()
        .find(|history| history.addr.eq(&addr));

    let h2_opt = METRICS_HISTORY
        .load(deps.storage, U64Key::new(timestamp2))?
        .into_iter()
        .find(|history| history.addr.eq(&addr));

    if h1_opt.is_none() || h2_opt.is_none() {
        return Err(StdError::GenericErr {
            msg: "Validator does'nt have metrics recorded for all the given times".to_string(),
        });
    }

    return Ok(ValidatorAprResponse {
        addr,
        apr: compute_apr(&h1_opt.unwrap(), &h2_opt.unwrap(), timestamp2 - timestamp1),
    });
}

fn convert_validator_metrics_to_map(
    metrics: Vec<ValidatorMetrics>,
) -> HashMap<Addr, ValidatorMetrics> {
    let mut map: HashMap<Addr, ValidatorMetrics> = HashMap::new();
    for metric in metrics {
        map.insert(metric.addr.clone(), metric);
    }
    map
}

fn query_all_validators_aprs(
    deps: Deps,
    timestamp1: u64,
    timestamp2: u64,
) -> StdResult<Vec<ValidatorAprResponse>> {
    if timestamp1.eq(&timestamp2) {
        return Err(StdError::GenericErr {
            msg: "timestamp1 and timestamp2 cannot be the same".to_string(),
        });
    }

    let mut response: Vec<ValidatorAprResponse> = vec![];

    let history1_map = convert_validator_metrics_to_map(
        METRICS_HISTORY.load(deps.storage, U64Key::new(timestamp1))?,
    );

    let history2 = METRICS_HISTORY.load(deps.storage, U64Key::new(timestamp2))?;

    for h2 in history2 {
        let h1_opt = history1_map.get(&h2.addr);
        if let Some(h1) = h1_opt {
            let apr = compute_apr(h1, &h2, timestamp2 - timestamp1);
            response.push(ValidatorAprResponse { addr: h2.addr, apr });
        }
    }

    Ok(response)
}

fn update_records_to_update_per_run(deps: DepsMut, no: u32) -> Result<Response, ContractError> {
    STATE.update(deps.storage, |mut s| -> StdResult<_> {
        s.max_records_to_update_per_run = no;
        Ok(s)
    })?;
    Ok(Response::new().add_attribute("method", "update_records_to_update_per_run"))
}

fn add_validator(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    validator_addr: Addr,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    let vault_denom = state.vault_denom;
    let amount_to_stake_per_validator = state.amount_to_stake_per_validator;

    // can only be called by manager
    if info.sender != state.manager {
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
        .validator_update_timings
        .iter()
        .any(|ValidatorUpdateTimings { addr, .. }| addr.eq(&validator_addr))
    {
        return Err(ContractError::ValidatorAlreadyExists {});
    }

    let funds = info.funds.first();
    if funds.is_none() {
        return Err(ContractError::NoFundsFound {});
    }

    if !funds.unwrap().amount.eq(&amount_to_stake_per_validator) {
        return Err(ContractError::NotMatchingFunds {});
    }

    let msg = StakingMsg::Delegate {
        validator: validator_addr.to_string(),
        amount: Coin {
            denom: vault_denom.clone(),
            amount: amount_to_stake_per_validator,
        },
    };

    STATE.update(deps.storage, |mut s| -> StdResult<_> {
        s.validator_update_timings.push(ValidatorUpdateTimings {
            updated_time: env.block.time.seconds(),
            addr: validator_addr.clone(),
        });
        Ok(s)
    })?;

    // TODO: Maybe push an initial history for the latest time stamp ?

    Ok(Response::new()
        .add_messages([msg])
        .add_attribute("method", "add_validator"))
}

pub fn record_validator_metrics(
    deps: DepsMut,
    env: Env,
    timestamp: u64,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;

    let validators_to_record = update_and_get_validators_to_record(
        deps.storage,
        state.max_records_to_update_per_run,
        timestamp,
    )?;

    if validators_to_record.is_empty() {
        return Ok(Response::new()
            .add_attribute("method", "record_validator_metrics")
            .add_attribute("msg", "All validators are recorded for the given cron time"));
    }

    let current_validators_metrics = compute_current_metrics(&deps, env, &validators_to_record)?;

    METRICS_HISTORY.update(
        deps.storage,
        U64Key::new(timestamp),
        |history| -> StdResult<_> {
            if let Some(mut value) = history {
                for current_metric in current_validators_metrics {
                    value.push(current_metric);
                }
                return Ok(value);
            } else {
                return Ok(current_validators_metrics);
            }
        },
    )?;

    STATE.update(deps.storage, |mut s| -> StdResult<_> {
        s.last_epoch_cron_time = timestamp;
        Ok(s)
    })?;

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
    previous_update_timings: &Vec<ValidatorUpdateTimings>,
) -> Result<Vec<ValidatorMetrics>, ContractError> {
    let state = STATE.load(deps.storage)?;
    let vault_denom = state.vault_denom;

    let mut exchange_rates_map: HashMap<String, Decimal> = HashMap::new();
    exchange_rates_map.insert(vault_denom.clone(), Decimal::one());
    let querier = TerraQuerier::new(&deps.querier);

    let mut current_metrics: Vec<ValidatorMetrics> = vec![];

    for update in previous_update_timings {
        let ValidatorUpdateTimings {
            addr: validator_addr,
            ..
        } = update;
        let delegation_opt = deps
            .querier
            .query_delegation(&env.contract.address, validator_addr)?;

        if delegation_opt.is_some() {
            let delegation = delegation_opt.unwrap();
            let current_rewards = get_total_rewards_in_vault_denom(
                &delegation,
                &vault_denom,
                &mut exchange_rates_map,
                &querier,
            );

            // This is the new Delegated amount after slashing Ex: (10 => 9.8 etc.,)
            let current_delegated_amount = delegation.amount.amount.clone();
            current_metrics.push(ValidatorMetrics {
                addr: validator_addr.clone(),
                rewards: current_rewards,
                delegated_amount: current_delegated_amount,
            });
        } else {
            // TODO: You should take a look at - Validator timings are already updated above
            return Err(ContractError::NoDelegationFound {
                manager: env.contract.address.clone(),
                validator: validator_addr.clone(),
            });
        }
    }
    Ok(current_metrics)
}

fn get_total_rewards_in_vault_denom(
    delegation: &FullDelegation,
    vault_denom: &String,
    exchange_rates_map: &mut HashMap<String, Decimal>,
    querier: &TerraQuerier,
) -> Decimal {
    let accumulated_rewards = &delegation.accumulated_rewards;
    let mut current_rewards: Decimal = Decimal::zero();
    for coin in accumulated_rewards {
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

fn update_and_get_validators_to_record(
    storage: &mut dyn Storage,
    max_records_to_update_per_run: u32,
    timestamp: u64,
) -> Result<Vec<ValidatorUpdateTimings>, ContractError> {
    let mut validator_updates: Vec<ValidatorUpdateTimings> = vec![];
    let mut records_to_update: u32 = 0;

    STATE.update(storage, |mut s| -> StdResult<_> {
        s.validator_update_timings = s
            .validator_update_timings
            .into_iter()
            .map(|timing| {
                if records_to_update <= max_records_to_update_per_run
                    && timing.updated_time < timestamp
                {
                    validator_updates.push(timing.clone()); // Push the previous history before updating to current timing
                    records_to_update += 1;
                    return ValidatorUpdateTimings {
                        addr: timing.addr,
                        updated_time: timestamp,
                    };
                }
                return timing;
            })
            .collect();
        Ok(s)
    })?;

    Ok(validator_updates)
}

fn get_amount_in_vault_denom(
    coin: &Coin,
    vault_denom: &String,
    exchange_rates_map: &mut HashMap<String, Decimal>, // Try to bring it outside (As we are mutating a func param)
    querier: &TerraQuerier,
) -> Option<Decimal> {
    if exchange_rates_map.contains_key(&coin.denom) {
        let exchange_rate = exchange_rates_map.get(&coin.denom).unwrap();
        return Some(convert_amount_to_valut_denom(coin, *exchange_rate)); // Not sure how this * works!
    } else {
        let rate_opt = query_exchange_rate(querier, vault_denom, &coin.denom);
        if rate_opt.is_none() {
            return None;
        }
        let exchange_rate = rate_opt.unwrap();
        exchange_rates_map.insert(coin.denom.clone(), exchange_rate);
        return Some(convert_amount_to_valut_denom(coin, exchange_rate));
    }
}

fn convert_amount_to_valut_denom(coin: &Coin, exchange_rate: Decimal) -> Decimal {
    let amount = uint128_to_decimal(coin.amount);
    let amount_in_vault_denom = decimal_multiplication_in_256(amount, exchange_rate);
    amount_in_vault_denom
}

// Refactor to a helper
fn query_exchange_rate(
    querier: &TerraQuerier,
    vault_denom: &String,
    coin_denom: &String,
) -> Option<Decimal> {
    let result = querier.query_exchange_rates(vault_denom, vec![coin_denom]);
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

fn query_validator_history(deps: Deps, timestamp: u64) -> StdResult<Vec<ValidatorMetrics>> {
    METRICS_HISTORY.load(deps.storage, U64Key::new(timestamp))
}

// TODO: Reuse from an util
pub fn decimal_summation_in_256(a: Decimal, b: Decimal) -> Decimal {
    let a_u256: Decimal256 = a.into();
    let b_u256: Decimal256 = b.into();
    let c_u256: Decimal = (b_u256 + a_u256).into();
    c_u256
}

pub fn decimal_subtraction_in_256(a: Decimal, b: Decimal) -> Decimal {
    let a_u256: Decimal256 = a.into();
    let b_u256: Decimal256 = b.into();
    let c_u256: Decimal = (a_u256 - b_u256).into();
    c_u256
}

pub fn decimal_multiplication_in_256(a: Decimal, b: Decimal) -> Decimal {
    let a_u256: Decimal256 = a.into();
    let b_u256: Decimal256 = b.into();
    let c_u256: Decimal = (b_u256 * a_u256).into();
    c_u256
}

pub fn decimal_division_in_256(a: Decimal, b: Decimal) -> Decimal {
    let a_u256: Decimal256 = a.into();
    let b_u256: Decimal256 = b.into();
    let c_u256: Decimal = (a_u256 / b_u256).into();
    c_u256
}

pub fn uint128_to_decimal(num: Uint128) -> Decimal {
    let numerator: u128 = num.into();
    Decimal::from_ratio(numerator, 1_u128)
}

pub fn u64_to_decimal(num: u64) -> Decimal {
    let numerator: u128 = num.into();
    Decimal::from_ratio(numerator, 1_u128)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn easy_flow() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        let msg = InstantiateMsg {
            amount_to_stake_per_validator: Uint128::new(10),
            vault_denom: "luna".to_string(),
            max_records_to_update_per_run: 10,
        };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Add Validator
        // Invoke Record metrics here
    }
}

// function computeAPR(h1: ValidatorMetric, h2: ValidatorMetric) {
//     const numerator = (+h2.rewards - +h1.rewards) * (365 * 86400) * 100;
//     const denominator = +h2.delegated_amount * (h2.timestamp - h1.timestamp);
//     return (numerator / denominator).toFixed(3) + "%";
//   }

fn compute_apr(h1: &ValidatorMetrics, h2: &ValidatorMetrics, time_diff_in_seconds: u64) -> Decimal {
    let numerator = decimal_multiplication_in_256(
        decimal_subtraction_in_256(h2.rewards, h1.rewards),
        u64_to_decimal(3153600000), // (365 * 86400) * 100 => (365 * 86400) = Seconds in an year, 100 = percentage
    );

    let denominator = decimal_multiplication_in_256(
        uint128_to_decimal(h2.delegated_amount),
        u64_to_decimal(time_diff_in_seconds),
    );

    decimal_division_in_256(numerator, denominator)
}
