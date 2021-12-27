use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use cw_storage_plus::{Item, Map, U64Key};
use crate::constants;
use crate::msg::{OffchainTimestampDetails, OffChainTimestamps, OffChainValidatorMetrics};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub vault_denom: String,
    // note: querying validator by address is costly, as it requires iteration, can
    // be optimized with a cleaner way to query, but might not be important, as number of validators
    // is not expected to grow beyond 20-30.
    pub validators: Vec<ValidatorAccounts>,
    //hard to remove from this, costs O(T) time, if was a set, could be O(1) average time
    pub cron_timestamps: Vec<u64>,
    pub validator_index_for_next_cron: u64
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OffChainState {
    pub new_validator_idx: u16

}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub manager: Addr,
    pub amount_to_stake_per_validator: Uint128,
    pub batch_size: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ValidatorAccounts {
    pub operator_address: Addr,
    pub account_address: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ValidatorMetrics {
    pub operator_addr: Addr,       // Validator's operator address
    pub rewards: Decimal,          // these are cummulative rewards
    pub delegated_amount: Uint128, // For tracking the amount delegated (With slashing)
    pub self_delegated_amount: Uint128,
    pub slashing_pointer: Decimal, // to track the slashed amount (current_delegated_amount/prev_delegated_amount)*prev_slashing_pointer
    pub commission: Decimal,
    pub max_commission: Decimal,
    pub timestamp: u64,
    pub rewards_in_coins: Vec<Coin>,
}


// (Validator Addr, Timestamp)
pub const METRICS_HISTORY: Map<(&Addr, U64Key), ValidatorMetrics> =
    Map::new("validator_metrics_history");

pub const STATE: Item<State> = Item::new("state");

pub const CONFIG: Item<Config> = Item::new("config");


// off chain details

pub const OFF_CHAIN_STATE: Item<OffChainState> = Item::new(constants::OFF_CHAIN_STATE);

pub const OFF_CHAIN_VALIDATOR_IDX_MAPPING: Map<&Addr, u16> =
    Map::new(constants::OFF_CHAIN_VALIDATOR_IDX_MAPPING);

pub const OFF_CHAIN_TIMESTAMP_DETAILS: Map<U64Key, OffchainTimestampDetails> =
    Map::new(constants::OFFCHAIN_TIMESTAMP_DETAILS);

pub const OFF_CHAIN_STATE_FOR_VALIDATOR: Map<(U64Key, Addr), OffChainValidatorMetrics> =
    Map::new(constants::OFF_CHAIN_METRICS_FOR_VALIDATOR);

pub const OFF_CHAIN_TIMESTAMPS: Item<OffChainTimestamps> = Item::new("off_chain_timestamps");
/*
 create a map of validators address to an index mapping [o]
 (apr)
 key storage might be redundant for a timestamp
 > Mapping to an index, incrementing id for a validator
 > Map<Addr, Index> address_mapping;
 > next_validator_ct = 0...         U16
 > Off_Chain_state : Take care in initiate and migrate

 > Push metrics : (Data will be in validator_addr, we can resolve in contract)
                : Ensures loose coupling instead of tight
 */


// pub exchange_rates: Vec<(String, Decimal)>, // FOR TESTING - REMOVE THIS
