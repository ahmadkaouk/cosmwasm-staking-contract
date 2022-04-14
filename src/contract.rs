#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo,
    Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg, StakerInfoResponse,
    StateResponse,
};
use crate::state::{Config, StakeInfo, State, CONFIG, STAKE, STATE};

type Result = std::result::Result<Response, ContractError>;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:my-first-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(deps: DepsMut, _env: Env, info: MessageInfo, msg: InstantiateMsg) -> Result {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    CONFIG.save(
        deps.storage,
        &Config {
            owner: info.sender,
            staking_token: deps.api.addr_validate(&msg.staking_token)?,
            staking_period: msg.unbond_period,
            activity_interval: msg.activity_interval,
            penalty_percentage: msg.penalty_percentage,
        },
    )?;

    STATE.save(
        deps.storage,
        &State {
            total_bond_amount: Uint128::zero(),
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> Result {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::Unbond { amount } => withdraw(deps, env, info, amount),
        ExecuteMsg::Claim {} => claim_rewards(deps, env, info),
        ExecuteMsg::UpdateConfig {
            staking_period,
            activity_interval,
            penalty_percentage,
        } => update_config(
            deps,
            info,
            staking_period,
            activity_interval,
            penalty_percentage,
        ),
        ExecuteMsg::KeepAlive => keep_alive(deps, env, info),
        ExecuteMsg::DeadmanDelay { addr } => deadman_delay(deps, env, info, addr),
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result {
    let config = CONFIG.load(deps.storage)?;

    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::Bond { backup_addr }) => {
            // only staking token contract can execute this message
            if config.staking_token != info.sender {
                return Err(ContractError::Unauthorized {});
            }

            let cw20_sender = deps.api.addr_validate(&cw20_msg.sender)?;
            let backup_addr = deps.api.addr_validate(&backup_addr)?;
            bond(deps, env, cw20_sender, cw20_msg.amount, backup_addr)
        }
        Err(_) => Err(ContractError::Std(StdError::generic_err(
            "data should be given",
        ))),
    }
}

fn bond(deps: DepsMut, env: Env, sender: Addr, amount: Uint128, backup_addr: Addr) -> Result {
    let config = CONFIG.load(deps.storage)?;

    if amount.is_zero() {
        return Err(ContractError::InvalidAmount);
    }

    let mut new_amount = amount;
    // update the sender's stake
    let _ = STAKE.update(deps.storage, &sender, |stake| -> StdResult<_> {
        if let Some(stake) = stake {
            new_amount += stake.amount;
        }
        Ok(StakeInfo {
            amount: new_amount,
            backup_addr,
            time_until: env.block.time.seconds() + config.activity_interval,
            last_time_active: env.block.time.seconds(),
        })
    })?;

    // update total bond amount
    STATE
        .update::<_, StdError>(deps.storage, |state| {
            Ok(State {
                total_bond_amount: state.total_bond_amount + amount,
            })
        })
        .expect("error updating state");

    Ok(Response::new().add_attributes(vec![
        ("action", "bond"),
        ("owner", sender.as_str()),
        ("amount", amount.to_string().as_str()),
        ("new_amount", new_amount.to_string().as_str()),
    ]))
}

fn withdraw(deps: DepsMut, env: Env, info: MessageInfo, amount: Uint128) -> Result {
    let config = CONFIG.load(deps.storage)?;
    if amount.is_zero() {
        return Err(ContractError::InvalidAmount);
    }

    let mut stake_info = STAKE.load(deps.storage, &info.sender)?;
    if let Err(e) = stake_info.amount.checked_sub(amount) {
        return Err(ContractError::SubstructionOverflow(e.to_string()));
    }
    // update last time active
    stake_info.last_time_active = env.block.time.seconds();
    STAKE.save(deps.storage, &info.sender, &stake_info)?;

    // update total bond amount
    STATE
        .update::<_, StdError>(deps.storage, |state| {
            Ok(State {
                total_bond_amount: state.total_bond_amount.checked_sub(amount).unwrap(),
            })
        })
        .expect("error updating state");

    if stake_info.amount < amount {
        return Err(ContractError::InsufficientFunds);
    }
    // Check if penalty is applied
    if stake_info.time_until > env.block.time.seconds() {
        let penalty = amount * Decimal::percent(config.penalty_percentage);
        if let Err(e) = amount.checked_sub(penalty) {
            return Err(ContractError::SubstructionOverflow(e.to_string()));
        }

        // Send penalty to admin (owner of the contract)
        let transfer = Cw20ExecuteMsg::Transfer {
            recipient: info.sender.clone().into(),
            amount: penalty,
        };

        Ok(Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.staking_token.to_string(),
                msg: to_binary(&transfer)?,
                funds: vec![],
            }))
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.staking_token.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: info.sender.clone().into(),
                    amount,
                })?,
                funds: vec![],
            }))
            .add_attributes(vec![
                ("action", "withdraw"),
                ("owner", info.sender.as_str()),
                ("withdrawed_amount", amount.to_string().as_str()),
            ]))
    } else {
        Ok(Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.staking_token.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: info.sender.clone().into(),
                    amount,
                })?,
                funds: vec![],
            }))
            .add_attributes(vec![
                ("action", "withdraw"),
                ("owner", info.sender.as_str()),
                ("withdrawed_amount", amount.to_string().as_str()),
            ]))
    }
}

fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    staking_period: Option<u64>,
    activity_interval: Option<u64>,
    penalty_percentage: Option<u64>,
) -> Result {
    let mut config = CONFIG.load(deps.storage)?;
    // Only the owner of the contract can change the configuration
    if info.sender != config.owner {
        return Err(ContractError::PermissionDenied(info.sender.into_string()));
    }
    config.staking_period = staking_period.unwrap_or(config.staking_period);
    config.activity_interval = activity_interval.unwrap_or(config.activity_interval);
    config.penalty_percentage = penalty_percentage.unwrap_or(config.penalty_percentage);

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

fn keep_alive(deps: DepsMut, env: Env, info: MessageInfo) -> Result {
    let _ = STAKE.update(deps.storage, &info.sender, |stake| -> StdResult<_> {
        if let Some(stake) = stake {
            Ok(StakeInfo {
                last_time_active: env.block.time.seconds(),
                ..stake
            })
        } else {
            Err(StdError::generic_err("Addr not found"))
        }
    })?;
    Ok(Response::default())
}

fn deadman_delay(deps: DepsMut, env: Env, info: MessageInfo, addr: String) -> Result {
    let staker_addr = deps.api.addr_validate(&addr)?;
    let config = CONFIG.load(deps.storage)?;
    let stake_info = STAKE.load(deps.storage, &staker_addr)?;

    // Only backup addr Owner is allowed to execute this actions
    if stake_info.backup_addr != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    // Allowed only if user is not active since activity_interval
    if stake_info.last_time_active + config.activity_interval > env.block.time.seconds() {
        return Err(ContractError::Unauthorized {});
    }

    // Set inactive Staker amount to zero
    STAKE.update(deps.storage, &staker_addr, |stake| {
        if let Some(stake) = stake {
            Ok(StakeInfo {
                amount: Uint128::zero(),
                ..stake
            })
        } else {
            Err(StdError::generic_err("Addr not found"))
        }
    })?;

    // update total bond amount
    STATE
        .update::<_, StdError>(deps.storage, |state| {
            Ok(State {
                total_bond_amount: state
                    .total_bond_amount
                    .checked_sub(stake_info.amount)
                    .unwrap(),
            })
        })
        .expect("error updating state");

    // Send amount to the backup addr
    let transfer = Cw20ExecuteMsg::Transfer {
        recipient: stake_info.backup_addr.to_string(),
        amount: stake_info.amount,
    };

    Ok(
        Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.staking_token.to_string(),
            msg: to_binary(&transfer)?,
            funds: vec![],
        })),
    )
}

fn claim_rewards(deps: DepsMut, env: Env, info: MessageInfo) -> Result {
    todo!()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::State {} => to_binary(&query_state(deps)?),
        QueryMsg::StakerInfo { staker } => {
            to_binary(&query_staker_info(deps, deps.api.addr_validate(&staker)?)?)
        }
    }
}

fn query_staker_info(deps: Deps, staker: Addr) -> StdResult<StakerInfoResponse> {
    let stake_info = STAKE.load(deps.storage, &staker)?;
    Ok(StakerInfoResponse {
        staker: staker.to_string(),
        amount: stake_info.amount,
        backup_addr: stake_info.backup_addr.to_string(),
        last_time_active: stake_info.last_time_active,
        time_until: stake_info.time_until,
    })
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: config.owner.to_string(),
        activity_interval: config.activity_interval,
        penalty_percentage: config.penalty_percentage,
        staking_token: config.staking_token.to_string(),
        unbond_period: config.staking_period,
    })
}

fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(StateResponse {
        total_bound_amount: state.total_bond_amount,
    })
}

#[cfg(test)]
mod tests {
    use crate::msg::ConfigResponse;

    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            staking_token: "luna".to_string(),
            activity_interval: 3600,
            penalty_percentage: 2,
            unbond_period: 7200,
        };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!("luna", value.staking_token);
        assert_eq!(3600, value.activity_interval);
        assert_eq!(2, value.penalty_percentage);
        assert_eq!(7200, value.unbond_period);
    }

    #[test]
    fn bond() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        let msg = InstantiateMsg {
            staking_token: "luna".to_string(),
            activity_interval: 3600,
            penalty_percentage: 2,
            unbond_period: 7200,
        };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // TODO continue the test
    }
}
