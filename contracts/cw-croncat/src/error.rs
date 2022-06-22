use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("An unknown reply ID was received.")]
    UnknownReplyID {},

    #[error("No coin balance found")]
    EmptyBalance {},

    #[error("No task found by hash")]
    NoTaskFound {},

    #[error("Only accepts tokens in the cw20_whitelist")]
    NotInWhitelist {},

    #[error("Agent not registered")]
    AgentNotRegistered {},

    #[error("{val:?} is paused")]
    ContractPaused { val: String },

    #[error("Not a registered agent")]
    AgentUnregistered {},

    #[error("Insufficient funds")]
    NotEnoughFunds {},

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
