#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn test_set_and_mint() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);

        let msg = InstantiateMsg { cw20_address: "cw20_address".to_string() };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let set_msg = ExecuteMsg::SetCw20Address { address: "new_cw20_address".to_string() };
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), set_msg).unwrap();

        let mint_msg = ExecuteMsg::Mint {};
        let info_with_funds = mock_info("sender", &coins(1000, "uluna"));
        let res = execute(deps.as_mut(), env.clone(), info_with_funds, mint_msg).unwrap();

        assert_eq!(res.messages.len(), 2);  // Ensure we have messages for burn and mint
    }
}