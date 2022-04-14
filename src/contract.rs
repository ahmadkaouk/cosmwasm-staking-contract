#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint128,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;

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
            unbond_period: msg.unbond_period,
            activity_interval: msg.activity_interval,
            penalty_percentage: msg.penalty_percentage,
        },
    )?;

    STATE.save(
        deps.storage,
        &State {
            total_bond_amount: Uint128::zero(),
        },
    );

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> Result {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::Unbond { amount } => unbond(deps, env, info, amount),
        ExecuteMsg::Claim {} => withdraw(deps, env, info),
        ExecuteMsg::UpdateConfig {
            unbond_period,
            activity_interval,
            penalty_percentage,
        } => update_config(
            deps,
            env,
            info,
            unbond_period,
            activity_interval,
            penalty_percentage,
        ),
        ExecuteMsg::KeepAlive => keep_alive(),
        ExecuteMsg::DeadmanDelay { addr } => deadman_delay(),
    }
}

fn keep_alive() -> Result {
    todo!()
}

fn deadman_delay() -> Result {
    todo!()
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
            bond(
                deps,
                env,
                cw20_sender,
                cw20_msg.amount,
                deps.api.addr_validate(&backup_addr)?,
            )
        }
        Err(_) => Err(ContractError::Std("data should be given")),
    }
}

fn bond(deps: DepsMut, env: Env, sender: Addr, amount: Uint128, backup_addr: Addr) -> Result {
    let config = CONFIG.load(deps.storage)?;

    if amount.is_zero() {
        return Err(ContractError::InvalidAmount);
    }

    // update the sender's stake
    let new_stake = STAKE.update(deps.storage, &sender, |stake| -> StdResult<_> {
        let new_amount = amount;
        if let Some(stake) = stake {
            amount += stake.amount;
        }
        Ok(StakeInfo {
            amount,
            backup_addr,
            time_until: env.block.time.seconds() + config.activity_interval,
            last_time_active: env.block.time.seconds(),
        })
    })?;

    Ok(Response::new().add_attributes(vec![
        ("action", "bond"),
        ("owner", sender.as_str()),
        ("amount", amount.to_string().as_str()),
    ]))
}

fn update_config(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    unbond_period: Option<u64>,
    activity_interval: Option<u64>,
    penalty_percentage: Option<u64>,
) -> Result {
    let mut config = CONFIG.load(deps.storage)?;
    // Only the owner of the contract can change the configuration
    if info.sender != config.owner {
        return Err(ContractError::PermissionDenied(info.sender.into_string()));
    }
    config.unbond_period = unbond_period.unwrap_or(config.unbond_period);
    config.activity_interval = activity_interval.unwrap_or(config.activity_interval);
    config.penalty_percentage = penalty_percentage.unwrap_or(config.penalty_percentage);

    CONFIG.save(deps.storage, &config);

    Ok(Response::new().add_attribute("action", "update_config"))
}

fn withdraw(deps: DepsMut, env: Env, info: MessageInfo) -> Result {
    todo!()
}

fn unbond(deps: DepsMut, env: Env, info: MessageInfo, amount: Uint128) -> Result {
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
    todo!()
}

fn query_state(deps: Deps) -> StdResult<StateResponse> {
    todo!()
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    todo!()
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
    }
}
