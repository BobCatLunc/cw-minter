use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub cw20_address: String,
    pub initial_uluna_burned: Option<Uint128>,
    pub initial_tokens_minted: Option<Uint128>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    SetCw20Address { address: String },
    Mint {},
    UpdateMinter { new_minter: String },
    SetBurnThreshold { threshold: Uint128 },
    SetMaxMintRatio { max_ratio: Uint128 },
    SetPaused { paused: bool }, // New message to pause/resume minting
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetCw20Address {},
    GetConfig {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Cw20AddressResponse {
    pub address: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub total_uluna_burned: Uint128,
    pub total_tokens_minted: Uint128,
    pub current_mint_ratio: Uint128,
    pub max_mint_ratio: Uint128,
}
