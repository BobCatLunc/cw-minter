use cosmwasm_std::{entry_point, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128, BankMsg, Binary, to_binary, CosmosMsg, WasmMsg, StdError};
use cw20::{Cw20ExecuteMsg, MinterResponse};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, Cw20AddressResponse};
use crate::state::{CW20_ADDRESS, ADMIN};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let addr = deps.api.addr_validate(&msg.cw20_address)?;
    CW20_ADDRESS.save(deps.storage, &addr)?;
    ADMIN.save(deps.storage, &info.sender)?;
    Ok(Response::new())
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
    }
}

fn try_set_address(deps: DepsMut, info: MessageInfo, address: String) -> StdResult<Response> {
    let admin = ADMIN.load(deps.storage)?;
    if info.sender != admin {
        return Err(StdError::generic_err("Only admin can set CW20 address"));
    }

    let addr = deps.api.addr_validate(&address)?;
    CW20_ADDRESS.save(deps.storage, &addr)?;
    Ok(Response::new().add_attribute("action", "set_cw20_address"))
}

fn try_mint(deps: DepsMut, env: Env, info: MessageInfo, amount: Uint128) -> StdResult<Response> {
    let cw20_address = CW20_ADDRESS.load(deps.storage)?;
    
    let burn_address = deps.api.addr_validate("terra1sk06e3dyexuq4shw77y3dsv480xv42mq73anxu")?;

    let response = Response::new()
        .add_message(BankMsg::Send {
            to_address: burn_address.to_string(),
            amount: vec![cosmwasm_std::Coin {
                denom: "uluna".to_string(),
                amount,
            }],
        })
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cw20_address.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: info.sender.to_string(),
                amount,
            })?,
            funds: vec![],
        }))
        .add_attribute("action", "mint_cw20")
        .add_attribute("amount", amount.to_string())
        .add_attribute("memo", "Burnix Mint n Lunc Burn 1:1");

    Ok(response)
}

fn try_update_minter(deps: DepsMut, info: MessageInfo, new_minter: String) -> StdResult<Response> {
    let admin = ADMIN.load(deps.storage)?;
    if info.sender != admin {
        return Err(StdError::generic_err("Only admin can update minter"));
    }

    let cw20_address = CW20_ADDRESS.load(deps.storage)?;
    
    let update_minter_msg = ExecuteMsg::UpdateMinter {
        new_minter: new_minter.clone(),
    };

    let response = Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cw20_address.to_string(),
            msg: to_binary(&update_minter_msg)?,
            funds: vec![],
        }))
        .add_attribute("action", "update_minter")
        .add_attribute("new_minter", new_minter);

    Ok(response)
}


#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCw20Address {} => to_binary(&Cw20AddressResponse {
            address: CW20_ADDRESS.load(deps.storage)?,
        }),
    }
}