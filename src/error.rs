use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("Permission Denied: {0} does not have the permissions to change contract config")]
    PermissionDenied(String),
    #[error("Invalid amount, amount should be greater than zero")]
    InvalidAmount,
    #[error("Unauthorized")]
    Unauthorized {},
}
