use crate::types::Agent;
use crate::types::{Action, Boundary, GenericBalance, Interval, Rule, Task};
use cosmwasm_std::{Addr, Coin, Timestamp};
use cw20::Balance;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// NOTE: Which version is more practical?
// // Exporting a nice schema
// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// pub enum Croncat {
//     Agent(Agent),
//     // Config(Config),
//     Task(Task),
//     ConfigResponse(ConfigResponse),
//     BalancesResponse(BalancesResponse),
//     GetAgentIdsResponse(GetAgentIdsResponse),
//     GetAgentTasksResponse(GetAgentTasksResponse),
//     TaskRequest(TaskRequest),
//     TaskResponse(TaskResponse),
// }

// Exporting a nice schema
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Croncat {
    agent: Option<Agent>,
    task: Option<Task>,
    config_response: Option<ConfigResponse>,
    balance_response: Option<BalancesResponse>,
    get_agent_ids_response: Option<GetAgentIdsResponse>,
    get_agent_tasks_response: Option<GetAgentTasksResponse>,
    task_request: Option<TaskRequest>,
    task_response: Option<TaskResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    // TODO: Submit issue for AppBuilder tests not working for -- deps.querier.query_bonded_denom()?;
    pub denom: String,
    pub owner_id: Option<Addr>,
    pub agent_nomination_duration: Option<u16>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateSettings {
        update_settings: UpdateSettings,
    },
    MoveBalances {
        balances: Vec<Balance>,
        account_id: Addr,
    },

    RegisterAgent {
        payable_account_id: Option<Addr>,
    },
    UpdateAgent {
        payable_account_id: Addr,
    },
    CheckInAgent {},
    UnregisterAgent {},
    WithdrawReward {},

    CreateTask {
        task: TaskRequest,
    },
    RemoveTask {
        task_hash: String,
    },
    RefillTaskBalance {
        task_hash: String,
    },
    ProxyCall {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetConfig {},
    GetBalances {},
    GetAgent {
        account_id: Addr,
    },
    GetAgentIds {},
    GetAgentTasks {
        account_id: Addr,
    },
    GetTasks {
        from_index: Option<u64>,
        limit: Option<u64>,
    },
    GetTasksByOwner {
        owner_id: Addr,
    },
    GetTask {
        task_hash: String,
    },
    GetTaskHash {
        task: Box<Task>,
    },
    ValidateInterval {
        interval: Interval,
    },
    GetSlotHashes {
        slot: Option<u64>,
    },
    GetSlotIds {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub paused: bool,
    pub owner_id: Addr,
    // pub treasury_id: Option<Addr>,
    pub min_tasks_per_agent: u64,
    pub agent_active_index: u64,
    pub agents_eject_threshold: u64,
    pub agent_fee: Coin,
    pub gas_price: u32,
    pub proxy_callback_gas: u32,
    pub slot_granularity: u64,
    pub native_denom: String,
    pub agent_nomination_begin_time: Option<Timestamp>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BalancesResponse {
    pub native_denom: String,
    pub available_balance: GenericBalance,
    pub staked_balance: GenericBalance,
    pub cw20_whitelist: Vec<Addr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GetAgentIdsResponse {
    pub active: Vec<Addr>,
    pub pending: Vec<Addr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GetAgentTasksResponse(pub u64, pub u64);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdateSettings {
    pub owner_id: Option<Addr>,
    pub slot_granularity: Option<u64>,
    pub paused: Option<bool>,
    pub agent_fee: Option<Coin>,
    pub gas_price: Option<u32>,
    pub proxy_callback_gas: Option<u32>,
    pub min_tasks_per_agent: Option<u64>,
    pub agents_eject_threshold: Option<u64>,
    // treasury_id: Option<Addr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TaskRequest {
    pub interval: Interval,
    pub boundary: Boundary,
    pub stop_on_fail: bool,
    pub actions: Vec<Action>,
    pub rules: Option<Vec<Rule>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TaskResponse {
    pub task_hash: String,
    pub owner_id: Addr,
    pub interval: Interval,
    pub boundary: Boundary,
    pub stop_on_fail: bool,
    pub total_deposit: Vec<Coin>,
    pub actions: Vec<Action>,
    pub rules: Option<Vec<Rule>>,
}
