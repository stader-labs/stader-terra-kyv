use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Item, Map, U64Key};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub last_epoch_cron_time: u64,
    pub vault_denom: String,
    pub manager: Addr,
    pub amount_to_stake_per_validator: Uint128,
    pub validator_update_timings: Vec<ValidatorUpdateTimings>,
    pub batch_size: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ValidatorUpdateTimings {
    pub addr: Addr,
    pub updated_time: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ValidatorMetrics {
    pub addr: Addr,                // Validator Address
    pub rewards: Decimal,          // these are cummulative rewards
    pub delegated_amount: Uint128, // For tracking the amount delegated (With slashing)
    pub commission: Decimal,
    pub max_commission: Decimal,
}

pub const METRICS_HISTORY: Map<U64Key, Vec<ValidatorMetrics>> =
    Map::new("validator_metrics_history");

pub const STATE: Item<State> = Item::new("state");
