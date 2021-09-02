use cosmwasm_std::{Addr, Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub vault_denom: String,
    pub amount_to_stake_per_validator: Uint128,
    pub batch_size: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    RecordMetrics { timestamp: u64 }, // 12:00AM
    AddValidator { addr: Addr },
    UpdateConfig { batch_size: u32 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetAllValidatorMetricsByTime {
        timestamp: u64,
    },
    GetValidatorMetricsByTime {
        timestamp: u64,
        addr: Addr,
    },
    GetState {},
    GetAllAprsByInterval {
        timestamp1: u64,
        timestamp2: u64,
    },
    GetAprByValidator {
        timestamp1: u64,
        timestamp2: u64,
        addr: Addr,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ValidatorAprResponse {
    pub addr: Addr,
    pub apr: Decimal,
}
