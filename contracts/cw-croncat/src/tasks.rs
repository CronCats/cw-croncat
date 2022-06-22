use crate::error::ContractError;
use crate::slots::Interval;
use crate::state::{Config, CwCroncat};
use cosmwasm_std::{
    coin, Addr, BankMsg, Coin, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult, SubMsg,
};
use cw20::Balance;
use cw_croncat_core::msg::{TaskRequest, TaskResponse};
use cw_croncat_core::types::{SlotType, Task};

impl<'a> CwCroncat<'a> {
    /// Returns task data
    /// Used by the frontend for viewing tasks
    pub(crate) fn query_get_tasks(
        &self,
        deps: Deps,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> StdResult<Vec<TaskResponse>> {
        let mut ret: Vec<TaskResponse> = Vec::new();
        let mut start = 0;
        let size: u64 = self
            .task_total
            .may_load(deps.storage)?
            .unwrap_or(100)
            .min(1000);
        if let Some(index) = from_index {
            start = index;
        }
        let mut end = u64::min(start.saturating_add(100), size);
        if let Some(l) = limit {
            end = u64::min(start.saturating_add(l), size);
        }

        // NOTE: could setup another index to allow efficient paginated
        let keys: Vec<Vec<u8>> = self
            .tasks
            .keys(deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?;

        for i in start..end {
            let task_hash = &keys[i as usize];
            let res = self.tasks.may_load(deps.storage, task_hash.to_vec())?;
            if let Some(task) = res {
                ret.push(TaskResponse {
                    task_hash: task.to_hash(),
                    owner_id: task.owner_id,
                    interval: task.interval,
                    boundary: task.boundary,
                    stop_on_fail: task.stop_on_fail,
                    total_deposit: task.total_deposit,
                    actions: task.actions,
                    rules: task.rules,
                });
            }
        }

        Ok(ret)
    }

    /// Returns task data for a specific owner
    pub(crate) fn query_get_tasks_by_owner(
        &self,
        deps: Deps,
        owner_id: Addr,
    ) -> StdResult<Vec<TaskResponse>> {
        let tasks_by_owner: Vec<TaskResponse> = self
            .tasks
            .idx
            .owner
            .prefix(owner_id)
            .range(deps.storage, None, None, Order::Ascending)
            .map(|x| {
                x.map(|(_, task)| TaskResponse {
                    task_hash: task.to_hash(),
                    owner_id: task.owner_id,
                    interval: task.interval,
                    boundary: task.boundary,
                    stop_on_fail: task.stop_on_fail,
                    total_deposit: task.total_deposit,
                    actions: task.actions,
                    rules: task.rules,
                })
            })
            .collect::<StdResult<Vec<_>>>()?;

        Ok(tasks_by_owner)
    }

    /// Returns single task data
    pub(crate) fn query_get_task(
        &self,
        deps: Deps,
        task_hash: String,
    ) -> StdResult<Option<TaskResponse>> {
        let res = self
            .tasks
            .may_load(deps.storage, task_hash.as_bytes().to_vec())?;
        if res.is_none() {
            return Ok(None);
        }

        let task: Task = res.unwrap();

        Ok(Some(TaskResponse {
            task_hash: task.to_hash(),
            owner_id: task.owner_id,
            interval: task.interval,
            boundary: task.boundary,
            stop_on_fail: task.stop_on_fail,
            total_deposit: task.total_deposit,
            actions: task.actions,
            rules: task.rules,
        }))
    }

    /// Returns a hash computed by the input task data
    pub(crate) fn query_get_task_hash(&self, task: Task) -> StdResult<String> {
        Ok(task.to_hash())
    }

    /// Check if interval params are valid by attempting to parse
    pub(crate) fn query_validate_interval(&self, interval: Interval) -> StdResult<bool> {
        Ok(interval.is_valid())
    }

    /// Gets a set of tasks.
    /// Default: Returns the next executable set of tasks hashes.
    ///
    /// Optional Parameters:
    /// "offset" - An unsigned integer specifying how far in the future to check for tasks that are slotted.
    ///
    /// Result:
    /// (block id, block task hash's, time id, time task hash's)
    pub(crate) fn query_slot_tasks(
        &self,
        deps: Deps,
        slot: Option<u64>,
    ) -> StdResult<(u64, Vec<String>, u64, Vec<String>)> {
        let mut block_id: u64 = 0;
        let mut block_hashes: Vec<Vec<u8>> = Vec::new();
        let mut time_id: u64 = 0;
        let mut time_hashes: Vec<Vec<u8>> = Vec::new();

        // Check if slot was supplied, otherwise get the next slots for block and time
        if let Some(id) = slot {
            block_hashes = self
                .block_slots
                .may_load(deps.storage, id)?
                .unwrap_or_default();
            if !block_hashes.is_empty() {
                block_id = id;
            }
            time_hashes = self
                .block_slots
                .may_load(deps.storage, id)?
                .unwrap_or_default();
            if !time_hashes.is_empty() {
                time_id = id;
            }
        } else {
            let time: Vec<(u64, _)> = self
                .time_slots
                .range(deps.storage, None, None, Order::Ascending)
                .take(1)
                .collect::<StdResult<Vec<(u64, _)>>>()?;

            if !time.is_empty() {
                // (time_id, time_hashes) = time[0].clone();
                let slot = time[0].clone();
                time_id = slot.0;
                time_hashes = slot.1;
            }

            let block: Vec<(u64, _)> = self
                .block_slots
                .range(deps.storage, None, None, Order::Ascending)
                .take(1)
                .collect::<StdResult<Vec<(u64, _)>>>()?;

            if !block.is_empty() {
                // (block_id, block_hashes) = block[0].clone();
                let slot = block[0].clone();
                block_id = slot.0;
                block_hashes = slot.1;
            }
        }

        // Generate strings for all hashes
        let b_hashes: Vec<_> = block_hashes
            .iter()
            .map(|b| String::from_utf8(b.to_vec()).unwrap_or_else(|_| "".to_string()))
            .collect();
        let t_hashes: Vec<_> = time_hashes
            .iter()
            .map(|t| String::from_utf8(t.to_vec()).unwrap_or_else(|_| "".to_string()))
            .collect();

        Ok((block_id, b_hashes, time_id, t_hashes))
    }

    /// Gets list of active slot ids, for both time & block slots
    /// (time, block)
    pub(crate) fn query_slot_ids(&self, deps: Deps) -> StdResult<(Vec<u64>, Vec<u64>)> {
        let time: Vec<u64> = self
            .time_slots
            .keys(deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?;
        let block: Vec<u64> = self
            .block_slots
            .keys(deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?;
        Ok((time, block))
    }

    /// Allows any user or contract to pay for future txns based on a specific schedule
    /// contract, function id & other settings. When the task runs out of balance
    /// the task is no longer executed, any additional funds will be returned to task owner.
    pub fn create_task(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        env: Env,
        task: TaskRequest,
    ) -> Result<Response, ContractError> {
        if info.funds.is_empty() {
            return Err(ContractError::CustomError {
                val: "Must attach funds".to_string(),
            });
        }
        let c: Config = self.config.load(deps.storage)?;
        if c.paused {
            return Err(ContractError::CustomError {
                val: "Create task paused".to_string(),
            });
        }

        let owner_id = info.sender;
        let item = Task {
            owner_id: owner_id.clone(),
            interval: task.interval,
            boundary: task.boundary,
            stop_on_fail: task.stop_on_fail,
            total_deposit: info.funds.clone(),
            actions: task.actions,
            rules: task.rules,
        };

        if !item.is_valid_msg(&env.contract.address, &owner_id, &c.owner_id) {
            return Err(ContractError::CustomError {
                val: "Actions Message Unsupported".to_string(),
            });
        }

        if !item.interval.is_valid() {
            return Err(ContractError::CustomError {
                val: "Interval invalid".to_string(),
            });
        }

        // TODO:
        // // Check that balance is sufficient for 1 execution minimum
        // let call_balance_used = self.task_balance_uses(&item);
        // let min_balance_needed: u128 = if recurring == Some(true) {
        //     call_balance_used * 2
        // } else {
        //     call_balance_used
        // };
        // assert!(
        //     min_balance_needed <= item.total_deposit.0,
        //     "Not enough task balance to execute job, need at least {}",
        //     min_balance_needed
        // );

        let hash = item.to_hash();

        // Parse interval into a future timestamp, then convert to a slot
        let (next_id, slot_kind) = item.interval.next(env.clone(), item.boundary);

        // If the next interval comes back 0, then this task should not schedule again
        if next_id == 0 {
            return Err(ContractError::CustomError {
                val: "Task ended".to_string(),
            });
        }

        // Add task to catalog
        self.tasks
            .update(deps.storage, item.to_hash_vec(), |old| match old {
                Some(_) => Err(ContractError::CustomError {
                    val: "Task already exists".to_string(),
                }),
                None => Ok(item.clone()),
            })?;

        // Increment task totals
        let size_res = self.increment_tasks(deps.storage);
        let size = if let Ok(size) = size_res {
            size
        } else {
            return Err(ContractError::CustomError {
                val: "Problem incrementing task total".to_string(),
            });
        };

        // Get previous task hashes in slot, add as needed
        let update_vec_data = |d: Option<Vec<Vec<u8>>>| -> StdResult<Vec<Vec<u8>>> {
            match d {
                // has some data, simply push new hash
                Some(data) => {
                    let mut s = data;
                    s.push(item.to_hash_vec());
                    Ok(s)
                }
                // No data, push new vec & hash
                None => Ok(vec![item.to_hash_vec()]),
            }
        };

        // Based on slot kind, put into block or cron slots
        match slot_kind {
            SlotType::Block => {
                self.block_slots
                    .update(deps.storage, next_id, update_vec_data)?;
            }
            SlotType::Cron => {
                self.time_slots
                    .update(deps.storage, next_id, update_vec_data)?;
            }
        }

        // Add the attached balance into available_balance
        let mut c: Config = self.config.load(deps.storage)?;
        c.available_balance.add_tokens(Balance::from(info.funds));

        // If the creation of this task means we'd like another agent, update config
        let min_tasks_per_agent = c.min_tasks_per_agent;
        let num_active_agents = self
            .agent_active_queue
            .may_load(deps.storage)?
            .unwrap_or_default()
            .len() as u64;
        let num_agents_to_accept =
            self.agents_to_let_in(&min_tasks_per_agent, &num_active_agents, &size);
        // If we should allow a new agent to take over
        if num_agents_to_accept != 0 {
            // Don't wipe out an older timestamp
            if c.agent_nomination_begin_time.is_none() {
                c.agent_nomination_begin_time = Some(env.block.time)
            }
        }

        self.config.save(deps.storage, &c)?;

        Ok(Response::new()
            .add_attribute("method", "create_task")
            .add_attribute("slot_id", next_id.to_string())
            .add_attribute("slot_kind", format!("{:?}", slot_kind))
            .add_attribute("task_hash", hash))
    }

    /// Deletes a task in its entirety, returning any remaining balance to task owner.
    pub fn remove_task(&self, deps: DepsMut, task_hash: String) -> Result<Response, ContractError> {
        let hash_vec = task_hash.clone().into_bytes();
        let task_raw = self.tasks.may_load(deps.storage, hash_vec.clone())?;
        let task = if let Some(task) = task_raw {
            task
        } else {
            return Err(ContractError::CustomError {
                val: "No task found by hash".to_string(),
            });
        };

        // Remove all the thangs
        self.tasks.remove(deps.storage, hash_vec)?;

        // find any scheduled things and remove them!
        // check which type of slot it would be in, then iterate to remove
        // NOTE: def could use some spiffy refactor here
        let time_ids: Vec<u64> = self
            .time_slots
            .keys(deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?;

        for tid in time_ids {
            let mut time_hashes = self
                .time_slots
                .may_load(deps.storage, tid)?
                .unwrap_or_default();
            if !time_hashes.is_empty() {
                time_hashes.retain(|h| String::from_utf8(h.to_vec()).unwrap() != task_hash.clone());
            }

            // save the updates, remove if slot no longer has hashes
            if time_hashes.is_empty() {
                self.time_slots.remove(deps.storage, tid);
            } else {
                self.time_slots.save(deps.storage, tid, &time_hashes)?;
            }
        }
        let block_ids: Vec<u64> = self
            .block_slots
            .keys(deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?;

        for bid in block_ids {
            let mut block_hashes = self
                .block_slots
                .may_load(deps.storage, bid)?
                .unwrap_or_default();
            if !block_hashes.is_empty() {
                block_hashes
                    .retain(|h| String::from_utf8(h.to_vec()).unwrap() != task_hash.clone());
            }

            // save the updates, remove if slot no longer has hashes
            if block_hashes.is_empty() {
                self.block_slots.remove(deps.storage, bid);
            } else {
                self.block_slots.save(deps.storage, bid, &block_hashes)?;
            }
        }

        // setup sub-msgs for returning any remaining total_deposit to the owner
        let task = task;
        let submsgs = SubMsg::new(BankMsg::Send {
            to_address: task.clone().owner_id.into(),
            amount: task.clone().total_deposit,
        });

        // remove from the total available_balance
        let mut c: Config = self.config.load(deps.storage)?;
        c.available_balance.minus_tokens(&task.total_deposit);
        self.config.save(deps.storage, &c)?;

        Ok(Response::new()
            .add_attribute("method", "remove_task")
            .add_submessage(submsgs))
    }

    /// Refill a task with more balance to continue its execution
    /// NOTE: Restricting this to owner only, so owner can make sure the task ends
    pub fn refill_task(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        task_hash: String,
    ) -> Result<Response, ContractError> {
        let hash_vec = task_hash.into_bytes();
        let task_raw = self.tasks.may_load(deps.storage, hash_vec.clone())?;
        let mut task = if let Some(task) = task_raw {
            task
        } else {
            return Err(ContractError::CustomError {
                val: "Task doesnt exist".to_string(),
            });
        };
        if task.owner_id != info.sender {
            return Err(ContractError::CustomError {
                val: "Only owner can refill their task".to_string(),
            });
        }

        // Add the attached balance into available_balance
        self.config
            .update(deps.storage, |mut config| -> Result<_, ContractError> {
                config
                    .available_balance
                    .add_tokens(Balance::from(info.funds.clone()));
                Ok(config)
            })?;

        let mut total_balance: Vec<Coin> = vec![];
        for t in task.total_deposit.iter() {
            for f in &info.funds {
                if f.denom == t.denom {
                    let amt = t.clone().amount.saturating_add(f.amount);
                    total_balance.push(coin(amt.into(), t.clone().denom));
                } else {
                    total_balance.push(t.clone());
                }
            }
        }
        task.total_deposit = total_balance;

        // update the task
        self.tasks.update(deps.storage, hash_vec, |old| match old {
            Some(_) => Ok(task.clone()),
            None => Err(ContractError::CustomError {
                val: "Task doesnt exist".to_string(),
            }),
        })?;

        // return the task total
        let coins_total: String = task.total_deposit.iter().map(|a| a.to_string()).collect();
        Ok(Response::new()
            .add_attribute("method", "refill_task")
            .add_attribute("total_deposit", coins_total))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::{
        coin, coins, to_binary, Addr, BankMsg, CosmosMsg, Empty, StakingMsg, WasmMsg,
    };
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
    // use crate::error::ContractError;
    use crate::helpers::CwTemplateContract;
    use cw_croncat_core::msg::{
        BalancesResponse, ExecuteMsg, InstantiateMsg, QueryMsg, UpdateSettings,
    };
    use cw_croncat_core::types::{Action, Boundary, BoundarySpec};

    pub fn contract_template() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::entry::execute,
            crate::entry::instantiate,
            crate::entry::query,
        );
        Box::new(contract)
    }

    const ADMIN: &str = "cosmos1sjllsnramtg3ewxqwwrwjxfgc4n4ef9u0tvx7u";
    const ANYONE: &str = "cosmos1t5u0jfg3ljsjrh2m9e47d4ny2hea7eehxrzdgd";
    const NATIVE_DENOM: &str = "atom";

    fn mock_app() -> App {
        AppBuilder::new().build(|router, _, storage| {
            let accounts: Vec<(u128, String)> =
                vec![(100, ADMIN.to_string()), (100, ANYONE.to_string())];
            for (amt, address) in accounts.iter() {
                router
                    .bank
                    .init_balance(
                        storage,
                        &Addr::unchecked(address),
                        vec![coin(amt.clone(), NATIVE_DENOM.to_string())],
                    )
                    .unwrap();
            }
        })
    }

    fn proper_instantiate() -> (App, CwTemplateContract) {
        let mut app = mock_app();
        let cw_template_id = app.store_code(contract_template());
        let owner_addr = Addr::unchecked(ADMIN);

        let msg = InstantiateMsg {
            denom: "atom".to_string(),
            owner_id: Some(owner_addr.clone()),
            agent_nomination_duration: Some(360),
        };
        let cw_template_contract_addr = app
            .instantiate_contract(cw_template_id, owner_addr, &msg, &[], "Manager", None)
            .unwrap();

        let cw_template_contract = CwTemplateContract(cw_template_contract_addr);

        (app, cw_template_contract)
    }

    #[test]
    fn query_task_hash_success() {
        let (app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        let to_address = String::from("you");
        let amount = coins(1015, "earth");
        let bank = BankMsg::Send { to_address, amount };
        let msg: CosmosMsg = bank.clone().into();

        let task = Task {
            owner_id: Addr::unchecked("nobody".to_string()),
            interval: Interval::Immediate,
            boundary: Boundary {
                start: None,
                end: None,
            },
            stop_on_fail: false,
            total_deposit: coins(37, "atom"),
            actions: vec![Action {
                msg,
                gas_limit: Some(150_000),
            }],
            rules: None,
        };

        // HASH CHECK!
        let task_hash: String = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTaskHash {
                    task: Box::new(task),
                },
            )
            .unwrap();
        assert_eq!(
            "3ccb739ea050ebbd2e08f74aeb0b7aa081b15fa78504cba44155ec774452bbee",
            task_hash
        );
    }

    #[test]
    fn query_validate_interval_success() {
        let (app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        let intervals: Vec<Interval> = vec![
            Interval::Once,
            Interval::Immediate,
            Interval::Block(12345),
            Interval::Cron("0 0 * * * *".to_string()),
        ];
        for i in intervals.iter() {
            let valid: bool = app
                .wrap()
                .query_wasm_smart(
                    &contract_addr.clone(),
                    &QueryMsg::ValidateInterval {
                        interval: i.to_owned(),
                    },
                )
                .unwrap();
            assert!(valid);
        }
    }

    #[test]
    fn query_get_tasks() {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        let validator = String::from("you");
        let amount = coin(3, "atom");
        let stake = StakingMsg::Delegate { validator, amount };
        let msg: CosmosMsg = stake.clone().into();

        let create_task_msg = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: Boundary {
                    start: None,
                    end: None,
                },
                stop_on_fail: false,
                actions: vec![Action {
                    msg,
                    gas_limit: Some(150_000),
                }],
                rules: None,
            },
        };

        // create a task
        app.execute_contract(
            Addr::unchecked(ANYONE),
            contract_addr.clone(),
            &create_task_msg,
            &coins(37, "atom"),
        )
        .unwrap();

        // check storage has the task
        let all_tasks: Vec<TaskResponse> = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTasks {
                    from_index: None,
                    limit: None,
                },
            )
            .unwrap();
        assert_eq!(all_tasks.len(), 1);

        let owner_tasks: Vec<TaskResponse> = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTasksByOwner {
                    owner_id: Addr::unchecked(ANYONE),
                },
            )
            .unwrap();
        assert_eq!(owner_tasks.len(), 1);
    }

    #[test]
    fn check_task_create_fail_cases() -> StdResult<()> {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        let validator = String::from("you");
        let amount = coin(3, "atom");
        let stake = StakingMsg::Delegate { validator, amount };
        let msg: CosmosMsg = stake.clone().into();

        let create_task_msg = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: Boundary {
                    start: None,
                    end: None,
                },
                stop_on_fail: false,
                actions: vec![Action {
                    msg: msg.clone(),
                    gas_limit: Some(150_000),
                }],
                rules: None,
            },
        };
        // let task_id_str = "ad15b0f15010d57a51ff889d3400fe8d083a0dab2acfc752c5eb55e9e6281705".to_string();
        // let task_id = task_id_str.clone().into_bytes();

        // Must attach funds
        let res_err = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &create_task_msg,
                &vec![],
            )
            .unwrap_err();
        assert_eq!(
            ContractError::CustomError {
                val: "Must attach funds".to_string()
            },
            res_err.downcast().unwrap()
        );

        // Create task paused
        let change_settings_msg = ExecuteMsg::UpdateSettings {
            update_settings: UpdateSettings {
                paused: Some(true),
                owner_id: None,
                // treasury_id: None,
                agent_fee: None,
                agents_eject_threshold: None,
                gas_price: None,
                proxy_callback_gas: None,
                slot_granularity: None,
                min_tasks_per_agent: None,
            },
        };
        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &change_settings_msg,
            &vec![],
        )
        .unwrap();
        let res_err = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &create_task_msg,
                &coins(13, "atom"),
            )
            .unwrap_err();
        assert_eq!(
            ContractError::CustomError {
                val: "Create task paused".to_string()
            },
            res_err.downcast().unwrap()
        );
        // Set it back
        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &ExecuteMsg::UpdateSettings {
                update_settings: UpdateSettings {
                    paused: Some(false),
                    owner_id: None,
                    // treasury_id: None,
                    agent_fee: None,
                    agents_eject_threshold: None,
                    gas_price: None,
                    proxy_callback_gas: None,
                    slot_granularity: None,
                    min_tasks_per_agent: None,
                },
            },
            &vec![],
        )
        .unwrap();

        // Creator invalid
        let action_self = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.clone().into_string(),
            funds: vec![],
            msg: to_binary(&change_settings_msg.clone())?,
        });
        let res_err = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &ExecuteMsg::CreateTask {
                    task: TaskRequest {
                        interval: Interval::Once,
                        boundary: Boundary {
                            start: None,
                            end: None,
                        },
                        stop_on_fail: false,
                        actions: vec![Action {
                            msg: action_self.clone(),
                            gas_limit: Some(150_000),
                        }],
                        rules: None,
                    },
                },
                &coins(13, "atom"),
            )
            .unwrap_err();
        assert_eq!(
            ContractError::CustomError {
                val: "Actions Message Unsupported".to_string()
            },
            res_err.downcast().unwrap()
        );

        // Interval invalid
        let res_err = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &ExecuteMsg::CreateTask {
                    task: TaskRequest {
                        interval: Interval::Cron("faux_paw".to_string()),
                        boundary: Boundary {
                            start: None,
                            end: None,
                        },
                        stop_on_fail: false,
                        actions: vec![Action {
                            msg: msg.clone(),
                            gas_limit: Some(150_000),
                        }],
                        rules: None,
                    },
                },
                &coins(13, "atom"),
            )
            .unwrap_err();
        assert_eq!(
            ContractError::CustomError {
                val: "Interval invalid".to_string()
            },
            res_err.downcast().unwrap()
        );

        // Task already exists
        app.execute_contract(
            Addr::unchecked(ANYONE),
            contract_addr.clone(),
            &create_task_msg,
            &coins(13, "atom"),
        )
        .unwrap();
        let res_err = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &create_task_msg,
                &coins(13, "atom"),
            )
            .unwrap_err();
        assert_eq!(
            ContractError::CustomError {
                val: "Task already exists".to_string()
            },
            res_err.downcast().unwrap()
        );

        // Task ended
        let res_err = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &ExecuteMsg::CreateTask {
                    task: TaskRequest {
                        interval: Interval::Block(12346),
                        boundary: Boundary {
                            start: None,
                            end: Some(BoundarySpec::Height(1)),
                        },
                        stop_on_fail: false,
                        actions: vec![Action {
                            msg,
                            gas_limit: Some(150_000),
                        }],
                        rules: None,
                    },
                },
                &coins(13, "atom"),
            )
            .unwrap_err();
        assert_eq!(
            ContractError::CustomError {
                val: "Task ended".to_string()
            },
            res_err.downcast().unwrap()
        );

        // TODO: (needs impl!) Not enough task balance to execute job

        Ok(())
    }

    #[test]
    fn check_task_create_success() -> StdResult<()> {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        let validator = String::from("you");
        let amount = coin(3, "atom");
        let stake = StakingMsg::Delegate { validator, amount };
        let msg: CosmosMsg = stake.clone().into();

        let create_task_msg = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: Boundary {
                    start: None,
                    end: None,
                },
                stop_on_fail: false,
                actions: vec![Action {
                    msg,
                    gas_limit: Some(150_000),
                }],
                rules: None,
            },
        };
        let task_id_str =
            "ad15b0f15010d57a51ff889d3400fe8d083a0dab2acfc752c5eb55e9e6281705".to_string();

        // create a task
        let res = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &create_task_msg,
                &coins(37, "atom"),
            )
            .unwrap();
        // Assert task hash is returned as part of event attributes
        let mut has_created_hash: bool = false;
        for e in res.events {
            for a in e.attributes {
                if a.key == "task_hash" && a.value == task_id_str.clone() {
                    has_created_hash = true;
                }
            }
        }
        assert!(has_created_hash);

        // check storage has the task
        let new_task: Option<TaskResponse> = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTask {
                    task_hash: task_id_str.clone(),
                },
            )
            .unwrap();
        assert!(new_task.is_some());
        if let Some(t) = new_task {
            assert_eq!(Addr::unchecked(ANYONE), t.owner_id);
            assert_eq!(Interval::Immediate, t.interval);
            assert_eq!(
                Boundary {
                    start: None,
                    end: None,
                },
                t.boundary
            );
            assert_eq!(false, t.stop_on_fail);
            assert_eq!(coins(37, "atom"), t.total_deposit);
            assert_eq!(task_id_str.clone(), t.task_hash);
        }

        // get slot ids
        let slot_ids: (Vec<u64>, Vec<u64>) = app
            .wrap()
            .query_wasm_smart(&contract_addr.clone(), &QueryMsg::GetSlotIds {})
            .unwrap();
        let s_1: Vec<u64> = Vec::new();
        assert_eq!(s_1, slot_ids.0);
        assert_eq!(vec![12346], slot_ids.1);

        // get slot hashs
        let slot_info: (u64, Vec<String>, u64, Vec<String>) = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetSlotHashes { slot: None },
            )
            .unwrap();
        let s_3: Vec<String> = Vec::new();
        assert_eq!(12346, slot_info.0);
        assert_eq!(vec![task_id_str.clone()], slot_info.1);
        assert_eq!(0, slot_info.2);
        assert_eq!(s_3, slot_info.3);

        Ok(())
    }

    #[test]
    fn check_remove_create() -> StdResult<()> {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        let validator = String::from("you");
        let amount = coin(3, "atom");
        let stake = StakingMsg::Delegate { validator, amount };
        let msg: CosmosMsg = stake.clone().into();

        let create_task_msg = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: Boundary {
                    start: None,
                    end: None,
                },
                stop_on_fail: false,
                actions: vec![Action {
                    msg,
                    gas_limit: Some(150_000),
                }],
                rules: None,
            },
        };
        let task_id_str =
            "ad15b0f15010d57a51ff889d3400fe8d083a0dab2acfc752c5eb55e9e6281705".to_string();

        // create a task
        app.execute_contract(
            Addr::unchecked(ANYONE),
            contract_addr.clone(),
            &create_task_msg,
            &coins(37, "atom"),
        )
        .unwrap();

        // check storage DOES have the task
        let new_task: Option<TaskResponse> = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTask {
                    task_hash: task_id_str.clone(),
                },
            )
            .unwrap();
        assert!(new_task.is_some());

        // Confirm slot exists, proving task was scheduled
        let slot_ids: (Vec<u64>, Vec<u64>) = app
            .wrap()
            .query_wasm_smart(&contract_addr.clone(), &QueryMsg::GetSlotIds {})
            .unwrap();
        let s_1: Vec<u64> = Vec::new();
        assert_eq!(s_1, slot_ids.0);
        assert_eq!(vec![12346], slot_ids.1);

        // Remove the Task
        app.execute_contract(
            Addr::unchecked(ANYONE),
            contract_addr.clone(),
            &ExecuteMsg::RemoveTask {
                task_hash: task_id_str.clone(),
            },
            &vec![],
        )
        .unwrap();

        // check storage DOESNT have the task
        let rem_task: Option<TaskResponse> = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTask {
                    task_hash: task_id_str.clone(),
                },
            )
            .unwrap();
        assert!(rem_task.is_none());

        // Check the contract total balance has decreased from the removed task
        let balances: BalancesResponse = app
            .wrap()
            .query_wasm_smart(&contract_addr.clone(), &QueryMsg::GetBalances {})
            .unwrap();
        assert_eq!(coins(0, "atom"), balances.available_balance.native);

        // Check the slots correctly removed the task
        let slot_ids: (Vec<u64>, Vec<u64>) = app
            .wrap()
            .query_wasm_smart(&contract_addr.clone(), &QueryMsg::GetSlotIds {})
            .unwrap();
        let s: Vec<u64> = Vec::new();
        assert_eq!(s.clone(), slot_ids.0);
        assert_eq!(s, slot_ids.1);

        Ok(())
    }

    #[test]
    fn check_refill_create() -> StdResult<()> {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        let validator = String::from("you");
        let amount = coin(3, "atom");
        let stake = StakingMsg::Delegate { validator, amount };
        let msg: CosmosMsg = stake.clone().into();

        let create_task_msg = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: Boundary {
                    start: None,
                    end: None,
                },
                stop_on_fail: false,
                actions: vec![Action {
                    msg,
                    gas_limit: Some(150_000),
                }],
                rules: None,
            },
        };
        let task_id_str =
            "ad15b0f15010d57a51ff889d3400fe8d083a0dab2acfc752c5eb55e9e6281705".to_string();

        // create a task
        app.execute_contract(
            Addr::unchecked(ANYONE),
            contract_addr.clone(),
            &create_task_msg,
            &coins(37, "atom"),
        )
        .unwrap();
        // refill task
        let res = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &ExecuteMsg::RefillTaskBalance {
                    task_hash: task_id_str.clone(),
                },
                &coins(3, "atom"),
            )
            .unwrap();
        // Assert returned event attributes include total
        let mut matches_new_totals: bool = false;
        for e in res.events {
            for a in e.attributes {
                if a.key == "total_deposit" && a.value == "40atom".to_string() {
                    matches_new_totals = true;
                }
            }
        }
        assert!(matches_new_totals);

        // check the task totals
        let new_task: Option<TaskResponse> = app
            .wrap()
            .query_wasm_smart(
                &contract_addr.clone(),
                &QueryMsg::GetTask {
                    task_hash: task_id_str.clone(),
                },
            )
            .unwrap();
        assert!(new_task.is_some());

        if let Some(t) = new_task {
            assert_eq!(Addr::unchecked(ANYONE), t.owner_id);
            assert_eq!(coins(40, "atom"), t.total_deposit);
        }

        // Check the balance has increased to include the new refilled total
        let balances: BalancesResponse = app
            .wrap()
            .query_wasm_smart(&contract_addr.clone(), &QueryMsg::GetBalances {})
            .unwrap();
        assert_eq!(coins(40, "atom"), balances.available_balance.native);

        Ok(())
    }
}
