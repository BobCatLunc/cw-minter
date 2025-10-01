use cosmwasm_std::{
    entry_point, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128, BankMsg, Binary,
    to_json_binary, CosmosMsg, WasmMsg, StdError,
};
use cw20::Cw20ExecuteMsg;
use serde_json;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, QueryMsg, Cw20AddressResponse, ConfigResponse,
};
use crate::state::{Config, CONFIG};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let addr = deps.api.addr_validate(&msg.cw20_address)?;
    let config = Config {
        cw20_address: addr.clone(),
        admin: info.sender.clone(),
        total_uluna_burned: msg.initial_uluna_burned.unwrap_or(Uint128::zero()),
        total_tokens_minted: msg.initial_tokens_minted.unwrap_or(Uint128::zero()),
        burn_threshold: Uint128::new(1_000_000_000_000), // Default 1T
        max_mint_ratio: Uint128::zero(), // Default 0 (no limit)
        paused: false, // Default to not paused
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("admin", info.sender.to_string())
        .add_attribute("cw20_address", msg.cw20_address)
        .add_attribute("initial_uluna_burned", config.total_uluna_burned.to_string())
        .add_attribute("initial_tokens_minted", config.total_tokens_minted.to_string())
        .add_attribute("burn_threshold", config.burn_threshold.to_string())
        .add_attribute("max_mint_ratio", config.max_mint_ratio.to_string()))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::SetCw20Address { address } => try_set_address(deps, info, address),
        ExecuteMsg::Mint {} => {
            if let Some(uluna_amount) = info.funds.iter().find(|c| c.denom == "uluna").map(|c| c.amount) {
                try_mint(deps, env, info, uluna_amount)
            } else {
                Err(StdError::generic_err("No uluna sent for minting"))
            }
        },
        ExecuteMsg::UpdateMinter { new_minter } => try_update_minter(deps, info, new_minter),
        ExecuteMsg::SetBurnThreshold { threshold } => try_set_burn_threshold(deps, info, threshold),
        ExecuteMsg::SetMaxMintRatio { max_ratio } => try_set_max_mint_ratio(deps, info, max_ratio),
        ExecuteMsg::SetPaused { paused } => try_set_paused(deps, info, paused),
    }
}

fn try_set_address(deps: DepsMut, info: MessageInfo, address: String) -> StdResult<Response> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(StdError::generic_err("Only admin can set CW20 address"));
    }

    let addr = deps.api.addr_validate(&address)?;
    config.cw20_address = addr.clone();
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "set_cw20_address"))
}

fn try_mint(deps: DepsMut, _env: Env, info: MessageInfo, amount: Uint128) -> StdResult<Response> {
    let mut config = CONFIG.load(deps.storage)?;
    if config.paused {
        return Err(StdError::generic_err("Minting is currently paused"));
    }

    let burn_address = deps.api.addr_validate("terra1sk06e3dyexuq4shw77y3dsv480xv42mq73anxu")?;

    let calculated_ratio = if config.total_uluna_burned < Uint128::new(5_000_000_000_000) {
        Uint128::one()
    } else {
        ((config.total_uluna_burned - Uint128::new(5_000_000_000_000)) / config.burn_threshold) + Uint128::new(2)
    };

    let mint_ratio = if config.max_mint_ratio.is_zero() || calculated_ratio <= config.max_mint_ratio {
        calculated_ratio
    } else {
        config.max_mint_ratio
    };

    let mint_amount = amount
        .checked_div(mint_ratio)
        .map_err(|_| StdError::generic_err("Division by zero in mint amount calculation"))?;

    config.total_uluna_burned += amount;
    config.total_tokens_minted += mint_amount;
    CONFIG.save(deps.storage, &config)?;

    let response = Response::new()
        .add_message(BankMsg::Send {
            to_address: burn_address.to_string(),
            amount: vec![cosmwasm_std::Coin {
                denom: "uluna".to_string(),
                amount,
            }],
        })
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.cw20_address.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Mint {
                recipient: info.sender.to_string(),
                amount: mint_amount,
            })?,
            funds: vec![],
        }))
        .add_attribute("action", "mint_cw20")
        .add_attribute("uluna_amount", amount.to_string())
        .add_attribute("mint_amount", mint_amount.to_string())
        .add_attribute("mint_ratio", mint_ratio.to_string())
        .add_attribute("max_mint_ratio", config.max_mint_ratio.to_string())
        .add_attribute("total_uluna_burned", config.total_uluna_burned.to_string())
        .add_attribute("total_tokens_minted", config.total_tokens_minted.to_string());

    Ok(response)
}

fn try_update_minter(deps: DepsMut, info: MessageInfo, new_minter: String) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(StdError::generic_err("Only admin can update minter"));
    }

    let update_minter_msg = serde_json::to_vec(&serde_json::json!({
        "update_minter": {
            "new_minter": new_minter.clone()
        }
    }))
    .map_err(|e| StdError::generic_err(format!("Failed to serialize update_minter message: {}", e)))?;

    let response = Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.cw20_address.to_string(),
            msg: Binary(update_minter_msg),
            funds: vec![],
        }))
        .add_attribute("action", "update_minter")
        .add_attribute("new_minter", new_minter);

    Ok(response)
}

fn try_set_burn_threshold(deps: DepsMut, info: MessageInfo, threshold: Uint128) -> StdResult<Response> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(StdError::generic_err("Only admin can set burn threshold"));
    }

    if threshold.is_zero() {
        return Err(StdError::generic_err("Burn threshold cannot be zero"));
    }

    config.burn_threshold = threshold;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "set_burn_threshold")
        .add_attribute("threshold", threshold.to_string()))
}

fn try_set_max_mint_ratio(deps: DepsMut, info: MessageInfo, max_ratio: Uint128) -> StdResult<Response> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(StdError::generic_err("Only admin can set max mint ratio"));
    }

    config.max_mint_ratio = max_ratio;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "set_max_mint_ratio")
        .add_attribute("max_ratio", max_ratio.to_string()))
}

fn try_set_paused(deps: DepsMut, info: MessageInfo, paused: bool) -> StdResult<Response> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(StdError::generic_err("Only admin can set pause status"));
    }

    config.paused = paused;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "set_paused")
        .add_attribute("paused", paused.to_string()))
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCw20Address {} => to_json_binary(&Cw20AddressResponse {
            address: CONFIG.load(deps.storage)?.cw20_address,
        }),
        QueryMsg::GetConfig {} => {
            let config = CONFIG.load(deps.storage)?;

            let calculated_ratio = if config.total_uluna_burned < Uint128::new(5_000_000_000_000) {
                Uint128::one()
            } else {
                ((config.total_uluna_burned - Uint128::new(5_000_000_000_000)) / config.burn_threshold) + Uint128::new(2)
            };

            let current_mint_ratio = if config.max_mint_ratio.is_zero() || calculated_ratio <= config.max_mint_ratio {
                calculated_ratio
            } else {
                config.max_mint_ratio
            };

            to_json_binary(&ConfigResponse {
                total_uluna_burned: config.total_uluna_burned,
                total_tokens_minted: config.total_tokens_minted,
                current_mint_ratio,
                max_mint_ratio: config.max_mint_ratio,
            })
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_json, Addr};

    const CW20_ADDR: &str = "terra1cw20address";

    #[test]
    fn test_instantiate_default() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("admin", &[]);

        let msg = InstantiateMsg {
            cw20_address: CW20_ADDR.to_string(),
            initial_uluna_burned: None,
            initial_tokens_minted: None,
        };
        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        assert_eq!(res.attributes.len(), 7);
        assert_eq!(res.attributes[0], ("action", "instantiate"));
        assert_eq!(res.attributes[1], ("admin", "admin"));
        assert_eq!(res.attributes[2], ("cw20_address", CW20_ADDR));
        assert_eq!(res.attributes[3], ("initial_uluna_burned", "0"));
        assert_eq!(res.attributes[4], ("initial_tokens_minted", "0"));
        assert_eq!(res.attributes[5], ("burn_threshold", "1000000000000"));
        assert_eq!(res.attributes[6], ("max_mint_ratio", "0"));

        let config_res: ConfigResponse = from_json(&query(
            deps.as_ref(),
            env,
            QueryMsg::GetConfig {},
        ).unwrap()).unwrap();
        assert_eq!(config_res.total_uluna_burned, Uint128::zero());
        assert_eq!(config_res.total_tokens_minted, Uint128::zero());
        assert_eq!(config_res.current_mint_ratio, Uint128::one());
        assert_eq!(config_res.max_mint_ratio, Uint128::zero());
    }

    #[test]
    fn test_instantiate_with_initial_values() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("admin", &[]);

        let initial_burn = Uint128::new(10_000_000_000_000);
        let initial_minted = Uint128::new(1_000_000_000_000);
        let msg = InstantiateMsg {
            cw20_address: CW20_ADDR.to_string(),
            initial_uluna_burned: Some(initial_burn),
            initial_tokens_minted: Some(initial_minted),
        };
        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        assert_eq!(res.attributes.len(), 7);
        assert_eq!(res.attributes[3], ("initial_uluna_burned", "10000000000000"));
        assert_eq!(res.attributes[4], ("initial_tokens_minted", "1000000000000"));

        let config_res: ConfigResponse = from_json(&query(
            deps.as_ref(),
            env,
            QueryMsg::GetConfig {},
        ).unwrap()).unwrap();
        assert_eq!(config_res.total_uluna_burned, initial_burn);
        assert_eq!(config_res.total_tokens_minted, initial_minted);
        assert_eq!(config_res.current_mint_ratio, Uint128::new(7));
        assert_eq!(config_res.max_mint_ratio, Uint128::zero());
    }

    #[test]
    fn test_mint_ratio_progression() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let admin_info = mock_info("admin", &[]);

        let msg = InstantiateMsg {
            cw20_address: CW20_ADDR.to_string(),
            initial_uluna_burned: None,
            initial_tokens_minted: None,
        };
        instantiate(deps.as_mut(), env.clone(), admin_info.clone(), msg).unwrap();

        let big_burn = Uint128::new(5_000_000_000_000);
        let user_info = mock_info("user", &coins(big_burn.u128(), "uluna"));
        let res = execute(deps.as_mut(), env.clone(), user_info.clone(), ExecuteMsg::Mint {}).unwrap();
        assert_eq!(res.attributes[2], ("mint_amount", "5000000000000"));
        assert_eq!(res.attributes[3], ("mint_ratio", "1"));

        let stats_res: ConfigResponse = from_json(&query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::GetConfig {},
        ).unwrap()).unwrap();
        assert_eq!(stats_res.total_uluna_burned, Uint128::new(5_000_000_000_000));
        assert_eq!(stats_res.total_tokens_minted, Uint128::new(5_000_000_000_000));
        assert_eq!(stats_res.current_mint_ratio, Uint128::new(2));
        assert_eq!(stats_res.max_mint_ratio, Uint128::zero());

        let next_burn = Uint128::new(1_000_000_000_000);
        let user_info = mock_info("user", &coins(next_burn.u128(), "uluna"));
        let res = execute(deps.as_mut(), env.clone(), user_info.clone(), ExecuteMsg::Mint {}).unwrap();
        assert_eq!(res.attributes[2], ("mint_amount", "500000000000"));
        assert_eq!(res.attributes[3], ("mint_ratio", "2"));

        let stats_res: ConfigResponse = from_json(&query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::GetConfig {},
        ).unwrap()).unwrap();
        assert_eq!(stats_res.total_uluna_burned, Uint128::new(6_000_000_000_000));
        assert_eq!(stats_res.total_tokens_minted, Uint128::new(5_500_000_000_000));
        assert_eq!(stats_res.current_mint_ratio, Uint128::new(3));
        assert_eq!(stats_res.max_mint_ratio, Uint128::zero());

        let res = execute(deps.as_mut(), env.clone(), user_info.clone(), ExecuteMsg::Mint {}).unwrap();
        assert_eq!(res.attributes[2], ("mint_amount", "333333333333"));
        assert_eq!(res.attributes[3], ("mint_ratio", "3"));
    }

    #[test]
    fn test_admin_only_functions() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let admin_info = mock_info("admin", &[]);

        let msg = InstantiateMsg {
            cw20_address: CW20_ADDR.to_string(),
            initial_uluna_burned: None,
            initial_tokens_minted: None,
        };
        instantiate(deps.as_mut(), env.clone(), admin_info.clone(), msg).unwrap();

        let non_admin_info = mock_info("non_admin", &[]);
        let res = execute(
            deps.as_mut(),
            env.clone(),
            non_admin_info.clone(),
            ExecuteMsg::SetCw20Address { address: "new_addr".to_string() },
        );
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "Generic error: Only admin can set CW20 address"
        );

        let res = execute(
            deps.as_mut(),
            env.clone(),
            non_admin_info.clone(),
            ExecuteMsg::UpdateMinter { new_minter: "new_minter".to_string() },
        );
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "Generic error: Only admin can update minter"
        );

        let res = execute(
            deps.as_mut(),
            env.clone(),
            admin_info.clone(),
            ExecuteMsg::SetCw20Address { address: "new_addr".to_string() },
        ).unwrap();
        assert_eq!(res.attributes[0], ("action", "set_cw20_address"));

        let res = execute(
            deps.as_mut(),
            env.clone(),
            admin_info.clone(),
            ExecuteMsg::UpdateMinter { new_minter: "new_minter".to_string() },
        ).unwrap();
        assert_eq!(res.attributes[0], ("action", "update_minter"));

        let res = execute(
            deps.as_mut(),
            env.clone(),
            non_admin_info.clone(),
            ExecuteMsg::SetBurnThreshold { threshold: Uint128::new(2_000_000_000_000) },
        );
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "Generic error: Only admin can set burn threshold"
        );

        let res = execute(
            deps.as_mut(),
            env.clone(),
            admin_info.clone(),
            ExecuteMsg::SetBurnThreshold { threshold: Uint128::new(2_000_000_000_000) },
        ).unwrap();
        assert_eq!(res.attributes[0], ("action", "set_burn_threshold"));
        assert_eq!(res.attributes[1], ("threshold", "2000000000000"));

        let res = execute(
            deps.as_mut(),
            env.clone(),
            non_admin_info.clone(),
            ExecuteMsg::SetMaxMintRatio { max_ratio: Uint128::new(5) },
        );
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "Generic error: Only admin can set max mint ratio"
        );

        let res = execute(
            deps.as_mut(),
            env.clone(),
            admin_info.clone(),
            ExecuteMsg::SetMaxMintRatio { max_ratio: Uint128::new(5) },
        ).unwrap();
        assert_eq!(res.attributes[0], ("action", "set_max_mint_ratio"));
        assert_eq!(res.attributes[1], ("max_ratio", "5"));

        let res = execute(
            deps.as_mut(),
            env.clone(),
            non_admin_info.clone(),
            ExecuteMsg::SetPaused { paused: true },
        );
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "Generic error: Only admin can set pause status"
        );

        let res = execute(
            deps.as_mut(), 
            env.clone(),
            admin_info.clone(),
            ExecuteMsg::SetPaused { paused: true },
        ).unwrap();
        assert_eq!(res.attributes[0], ("action", "set_paused"));
        assert_eq!(res.attributes[1], ("paused", "true"));

        let user_info = mock_info("user", &coins(1_000_000_000_000u128, "uluna"));
        let res = execute(deps.as_mut(), env.clone(), user_info.clone(), ExecuteMsg::Mint {});
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "Generic error: Minting is currently paused"
        );

        let res = execute(
            deps.as_mut(),
            env,
            admin_info,
            ExecuteMsg::SetPaused { paused: false },
        ).unwrap();
        assert_eq!(res.attributes[0], ("action", "set_paused"));
        assert_eq!(res.attributes[1], ("paused", "false"));
    }

    #[test]
    fn test_mint_with_max_ratio() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let admin_info = mock_info("admin", &[]);

        let msg = InstantiateMsg {
            cw20_address: CW20_ADDR.to_string(),
            initial_uluna_burned: None,
            initial_tokens_minted: None,
        };
        instantiate(deps.as_mut(), env.clone(), admin_info.clone(), msg).unwrap();

        let res = execute(
            deps.as_mut(),
            env.clone(),
            admin_info.clone(),
            ExecuteMsg::SetMaxMintRatio { max_ratio: Uint128::new(3) },
        ).unwrap();
        assert_eq!(res.attributes[0], ("action", "set_max_mint_ratio"));
        assert_eq!(res.attributes[1], ("max_ratio", "3"));

        let big_burn = Uint128::new(5_000_000_000_000);
        let user_info = mock_info("user", &coins(big_burn.u128(), "uluna"));
        let res = execute(deps.as_mut(), env.clone(), user_info.clone(), ExecuteMsg::Mint {}).unwrap();
        assert_eq!(res.attributes[2], ("mint_amount", "5000000000000"));
        assert_eq!(res.attributes[3], ("mint_ratio", "1"));

        let next_burn = Uint128::new(1_000_000_000_000);
        let user_info = mock_info("user", &coins(next_burn.u128(), "uluna"));
        let res = execute(deps.as_mut(), env.clone(), user_info.clone(), ExecuteMsg::Mint {}).unwrap();
        assert_eq!(res.attributes[2], ("mint_amount", "500000000000"));
        assert_eq!(res.attributes[3], ("mint_ratio", "2"));

        let res = execute(deps.as_mut(), env.clone(), user_info.clone(), ExecuteMsg::Mint {}).unwrap();
        assert_eq!(res.attributes[2], ("mint_amount", "333333333333"));
        assert_eq!(res.attributes[3], ("mint_ratio", "3"));

        let res = execute(deps.as_mut(), env.clone(), user_info.clone(), ExecuteMsg::Mint {}).unwrap();
        assert_eq!(res.attributes[2], ("mint_amount", "333333333333"));
        assert_eq!(res.attributes[3], ("mint_ratio", "3"));

        let config_res: ConfigResponse = from_json(&query(
            deps.as_ref(),
            env,
            QueryMsg::GetConfig {},
        ).unwrap()).unwrap();
        assert_eq!(config_res.total_uluna_burned, Uint128::new(8_000_000_000_000));
        assert_eq!(config_res.total_tokens_minted, Uint128::new(6_166_666_666_666));
        assert_eq!(config_res.current_mint_ratio, Uint128::new(3));
        assert_eq!(config_res.max_mint_ratio, Uint128::new(3));
    }

    #[test]
    fn test_burn_threshold_update() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let admin_info = mock_info("admin", &[]);

        let msg = InstantiateMsg {
            cw20_address: CW20_ADDR.to_string(),
            initial_uluna_burned: Some(Uint128::new(5_000_000_000_000)),
            initial_tokens_minted: None,
        };
        instantiate(deps.as_mut(), env.clone(), admin_info.clone(), msg).unwrap();

        let next_burn = Uint128::new(1_000_000_000_000);
        let user_info = mock_info("user", &coins(next_burn.u128(), "uluna"));
        let res = execute(deps.as_mut(), env.clone(), user_info.clone(), ExecuteMsg::Mint {}).unwrap();
        assert_eq!(res.attributes[2], ("mint_amount", "500000000000"));
        assert_eq!(res.attributes[3], ("mint_ratio", "2"));

        let res = execute(
            deps.as_mut(),
            env.clone(),
            admin_info.clone(),
            ExecuteMsg::SetBurnThreshold { threshold: Uint128::new(2_000_000_000_000) },
        ).unwrap();
        assert_eq!(res.attributes[0], ("action", "set_burn_threshold"));
        assert_eq!(res.attributes[1], ("threshold", "2000000000000"));

        let res = execute(deps.as_mut(), env.clone(), user_info.clone(), ExecuteMsg::Mint {}).unwrap();
        assert_eq!(res.attributes[2], ("mint_amount", "500000000000"));
        assert_eq!(res.attributes[3], ("mint_ratio", "2"));

        let bigger_burn = Uint128::new(2_000_000_000_000);
        let user_info = mock_info("user", &coins(bigger_burn.u128(), "uluna"));
        let res = execute(deps.as_mut(), env.clone(), user_info.clone(), ExecuteMsg::Mint {}).unwrap();
        assert_eq!(res.attributes[2], ("mint_amount", "666666666666"));
        assert_eq!(res.attributes[3], ("mint_ratio", "3"));

        let config_res: ConfigResponse = from_json(&query(
            deps.as_ref(),
            env,
            QueryMsg::GetConfig {},
        ).unwrap()).unwrap();
        assert_eq!(config_res.total_uluna_burned, Uint128::new(9_000_000_000_000));
        assert_eq!(config_res.total_tokens_minted, Uint128::new(1_666_666_666_666));
        assert_eq!(config_res.current_mint_ratio, Uint128::new(4));
        assert_eq!(config_res.max_mint_ratio, Uint128::zero());
    }

    #[test]
    fn test_queries() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("admin", &[]);

        let msg = InstantiateMsg {
            cw20_address: CW20_ADDR.to_string(),
            initial_uluna_burned: Some(Uint128::new(7_500_000_000_000)),
            initial_tokens_minted: Some(Uint128::new(1_000_000_000_000)),
        };
        instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let addr_res: Cw20AddressResponse = from_json(&query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::GetCw20Address {},
        ).unwrap()).unwrap();
        assert_eq!(addr_res.address, Addr::unchecked(CW20_ADDR));

        let config_res: ConfigResponse = from_json(&query(
            deps.as_ref(),
            env,
            QueryMsg::GetConfig {},
        ).unwrap()).unwrap();
        assert_eq!(config_res.total_uluna_burned, Uint128::new(7_500_000_000_000));
        assert_eq!(config_res.total_tokens_minted, Uint128::new(1_000_000_000_000));
        assert_eq!(config_res.current_mint_ratio, Uint128::new(4));
        assert_eq!(config_res.max_mint_ratio, Uint128::zero());
    }
}
