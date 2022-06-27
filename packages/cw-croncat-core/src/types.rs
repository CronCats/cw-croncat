use cosmwasm_std::{
    Addr, BankMsg, Binary, Coin, CosmosMsg, Empty, Env, GovMsg, IbcMsg, Timestamp, WasmMsg,
};
use cron_schedule::Schedule;
use cw20::Cw20CoinVerified;
use hex::encode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct GenericBalance {
    pub native: Vec<Coin>,
    pub cw20: Vec<Cw20CoinVerified>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum AgentStatus {
    // Default for any new agent, if tasks ratio allows
    Active,

    // Default for any new agent, until more tasks come online
    Pending,

    // More tasks are available, agent must checkin to become active
    Nominated,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Agent {
    // Where rewards get transferred
    pub payable_account_id: Addr,

    // accrued reward balance
    pub balance: GenericBalance,

    // stats
    pub total_tasks_executed: u64,

    // Holds slot number of a missed slot.
    // If other agents see an agent miss a slot, they store the missed slot number.
    // If agent does a task later, this number is reset to zero.
    // Example data: 1633890060000000000 or 0
    pub last_missed_slot: u64,

    // Timestamp of when agent first registered
    // Useful for rewarding agents for their patience while they are pending and operating service
    // Agent will be responsible to constantly monitor when it is their turn to join in active agent set (done as part of agent code loops)
    // Example data: 1633890060000000000 or 0
    pub register_start: Timestamp,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AgentResponse {
    // This field doesn't exist in the Agent struct and is the only one that differs
    pub status: AgentStatus,
    pub payable_account_id: Addr,
    pub balance: GenericBalance,
    pub total_tasks_executed: u64,
    pub last_missed_slot: u64,
    pub register_start: Timestamp,
}

/// Defines the spacing of execution
/// NOTE:S
/// - Block Height Based: Once, Immediate, Block
/// - Timestamp Based: Cron
/// - No Epoch support directly, advised to use block heights instead
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum Interval {
    /// For when this is a non-recurring future scheduled TXN
    Once,

    /// The ugly batch schedule type, in case you need to exceed single TXN gas limits, within fewest block(s)
    Immediate,

    /// Allows timing based on block intervals rather than timestamps
    Block(u64),

    /// Crontab Spec String
    Cron(String),
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, JsonSchema)]
pub enum BoundarySpec {
    /// Represents the block height
    Height(u64),

    /// Represents the block timestamp
    Time(Timestamp),
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, JsonSchema)]
pub struct Boundary {
    ///
    pub start: Option<BoundarySpec>,
    ///
    pub end: Option<BoundarySpec>,
}

#[derive(Debug, PartialEq)]
pub enum SlotType {
    Block,
    Cron,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Rule {
    /// TBD: Interchain query support (See ibc::IbcMsg)
    // pub chain_id: Option<String>,

    /// Account to direct all view calls against
    pub contract_addr: Addr,

    // NOTE: Only allow static pre-defined query msg
    pub msg: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Action<T = Empty> {
    // NOTE: Only allow static pre-defined query msg
    /// Supported CosmosMsgs only!
    pub msg: CosmosMsg<T>,

    /// The gas needed to safely process the execute msg
    pub gas_limit: Option<u64>,
}

/// The response required by all rule queries. Bool is needed for croncat, T allows flexible rule engine
pub type RuleResponse<T> = (bool, T);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Task {
    /// Entity responsible for this task, can change task details
    pub owner_id: Addr,

    /// Scheduling definitions
    pub interval: Interval,
    pub boundary: Boundary,

    /// Defines if this task can continue until balance runs out
    pub stop_on_fail: bool,

    /// NOTE: Only tally native balance here, manager can maintain token/balances outside of tasks
    pub total_deposit: Vec<Coin>,

    /// The cosmos message to call, if time or rules are met
    pub actions: Vec<Action>,
    /// A prioritized list of messages that can be chained decision matrix
    /// required to complete before task action
    /// Rules MUST return the ResolverResponse type
    pub rules: Option<Vec<Rule>>,
    // TODO: funds! should we support funds being attached?
}

impl Task {
    /// Get the hash of a task based on parameters
    pub fn to_hash(&self) -> String {
        let message = format!(
            "{:?}{:?}{:?}{:?}{:?}",
            self.owner_id,
            self.interval,
            self.clone().boundary,
            self.actions,
            self.rules
        );

        let hash = Sha256::digest(message.as_bytes());
        encode(hash)
    }
    /// Get the hash of a task based on parameters
    pub fn to_hash_vec(&self) -> Vec<u8> {
        self.to_hash().into_bytes()
    }
    // /// Returns the base amount required to execute 1 task
    // /// NOTE: this is not the final used amount, just the user-specified amount total needed
    // pub fn task_balance_uses(&self, task: &Task) -> u128 {
    //     task.deposit.0 + (u128::from(task.gas) * self.gas_price) + self.agent_fee
    // }

    /// Validate the task actions only use the supported messages
    pub fn is_valid_msg(&self, self_addr: &Addr, sender: &Addr, owner_id: &Addr) -> bool {
        // TODO: Chagne to default FALSE, once all messages are covered in tests
        let mut valid = true;

        for action in self.actions.iter() {
            match action.clone().msg {
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr,
                    funds: _,
                    msg: _,
                }) => {
                    // TODO: Is there any way sender can be "self" creating a malicious task?
                    // cannot be THIS contract id, unless predecessor is owner of THIS contract
                    if &contract_addr == self_addr && sender != owner_id {
                        valid = false;
                    }
                }
                // TODO: Allow send, as long as coverage of assets is correctly handled
                CosmosMsg::Bank(BankMsg::Send { .. }) => {
                    // Restrict bank msg for time being, so contract doesnt get drained, however could allow an escrow type setup
                    valid = false;
                }
                CosmosMsg::Bank(BankMsg::Burn { .. }) => {
                    // Restrict bank msg for time being, so contract doesnt get drained, however could allow an escrow type setup
                    valid = false;
                }
                CosmosMsg::Gov(GovMsg::Vote { .. }) => {
                    // Restrict bank msg for time being, so contract doesnt get drained, however could allow an escrow type setup
                    valid = false;
                }
                // TODO: Setup better support for IBC
                CosmosMsg::Ibc(IbcMsg::Transfer { .. }) => {
                    // Restrict bank msg for time being, so contract doesnt get drained, however could allow an escrow type setup
                    valid = false;
                }
                // TODO: Check authZ messages
                _ => (),
            }
        }

        valid
    }

    /// Get task gas total
    /// helper for getting total configured gas for this tasks actions
    pub fn to_gas_total(&self) -> u64 {
        let mut gas: u64 = 0;

        // tally all the gases
        for action in self.actions.iter() {
            gas = gas.saturating_add(action.gas_limit.unwrap_or(0));
        }

        gas
    }
}

impl GenericBalance {
    pub fn add_tokens(&mut self, balance: &[Coin]) {
        for token in balance {
            let index = self.native.iter().enumerate().find_map(|(i, exist)| {
                if exist.denom == token.denom {
                    Some(i)
                } else {
                    None
                }
            });
            match index {
                Some(idx) => self.native[idx].amount += token.amount,
                None => self.native.push(token.clone()),
            }
        }
    }

    pub fn add_cw20tokens(&mut self, token: &Cw20CoinVerified) {
        let index = self.cw20.iter().enumerate().find_map(|(i, exist)| {
            if exist.address == token.address {
                Some(i)
            } else {
                None
            }
        });
        match index {
            Some(idx) => self.cw20[idx].amount += token.amount,
            None => self.cw20.push(token.clone()),
        }
    }

    pub fn minus_tokens(&mut self, balance: &[Coin]) {
        for token in balance {
            let index = self.native.iter().enumerate().find_map(|(i, exist)| {
                if exist.denom == token.denom {
                    Some(i)
                } else {
                    None
                }
            });
            if let Some(idx) = index {
                self.native[idx].amount -= token.amount
            }
        }
    }

    pub fn minus_cw20tokens(&mut self, token: &Cw20CoinVerified) {
        let index = self.cw20.iter().enumerate().find_map(|(i, exist)| {
            if exist.address == token.address {
                Some(i)
            } else {
                None
            }
        });
        if let Some(idx) = index {
            self.cw20[idx].amount -= token.amount
        }
    }
}

fn get_next_block_limited(env: Env, boundary: Boundary) -> (u64, SlotType) {
    let current_block_height = env.block.height;

    let next_block_height = if boundary.start.is_some() {
        match boundary.start.unwrap() {
            // Note: Not bothering with time, as that should get handled with the cron situations,
            // and probably throw an error when mixing blocks and cron
            BoundarySpec::Height(id) => {
                if current_block_height < id {
                    // shorthand - remove 1 since it adds 1 later
                    id - 1
                } else {
                    current_block_height
                }
            }
            _ => current_block_height,
        }
    } else {
        current_block_height
    };

    if boundary.end.is_some() {
        match boundary.end.unwrap() {
            BoundarySpec::Height(id) => {
                // stop if passed end height
                if current_block_height > id {
                    return (0, SlotType::Block);
                }
                // we ONLY want to catch if we're passed the end block height
                if next_block_height > id {
                    return (id, SlotType::Block);
                }
            }
            _ => unreachable!(),
        }
    }

    // immediate needs to return this block + 1
    (next_block_height + 1, SlotType::Block)
}

// So either:
// - Boundary specifies a start/end that block offsets can be computed from
// - Block offset will truncate to specific modulo offsets
fn get_next_block_by_offset(env: Env, boundary: Boundary, block: u64) -> (u64, SlotType) {
    let current_block_height = env.block.height;
    let modulo_block = current_block_height.saturating_sub(current_block_height % block) + block;

    let next_block_height = if boundary.start.is_some() {
        match boundary.start.unwrap() {
            // Note: Not bothering with time, as that should get handled with the cron situations,
            // and probably throw an error when mixing blocks and cron
            BoundarySpec::Height(id) => {
                if current_block_height < id {
                    let rem = id % block;
                    if rem > 0 {
                        id.saturating_sub(rem) + block
                    } else {
                        id
                    }
                } else {
                    modulo_block
                }
            }
            _ => modulo_block,
        }
    } else {
        modulo_block
    };

    if boundary.end.is_some() {
        match boundary.end.unwrap() {
            BoundarySpec::Height(id) => {
                let rem = id % block;
                let end_height = if rem > 0 { id.saturating_sub(rem) } else { id };

                // stop if passed end height
                if current_block_height > id {
                    return (0, SlotType::Block);
                }

                // we ONLY want to catch if we're passed the end block height
                if next_block_height > end_height {
                    return (end_height, SlotType::Block);
                }
            }
            _ => unreachable!(),
        }
    }

    (next_block_height, SlotType::Block)
}

impl Interval {
    pub fn next(&self, env: Env, boundary: Boundary) -> (u64, SlotType) {
        match self {
            // return the first block within a specific range that can be triggered 1 time.
            Interval::Once => get_next_block_limited(env, boundary),
            // return the first block within a specific range that can be triggered immediately, potentially multiple times.
            Interval::Immediate => get_next_block_limited(env, boundary),
            // return the first block within a specific range that can be triggered 1 or more times based on timestamps.
            // Uses crontab spec
            Interval::Cron(crontab) => {
                let current_block_ts: u64 = env.block.time.nanos();
                // TODO: get current timestamp within boundary
                let current_ts: u64 = if boundary.start.is_some() {
                    let start = boundary.start.unwrap();

                    match start {
                        // Note: Not bothering with height, as that should get handled with the block situations,
                        // and probably throw an error when mixing blocks and cron
                        BoundarySpec::Time(ts) => {
                            if current_block_ts < ts.nanos() {
                                ts.nanos()
                            } else {
                                current_block_ts
                            }
                        }
                        _ => current_block_ts,
                    }
                } else {
                    current_block_ts
                };

                let schedule = Schedule::from_str(crontab.as_str()).unwrap();
                let next_ts = schedule.next_after(&current_ts).unwrap();
                (next_ts, SlotType::Cron)
            }
            // return the block within a specific range that can be triggered 1 or more times based on block heights.
            // Uses block offset (Example: Block(100) will trigger every 100 blocks)
            // So either:
            // - Boundary specifies a start/end that block offsets can be computed from
            // - Block offset will truncate to specific modulo offsets
            Interval::Block(block) => get_next_block_by_offset(env, boundary, *block),
        }
    }
    pub fn is_valid(&self) -> bool {
        match self {
            Interval::Once => true,
            Interval::Immediate => true,
            Interval::Block(_) => true,
            Interval::Cron(crontab) => {
                let s = Schedule::from_str(crontab);
                s.is_ok()
            }
        }
    }
}
