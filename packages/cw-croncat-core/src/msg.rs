use crate::types::{
    Action, AgentResponse, Boundary, BoundaryValidated, GenericBalance, Interval, Rule, Task,
};
use crate::types::{Agent, SlotType};
use cosmwasm_std::{Addr, Coin, Timestamp, Uint64};
use cw20::Balance;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// NOTE: Which version is more practical?
// // Exporting a nice schema
// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// pub enum Croncat {
//     Agent(Agent),
//     Task(Task),
//     ConfigResponse(GetConfigResponse),
//     BalancesResponse(GetBalancesResponse),
//     GetAgentIdsResponse(GetAgentIdsResponse),
//     GetAgentTasksResponse(GetAgentTasksResponse),
//     TaskRequest(TaskRequest),
//     TaskResponse(TaskResponse),
//     ValidateIntervalResponse(ValidateIntervalResponse),
//     GetAgentResponse(GetAgentResponse),
//     GetTasksResponse(GetTasksResponse),
//     GetTasksByOwnerResponse(GetTasksByOwnerResponse),
//     GetTaskResponse(GetTaskResponse),
//     GetTaskHashResponse(GetTaskHashResponse),
//     GetSlotHashesResponse(GetSlotHashesResponse),
//     GetSlotIdsResponse(GetSlotIdsResponse),
// }

// Exporting a nice schema
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct Croncat {
    agent: Option<Agent>,
    task: Option<Task>,
    config_response: Option<GetConfigResponse>,
    balance_response: Option<GetBalancesResponse>,
    get_agent_ids_response: Option<GetAgentIdsResponse>,
    get_agent_tasks_response: Option<AgentTaskResponse>,
    task_request: Option<TaskRequest>,
    task_response: Option<TaskResponse>,
    validate_interval_response: Option<bool>,
    get_agent_response: Option<Option<AgentResponse>>,
    get_tasks_response: Option<Vec<TaskResponse>>,
    get_tasks_by_owner_response: Option<Vec<TaskResponse>>,
    get_task_response: Option<Option<TaskResponse>>,
    get_task_hash_response: Option<String>,
    get_slot_hashes_response: Option<GetSlotHashesResponse>,
    get_slot_ids_response: Option<GetSlotIdsResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    // TODO: Submit issue for AppBuilder tests not working for -- deps.querier.query_bonded_denom()?;
    pub denom: String,
    pub owner_id: Option<Addr>,
    pub gas_base_fee: Option<Uint64>,
    pub agent_nomination_duration: Option<u16>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateSettings {
        owner_id: Option<Addr>,
        slot_granularity: Option<u64>,
        paused: Option<bool>,
        agent_fee: Option<Coin>,
        gas_price: Option<u32>,
        proxy_callback_gas: Option<u32>,
        min_tasks_per_agent: Option<u64>,
        agents_eject_threshold: Option<u64>,
        // treasury_id: Option<Addr>,
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
pub struct GetConfigResponse {
    pub paused: bool,
    pub owner_id: Addr,
    // pub treasury_id: Option<Addr>,
    pub min_tasks_per_agent: u64,
    pub agent_active_indices: Vec<(SlotType, u32, u32)>,
    pub agents_eject_threshold: u64,
    pub agent_fee: Coin,
    pub gas_price: u32,
    pub proxy_callback_gas: u32,
    pub slot_granularity: u64,
    pub native_denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GetBalancesResponse {
    pub native_denom: String,
    pub available_balance: GenericBalance,
    pub staked_balance: GenericBalance,
    pub cw20_whitelist: Vec<Addr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct GetAgentIdsResponse {
    pub active: Vec<Addr>,
    pub pending: Vec<Addr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct AgentTaskResponse {
    pub num_block_tasks: Uint64,
    pub num_block_tasks_extra: Uint64,
    pub num_cron_tasks: Uint64,
    pub num_cron_tasks_extra: Uint64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TaskRequest {
    pub interval: Interval,
    pub boundary: Option<Boundary>,
    pub stop_on_fail: bool,
    pub actions: Vec<Action>,
    pub rules: Option<Vec<Rule>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TaskResponse {
    pub task_hash: String,
    pub owner_id: Addr,
    pub interval: Interval,
    pub boundary: Option<Boundary>,
    pub stop_on_fail: bool,
    pub total_deposit: Vec<Coin>,
    pub actions: Vec<Action>,
    pub rules: Option<Vec<Rule>>,
}

impl From<Task> for TaskResponse {
    fn from(task: Task) -> Self {
        let boundary = match (task.boundary, &task.interval) {
            (
                BoundaryValidated {
                    start: None,
                    end: None,
                },
                _,
            ) => None,
            (BoundaryValidated { start, end }, Interval::Cron(_)) => Some(Boundary::Time {
                start: start.map(Timestamp::from_nanos),
                end: end.map(Timestamp::from_nanos),
            }),
            (BoundaryValidated { start, end }, _) => Some(Boundary::Height {
                start: start.map(Into::into),
                end: end.map(Into::into),
            }),
        };
        TaskResponse {
            task_hash: task.to_hash(),
            owner_id: task.owner_id,
            interval: task.interval,
            boundary,
            stop_on_fail: task.stop_on_fail,
            total_deposit: task.total_deposit,
            actions: task.actions,
            rules: task.rules,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct GetSlotHashesResponse {
    pub block_id: u64,
    pub block_task_hash: Vec<String>,
    pub time_id: u64,
    pub time_task_hash: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct GetSlotIdsResponse {
    pub time_ids: Vec<u64>,
    pub block_ids: Vec<u64>,
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, coins, BankMsg, CosmosMsg, Timestamp, Uint128};
    use cw20::Cw20CoinVerified;

    use crate::types::AgentStatus;

    use super::*;

    use super::Croncat;

    #[test]
    fn everything_can_be_de_serealized() {
        let generic_balance = GenericBalance {
            native: vec![coin(5, "test")],
            cw20: vec![Cw20CoinVerified {
                address: Addr::unchecked("juno1"),
                amount: 125u128.into(),
            }],
        };
        let agent = Agent {
            payable_account_id: Addr::unchecked("test"),
            balance: generic_balance.clone(),
            total_tasks_executed: 0,
            last_missed_slot: 3,
            register_start: Timestamp::from_nanos(5),
        }
        .into();

        let msg: CosmosMsg = BankMsg::Send {
            to_address: "you".to_string(),
            amount: coins(1015, "earth"),
        }
        .into();

        let task = Task {
            funds_withdrawn_recurring: Uint128::zero(),
            owner_id: Addr::unchecked("nobody".to_string()),
            interval: Interval::Immediate,
            boundary: BoundaryValidated {
                start: Some(54),
                end: Some(44),
            },
            stop_on_fail: false,
            total_deposit: vec![],
            actions: vec![Action {
                msg,
                gas_limit: Some(150_000),
            }],
            rules: None,
        }
        .into();

        let config_response = GetConfigResponse {
            paused: true,
            owner_id: Addr::unchecked("bob"),
            min_tasks_per_agent: 5,
            agent_active_indices: vec![(SlotType::Block, 10, 5)],
            agents_eject_threshold: 5,
            agent_fee: coin(5, "earth"),
            gas_price: 2,
            proxy_callback_gas: 3,
            slot_granularity: 1,
            native_denom: "juno".to_string(),
        }
        .into();
        let balance_response = GetBalancesResponse {
            native_denom: "some".to_string(),
            available_balance: generic_balance.clone(),
            staked_balance: generic_balance.clone(),
            cw20_whitelist: vec![Addr::unchecked("bob")],
        }
        .into();
        let get_agent_ids_response = GetAgentIdsResponse {
            active: vec![Addr::unchecked("bob")],
            pending: vec![Addr::unchecked("bob")],
        }
        .into();
        let get_agent_tasks_response = AgentTaskResponse {
            num_block_tasks: 1u64.into(),
            num_block_tasks_extra: 2u64.into(),
            num_cron_tasks: 3u64.into(),
            num_cron_tasks_extra: 300u64.into(),
        }
        .into();
        let task_request = TaskRequest {
            interval: Interval::Block(5),
            boundary: Some(Boundary::Height {
                start: Some(Uint64::from(5u64)),
                end: Some(Uint64::from(64u64)),
            }),
            stop_on_fail: true,
            actions: vec![],
            rules: None, // TODO
        }
        .into();
        let task_response_raw = TaskResponse {
            task_hash: "test".to_string(),
            owner_id: Addr::unchecked("bob"),
            interval: Interval::Cron("blah-blah".to_string()),
            boundary: Some(Boundary::Time {
                start: Some(Timestamp::from_nanos(12345)),
                end: Some(Timestamp::from_nanos(67890)),
            }),
            stop_on_fail: true,
            total_deposit: vec![coin(5, "earth")],
            actions: vec![],
            rules: None,
        };
        let task_response = task_response_raw.clone().into();
        let validate_interval_response = false.into();
        let get_agent_response = Some(AgentResponse {
            status: AgentStatus::Active,
            payable_account_id: Addr::unchecked("bob"),
            balance: generic_balance.clone(),
            total_tasks_executed: 2,
            last_missed_slot: 2,
            register_start: Timestamp::from_nanos(5),
        })
        .into();
        let get_tasks_response = vec![task_response_raw.clone()].into();
        let get_tasks_by_owner_response = vec![task_response_raw.clone()].into();
        let get_task_response = Some(task_response_raw).into();
        let get_task_hash_response = ("asd".to_string()).into();
        let get_slot_hashes_response = GetSlotHashesResponse {
            block_id: 5,
            block_task_hash: vec!["bob".to_string()],
            time_id: 4,
            time_task_hash: vec!["alice".to_string()],
        }
        .into();
        let get_slot_ids_response = GetSlotIdsResponse {
            time_ids: vec![1],
            block_ids: vec![3],
        }
        .into();
        let croncat = Croncat {
            agent,
            task,
            config_response,
            balance_response,
            get_agent_ids_response,
            get_agent_tasks_response,
            task_request,
            task_response,
            validate_interval_response,
            get_agent_response,
            get_tasks_response,
            get_tasks_by_owner_response,
            get_task_response,
            get_task_hash_response,
            get_slot_hashes_response,
            get_slot_ids_response,
        };

        let ser = serde_json_wasm::to_string(&croncat);
        assert!(ser.is_ok());

        let deser: Result<Croncat, _> = serde_json_wasm::from_str(&ser.unwrap());
        assert!(deser.is_ok());
    }
}
