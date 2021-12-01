use cosmwasm_std::{Addr, StdError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Validator not present in block chain")]
    ValidatorDoesNotExist {},

    #[error("Validator is already added to record metrics")]
    ValidatorAlreadyExists {},

    #[error("No funds found")]
    NoFundsFound {},

    #[error("In sufficient funds for this action")]
    InsufficientFunds {},

    #[error("Something went wrong while getting the delegation??")]
    NoDelegationFound { manager: Addr, validator: Addr },

    #[error("Invalid timestamps provided")]
    InvalidTimestamps { msg: String },

    #[error("Batch size cannot be zero")]
    BatchSizeCannotBeZero {},

    #[error("Amount cannot be zero")]
    ZeroAmount {},

    #[error("No metrics found for the given timestamp range")]
    TimestampOutOfRange {},

    #[error("No metrics found for given timestamp, for validators in given range")]
    ValidatorOutOfRange {},

    #[error("Timestamp not found")]
    InvalidTimestamp {},
}
