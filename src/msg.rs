use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub staking_token: String,
    pub unbond_period: u64,
    pub activity_interval: u64,
    pub penalty_percentage: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Deposit (stake) all the amount of tokens sent within the message
    Receive(Cw20ReceiveMsg),
    /// Withdraw the given amount of staked tokens and send them
    /// to the message sender (after applying penalty if necessary)
    Unbond { amount: Uint128 },
    /// Claim the accrued rewards
    Claim,
    /// Keep Alive signal to indicate that Staker is still active
    KeepAlive,
    /// Claim staked tokens for inactive users
    DeadmanDelay { addr: String },
    /// Update Config
    UpdateConfig {
        staking_period: Option<u64>,
        activity_interval: Option<u64>,
        penalty_percentage: Option<u64>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Returns the current config
    Config,
    ///
    State,
    /// Returns the stake infos of the given Staker
    StakerInfo { staker: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    Bond { backup_addr: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub staking_token: String,
    pub unbond_period: u64,
    pub activity_interval: u64,
    pub penalty_percentage: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateResponse {
    pub total_bound_amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StakerInfoResponse {
    pub staker: String,
    pub time_until: u64,
    pub backup_addr: String,
    pub amount: Uint128,
    pub last_time_active: u64,
}