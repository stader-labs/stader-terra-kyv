use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use cw_storage_plus::{Item, Map, U64Key};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub vault_denom: String,
    pub validators: Vec<Addr>,
    pub validators_account_addr: Vec<Addr>, //account addresses of validators
    pub cron_timestamps: Vec<u64>,
    pub validator_index_for_next_cron: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub manager: Addr,
    pub amount_to_stake_per_validator: Uint128,
    pub batch_size: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ValidatorMetrics {
    pub addr: Addr,                // Validator Address
    pub rewards: Decimal,          // these are cummulative rewards
    pub delegated_amount: Uint128, // For tracking the amount delegated (With slashing)
    pub self_delegated_amount: Uint128,
    pub commission: Decimal,
    pub max_commission: Decimal,
    pub timestamp: u64,
    pub rewards_in_coins: Vec<Coin>,
}

pub const METRICS_HISTORY: Map<(&Addr, U64Key), ValidatorMetrics> =
    Map::new("validator_metrics_history");

pub const STATE: Item<State> = Item::new("state");

pub const CONFIG: Item<Config> = Item::new("config");

// pub exchange_rates: Vec<(String, Decimal)>, // FOR TESTING - REMOVE THIS
