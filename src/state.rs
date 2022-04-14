use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    /// Total number of tokens staked in the contract
    total_bond_amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Addr of the owner (used also to transfer penalties)
    pub owner: Addr,
   /// Contract addr of the token to stake 
    pub staking_token: Addr,
    /// Stake Duration to unbond tokens without penalty
    pub unbond_period: u64,
    /// Time interval to acknowledge that a user is still alive (in seconds)
    pub activity_interval: u64,
    /// The penalty percentage to send to admin account
    pub penalty_percentage: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StakeInfo {
    /// Timestamp at which user can withdraw tokens without penalty
    pub time_until: u64,
    /// Backup address to claim tokens
    pub backup_addr: Addr,
    /// Amount of tokens stacked
    pub amount: Uint128,
    /// Timestamp of last activity of the staker (in seconds)
    pub last_time_active: u64,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const STATE: Item<State> = Item::new("state");
pub const STAKE: Map<&Addr, StakeInfo> = Map::new("stake");
