use crate::state::{OffChainValidatorMetrics, OffchainTimestampMetaData};
use cosmwasm_std::{Addr, Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub vault_denom: String,
    pub amount_to_stake_per_validator: Uint128,
    pub batch_size: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    RecordMetrics {
        timestamp: u64,
    }, // 12:00AM
    AddValidator {
        validator_opr_addr: Addr,
        account_addr: String,
    }, //validator's operator address,validator's account address
    UpdateConfig {
        batch_size: u64,
    },
    RemoveValidator {
        validator_oper_addr: Addr,
    },
    WithdrawFunds {
        amount: Uint128,
    },
    DeleteMetricsForTimestamp {
        timestamp: u64,
        validator_idx: u64,
        validator_ct: u64,
    }, // used to delete all metrics associated with the timestamp, along with the timestamp record
    DeleteMetricsForValidator {
        validator_opr_addr: Addr,
        timestamp_idx: u64,
        timestamp_ct: u64,
    },
    RemoveTimestamp {
        timestamp: u64,
    },
    RemoveOffChainMetricsForTimestamp {
        timestamp: u64,
        no_of_validators_to_remove: u16,
    },
    OffChainAddValidator {
        oper_addr: Addr,
    },
    OffChainRecordTimestampMetaData {
        timestamp: u64,
        timestamp_meta_data: OffchainTimestampMetaData,
    },
    OffChainAddValidatorMetricsForTimestamp {
        timestamp: u64,
        validator_metrics: Vec<OffChainValidatorMetrics>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetAllTimestamps {},
    GetAllValidatorMetrics {
        addr: Addr,
    },
    GetValidatorMetricsBtwTimestamps {
        addr: Addr,
        timestamp1: u64,
        timestamp2: u64,
    },
    GetValidatorMetricsByTimestamp {
        timestamp: u64,
        addr: Addr,
    },
    GetValidatorsMetricsByTimestamp {
        timestamp: u64,
        from: u64,
        to: u64,
    },
    GetState {},
    GetConfig {},
    GetAllAprsByInterval {
        timestamp1: u64,
        timestamp2: u64,
        from: u64,
        to: u64,
    },
    GetAprByValidator {
        timestamp1: u64,
        timestamp2: u64,
        addr: Addr,
    },
    GetOffChainValidatorMetrics {
        timestamp: u64,
        validator_addr: Addr,
    },
    GetOffChainState {},
    GetOffChainMetricsTimestamps {},
    GetOffChainTimestampMetaData {
        timestamp: u64,
    },
    GetOffChainValidators {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {
    pub manager_address: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ValidatorAprResponse {
    pub addr: Addr,
    pub apr: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct OffChainTimestamps {
    pub timestamps: Vec<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct OffChainValidators {
    pub validator_addresses: Vec<Addr>,
}
