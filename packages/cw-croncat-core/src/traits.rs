use crate::types::{Boundary, SlotType};
use cosmwasm_std::{Addr, Coin, Env};
use cw20::Cw20CoinVerified;

pub trait GenericBalances {
    fn add_tokens(&mut self, balance: &Vec<Coin>);
    fn add_cw20tokens(&mut self, token: &Cw20CoinVerified);
    fn minus_tokens(&mut self, balance: &Vec<Coin>);
    fn minus_cw20tokens(&mut self, token: &Cw20CoinVerified);
}

pub trait Intervals {
    fn next(&self, env: Env, boundary: Boundary) -> (u64, SlotType);
    fn is_valid(&self) -> bool;
}

pub trait TaskHash {
    fn to_hash(&self) -> String;
    fn to_hash_vec(&self) -> Vec<u8>;
    fn is_valid_msg(&self, self_addr: &Addr, sender: &Addr, owner_id: &Addr) -> bool;
    fn to_gas_total(&self) -> u64;
}
