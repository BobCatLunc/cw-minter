use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub cw20_address: Addr,
    pub admin: Addr,
    pub total_uluna_burned: Uint128,
    pub total_tokens_minted: Uint128,
    pub burn_threshold: Uint128,
    pub max_mint_ratio: Uint128,
    pub paused: bool, // New field to track pause status
}

pub const CONFIG: Item<Config> = Item::new("config");
