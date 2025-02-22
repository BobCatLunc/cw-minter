use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const CW20_ADDRESS: Item<Addr> = Item::new("cw20_address");
pub const ADMIN: Item<Addr> = Item::new("admin");