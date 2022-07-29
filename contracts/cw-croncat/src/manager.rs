use crate::error::ContractError;
use crate::helpers::ReplyMsgParser;
use crate::state::{Config, CwCroncat, QueueItem};
use cosmwasm_std::{
    Addr, DepsMut, Empty, Env, MessageInfo, Reply, Response, StdResult, Storage, SubMsg,
    SubMsgResult,
};
use cw20::Balance;
use cw_croncat_core::traits::Intervals;
use cw_croncat_core::types::{Agent, SlotType};

impl<'a> CwCroncat<'a> {
    /// Executes a task based on the current task slot
    /// Computes whether a task should continue further or not
    /// Makes a cross-contract call with the task configuration
    /// Called directly by a registered agent
    pub fn proxy_call(
        &mut self,
        deps: DepsMut,
        info: MessageInfo,
        env: Env,
    ) -> Result<Response, ContractError> {
        if !info.funds.is_empty() {
            return Err(ContractError::CustomError {
                val: "Must not attach funds".to_string(),
            });
        }
        let c: Config = self.config.load(deps.storage)?;
        if c.paused {
            return Err(ContractError::CustomError {
                val: "Contract paused".to_string(),
            });
        }

        if c.available_balance.native.is_empty() {
            return Err(ContractError::CustomError {
                val: "Not enough available balance for sending agent reward".to_string(),
            });
        }
        // only registered agent signed, because micropayments will benefit long term
        let agent_opt = self.agents.may_load(deps.storage, info.sender.clone())?;
        if agent_opt.is_none() {
            return Err(ContractError::AgentNotRegistered {});
        }
        let active_agents: Vec<Addr> = self.agent_active_queue.load(deps.storage)?;

        // make sure agent is active
        if !active_agents.contains(&info.sender) {
            return Err(ContractError::AgentNotRegistered {});
        }
        let agent = agent_opt.unwrap();

        // get slot items, find the next task hash available
        // if empty slot found, let agent get paid for helping keep house clean
        let slot = self.get_current_slot_items(&env.block, deps.storage, Some(1));
        // Give preference for block-based slots
        let slot_id: u64;
        let some_hash: Option<Vec<u8>>;
        if slot.0.is_none() {
            // See if there are cron (time-based) tasks to execute
            if slot.1.is_none() {
                self.send_base_agent_reward(deps.storage, agent, info);
                return Err(ContractError::CustomError {
                    val: "No Tasks For Slot".to_string(),
                });
            } else {
                slot_id = slot.1.unwrap();
                // There aren't block tasks but there are cron tasks
                some_hash = self.pop_slot_item(deps.storage, &slot_id, &SlotType::Cron);
            }
        } else {
            // There are block tasks (which we prefer to execute before time-based ones at this point)
            slot_id = slot.0.unwrap();
            some_hash = self.pop_slot_item(deps.storage, &slot.0.unwrap(), &SlotType::Block);
        }
        if some_hash.is_none() {
            self.send_base_agent_reward(deps.storage, agent, info);
            return Err(ContractError::CustomError {
                val: "No Tasks For Slot".to_string(),
            });
        }

        // Get the task details
        // if no task, exit and reward agent.
        let hash = some_hash.unwrap();
        let some_task = self.tasks.may_load(deps.storage, hash.clone())?;
        if some_task.is_none() {
            // NOTE: This could should never get reached, however we cover just in case
            self.send_base_agent_reward(deps.storage, agent, info);
            return Err(ContractError::NoTaskFound {});
        }

        // ----------------------------------------------------
        // TODO: FINISH!!!!!!
        // AGENT Task Allowance Logic: see line 339
        // ----------------------------------------------------

        let task = some_task.unwrap();

        //Restrict bank msg so contract doesnt get drained
        if task.is_recurring()
            && task.contains_send_msg()
            && !task.is_valid_msg(&env.contract.address, &info.sender, &c.owner_id)
        {
            return Err(ContractError::CustomError {
                val: "Invalid process_call message!".to_string(),
            });
        }

        // TODO: Bring this back!
        // // Fee breakdown:
        // // - Used Gas: Task Txn Fee Cost
        // // - Agent Fee: Incentivize Execution SLA
        // //
        // // Task Fee Examples:
        // // Total Fee = Gas Fee + Agent Fee
        // // Total Balance = Task Deposit + Total Fee
        // //
        // // NOTE: Gas cost includes the cross-contract call & internal logic of this contract.
        // // Direct contract gas fee will be lower than task execution costs, however
        // // we require the task owner to appropriately estimate gas for overpayment.
        // // The gas overpayment will also accrue to the agent since there is no way to read
        // // how much gas was actually used on callback.
        // let call_fee_used = u128::from(task.gas).saturating_mul(self.gas_price);
        // let call_total_fee = call_fee_used.saturating_add(self.agent_fee);
        // let call_total_balance = task.deposit.0.saturating_add(call_total_fee);

        // // safety check and not burn too much gas.
        // if call_total_balance > task.total_deposit.0 {
        //     log!("Not enough task balance to execute task, exiting");
        //     // Process task exit, if no future task can execute
        //     return self.exit_task(hash);
        // }

        // TODO: Bring this back!
        // // Update agent storage
        // // Increment agent reward & task count
        // // Reward for agent MUST include the amount of gas used as a reimbursement
        // agent.balance = U128::from(agent.balance.0.saturating_add(call_total_fee));
        // agent.total_tasks_executed = U128::from(agent.total_tasks_executed.0.saturating_add(1));
        // self.available_balance = self.available_balance.saturating_sub(call_total_fee);

        // TODO: Bring this back!
        // // Reset missed slot, if any
        // if agent.last_missed_slot != 0 {
        //     agent.last_missed_slot = 0;
        // }
        // self.agents.insert(&env::signer_account_id(), &agent);

        // TODO: Bring this back!
        // // Decrease task balance, Update task storage
        // task.total_deposit = U128::from(task.total_deposit.0.saturating_sub(call_total_balance));
        // self.tasks.insert(&hash, &task);

        // TODO: Move to external rule query handler
        // Proceed to query loops if rules are found in the task
        // Each rule is chained into the next, then evaluated if response is true before proceeding
        // let mut rule_responses: Vec<Attribute> = vec![];
        // if task.rules.is_some() {
        //     let mut rule_success: bool = false;
        //     // let mut previous_msg: Option<Binary>;
        //     for (idx, rule) in task.clone().rules.unwrap().iter().enumerate() {
        //         let rule_res: RuleResponse<Option<Binary>> = deps
        //             .querier
        //             .query_wasm_smart(&rule.contract_addr, &rule.msg)?;
        //         println!("{:?}", rule_res);
        //         rule_success = rule_res.0;

        //         // TODO: needs better approach
        //         d.push(Attribute::new(idx.to_string(), format!("{:?}", rule_res.1)));
        //     }
        //     if !rule_success {
        //         return Err(ContractError::CustomError {
        //             val: "Rule evaluated to false".to_string(),
        //         });
        //     }
        // }

        // Setup submessages for actions for this task
        // Each submessage in storage, computes & stores the "next" reply to allow for chained message processing.
        let mut sub_msgs: Vec<SubMsg<Empty>> = vec![];
        let next_idx = self.rq_next_id(deps.storage)?;
        let actions = task.clone().actions;
        let self_addr = env.contract.address;

        // Add submessages for all actions
        for action in actions {
            let sub_msg: SubMsg = SubMsg::reply_always(action.msg, next_idx);
            if let Some(gas_limit) = action.gas_limit {
                sub_msgs.push(sub_msg.with_gas_limit(gas_limit));
            } else {
                sub_msgs.push(sub_msg);
            }
        }

        // Keep track for later scheduling
        self.rq_push(
            deps.storage,
            QueueItem {
                prev_idx: None,
                task_hash: Some(hash),
                contract_addr: Some(self_addr),
            },
        )?;

        // TODO: Add supported msgs if not a SubMessage?
        // Add the messages, reply handler responsible for task rescheduling
        let final_res = Response::new()
            .add_attribute("method", "proxy_call")
            .add_attribute("agent", info.sender)
            .add_attribute("slot_id", slot_id.to_string())
            .add_attribute("slot_kind", format!("{:?}", SlotType::Block))
            .add_attribute("task_hash", task.to_hash())
            // .add_attributes(rule_responses)
            .add_submessages(sub_msgs);

        Ok(final_res)
    }

    /// Logic executed on the completion of a proxy call
    /// Reschedule next task
    pub(crate) fn proxy_callback(
        &self,
        deps: DepsMut,
        env: Env,
        msg: Reply,
        task_hash: Vec<u8>,
    ) -> Result<Response, ContractError> {
        let mut response = Response::new().add_attribute("method", "proxy_callback");

        // check if reply had failure
        let mut reply_submsg_failed = false;
        if let SubMsgResult::Ok(response) = &msg.result {
            for e in &response.events {
                for a in &e.attributes {
                    if e.ty == "reply" && a.key == "mode" && a.value == "handle_failure" {
                        reply_submsg_failed = true;
                    }
                }
            }
        } else {
            reply_submsg_failed = true;
        }

        // reschedule next!
        if let Some(task) = self.tasks.may_load(deps.storage, task_hash.clone())? {
            let task_hash_str = task.to_hash();
            // TODO: How can we compute gas & fees paid on this txn?
            // let out_of_funds = call_total_balance > task.total_deposit;

            // if non-recurring, exit
            if task.stop_on_fail && reply_submsg_failed {
                // Process task exit, if no future task can execute
                let rt = self.remove_task(deps, task_hash_str);
                if let Ok(..) = rt {
                    let resp = rt.unwrap();
                    response = response
                        .add_attributes(resp.attributes)
                        .add_submessages(resp.messages)
                        .add_events(resp.events);
                }
                return Ok(response);
            }

            // Parse interval into a future timestamp, then convert to a slot
            let (next_id, slot_kind) = task.interval.next(env, task.boundary);

            // If the next interval comes back 0, then this task should not schedule again
            if next_id == 0 {
                let rt = self.remove_task(deps, task_hash_str.clone());
                if let Ok(..) = rt {
                    let resp = rt.unwrap();
                    response = response
                        .add_attributes(resp.attributes)
                        .add_submessages(resp.messages)
                        .add_events(resp.events);
                }
                response = response.add_attribute("ended_task", task_hash_str);
                return Ok(response);
            } else {
                //If Send and reccuring task increment withdrawn funds so contract doesnt get drained
                let _transferred_bank_tokens = msg.transferred_bank_tokens();
                if task.contains_send_msg() && task.is_recurring() {
                    //task.funds_withdrawn_recurring.saturating_add(msg..funds[0].amount);
                    self.tasks.save(deps.storage, task_hash, &task)?;
                }
            }

            response = response.add_attribute("slot_id", next_id.to_string());
            response = response.add_attribute("slot_kind", format!("{:?}", slot_kind));

            // Get previous task hashes in slot, add as needed
            let update_vec_data = |d: Option<Vec<Vec<u8>>>| -> StdResult<Vec<Vec<u8>>> {
                match d {
                    // has some data, simply push new hash
                    Some(data) => {
                        let mut s = data;
                        s.push(task.to_hash_vec());
                        Ok(s)
                    }
                    // No data, push new vec & hash
                    None => Ok(vec![task.to_hash_vec()]),
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
        } else {
            return Err(ContractError::NoTaskFound {});
        }

        Ok(response)
    }

    /// Internal management of agent reward
    /// Used in cases where there are empty slots or failed txns
    /// Keep the agent profitable, as this will be a business expense
    pub(crate) fn send_base_agent_reward(
        &self,
        storage: &mut dyn Storage,
        mut agent: Agent,
        message: MessageInfo,
    ) {
        let mut config: Config = self.config.load(storage).unwrap();

        let agent_base_fee = config.agent_fee.clone();
        let coin = vec![agent_base_fee.clone()];
        let add_native: Balance = Balance::from(coin);

        agent.balance.add_tokens(add_native.clone());
        agent.total_tasks_executed = agent.total_tasks_executed.saturating_add(1);
        println!("{:?}", add_native);
        println!("{:?}", config.available_balance.native);

        if !config.available_balance.native.is_empty()
            && config.available_balance.native.first().unwrap().amount >= agent_base_fee.amount
        {
            config.available_balance.minus_tokens(add_native);
        }

        self.config
            .save(storage, &config)
            .expect("Could not save config");

        // Reset missed slot, if any
        if agent.last_missed_slot != 0 {
            agent.last_missed_slot = 0;
        }
        self.agents.save(storage, message.sender, &agent).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{
        coin, coins, to_binary, Addr, BankMsg, BlockInfo, CosmosMsg, Empty, StakingMsg, WasmMsg,
    };
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
    // use cw20::Balance;
    use crate::helpers::CwTemplateContract;
    use cw_croncat_core::msg::{ExecuteMsg, InstantiateMsg, TaskRequest};
    use cw_croncat_core::types::{Action, Boundary, Interval};

    pub fn contract_template() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::entry::execute,
            crate::entry::instantiate,
            crate::entry::query,
        )
        .with_reply(crate::entry::reply);
        Box::new(contract)
    }

    const ADMIN: &str = "cosmos1sjllsnramtg3ewxqwwrwjxfgc4n4ef9u0tvx7u";
    const ANYONE: &str = "cosmos1t5u0jfg3ljsjrh2m9e47d4ny2hea7eehxrzdgd";
    const AGENT0: &str = "cosmos1a7uhnpqthunr2rzj0ww0hwurpn42wyun6c5puz";
    const AGENT1_BENEFICIARY: &str = "cosmos1t5u0jfg3ljsjrh2m9e47d4ny2hea7eehxrzdgd";
    const NATIVE_DENOM: &str = "atom";

    fn mock_app() -> App {
        AppBuilder::new().build(|router, _, storage| {
            let accounts: Vec<(u128, String)> = vec![
                (6_000_000, ADMIN.to_string()),
                (500_000, ANYONE.to_string()),
                (2_000_000, AGENT0.to_string()),
                (2_000_000, AGENT1_BENEFICIARY.to_string()),
            ];
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
            denom: NATIVE_DENOM.to_string(),
            owner_id: Some(owner_addr.clone()),
            gas_base_fee: None,
            agent_nomination_duration: None,
        };
        let cw_template_contract_addr = app
            //Must send some available balance for rewards
            .instantiate_contract(
                cw_template_id,
                owner_addr,
                &msg,
                &coins(2_000_000, NATIVE_DENOM),
                "Manager",
                None,
            )
            .unwrap();

        let cw_template_contract = CwTemplateContract(cw_template_contract_addr);

        (app, cw_template_contract)
    }

    pub fn add_little_time(block: &mut BlockInfo) {
        // block.time = block.time.plus_seconds(360);
        block.time = block.time.plus_seconds(19);
        block.height += 1;
    }

    pub fn add_one_duration_of_time(block: &mut BlockInfo) {
        // block.time = block.time.plus_seconds(360);
        block.time = block.time.plus_seconds(420);
        block.height += 1;
    }

    #[test]
    fn proxy_call_fail_cases() -> StdResult<()> {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();
        let proxy_call_msg = ExecuteMsg::ProxyCall {};
        let validator = String::from("you");
        let amount = coin(3, NATIVE_DENOM);
        let stake = StakingMsg::Delegate { validator, amount };
        let msg: CosmosMsg = stake.clone().into();

        let create_task_msg = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: Some(Boundary::Height {
                    start: None,
                    end: None,
                }),
                stop_on_fail: false,
                actions: vec![Action {
                    msg,
                    gas_limit: Some(150_000),
                }],
                rules: None,
            },
        };
        let task_id_str =
            "95c916a53fa9d26deef094f7e1ee31c00a2d47b8bf474b2e06d39aebfb1fecc7".to_string();

        // Must attach funds
        let res_err = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &proxy_call_msg,
                &coins(300010, NATIVE_DENOM),
            )
            .unwrap_err();
        assert_eq!(
            ContractError::CustomError {
                val: "Must not attach funds".to_string()
            },
            res_err.downcast().unwrap()
        );

        // Create task paused
        let change_settings_msg = ExecuteMsg::UpdateSettings {
            paused: Some(true),
            owner_id: None,
            // treasury_id: None,
            agent_fee: None,
            min_tasks_per_agent: None,
            agents_eject_threshold: None,
            gas_price: None,
            proxy_callback_gas: None,
            slot_granularity: None,
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
                &proxy_call_msg,
                &vec![],
            )
            .unwrap_err();
        assert_eq!(
            ContractError::CustomError {
                val: "Contract paused".to_string()
            },
            res_err.downcast().unwrap()
        );
        // Set it back
        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &ExecuteMsg::UpdateSettings {
                paused: Some(false),
                owner_id: None,
                // treasury_id: None,
                agent_fee: None,
                min_tasks_per_agent: None,
                agents_eject_threshold: None,
                gas_price: None,
                proxy_callback_gas: None,
                slot_granularity: None,
            },
            &vec![],
        )
        .unwrap();

        // AgentNotRegistered
        let res_err = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &proxy_call_msg,
                &vec![],
            )
            .unwrap_err();
        assert_eq!(
            ContractError::AgentNotRegistered {},
            res_err.downcast().unwrap()
        );

        // quick agent register
        let msg = ExecuteMsg::RegisterAgent {
            payable_account_id: Some(Addr::unchecked(AGENT1_BENEFICIARY)),
        };
        app.execute_contract(Addr::unchecked(AGENT0), contract_addr.clone(), &msg, &[])
            .unwrap();

        // create task, so any slot actually exists
        let res = app
            .execute_contract(
                Addr::unchecked(ANYONE),
                contract_addr.clone(),
                &create_task_msg,
                &coins(300010, NATIVE_DENOM),
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

        // NoTasksForSlot
        let res_err = app
            .execute_contract(
                Addr::unchecked(AGENT0),
                contract_addr.clone(),
                &proxy_call_msg,
                &vec![],
            )
            .unwrap_err();
        assert_eq!(
            ContractError::CustomError {
                val: "No Tasks For Slot".to_string()
            },
            res_err.downcast().unwrap()
        );

        // NOTE: Unless there's a way to fake a task getting removed but hash remains in slot,
        // this coverage is not mockable. There literally shouldn't be any code that allows
        // this scenario to happen since all slot/task removal cases are covered
        // // delete the task so we test leaving an empty slot
        // app.execute_contract(
        //     Addr::unchecked(ANYONE),
        //     contract_addr.clone(),
        //     &ExecuteMsg::RemoveTask {
        //         task_hash: task_id_str.clone(),
        //     },
        //     &vec![],
        // )
        // .unwrap();

        // // NoTaskFound
        // let res_err = app
        //     .execute_contract(
        //         Addr::unchecked(AGENT0),
        //         contract_addr.clone(),
        //         &proxy_call_msg,
        //         &vec![],
        //     )
        //     .unwrap_err();
        // assert_eq!(
        //     ContractError::NoTaskFound {},
        //     res_err.downcast().unwrap()
        // );

        // TODO: TestCov: Task balance too small

        Ok(())
    }

    // TODO: TestCov: Agent balance updated (send_base_agent_reward)
    // TODO: TestCov: Total balance updated
    #[test]
    fn proxy_call_success() -> StdResult<()> {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();
        let proxy_call_msg = ExecuteMsg::ProxyCall {};
        let task_id_str =
            "dcbe1820cda5783a78afd66b68df4609c3fbce8e07f1f22c9585ae1ae5cf3289".to_string();

        // Doing this msg since its the easiest to guarantee success in reply
        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_binary(&ExecuteMsg::WithdrawReward {})?,
            funds: coins(1, NATIVE_DENOM),
        });

        let create_task_msg = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: Some(Boundary::Height {
                    start: None,
                    end: None,
                }),
                stop_on_fail: false,
                actions: vec![Action {
                    msg,
                    gas_limit: Some(250_000),
                }],
                rules: None,
            },
        };

        // create a task
        let res = app
            .execute_contract(
                Addr::unchecked(ADMIN),
                contract_addr.clone(),
                &create_task_msg,
                &coins(500010, NATIVE_DENOM),
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

        // quick agent register
        let msg = ExecuteMsg::RegisterAgent {
            payable_account_id: Some(Addr::unchecked(AGENT1_BENEFICIARY)),
        };
        app.execute_contract(Addr::unchecked(AGENT0), contract_addr.clone(), &msg, &[])
            .unwrap();
        app.execute_contract(
            Addr::unchecked(contract_addr.clone()),
            contract_addr.clone(),
            &msg,
            &[],
        )
        .unwrap();

        // might need block advancement?!
        app.update_block(add_little_time);

        // execute proxy_call
        let res = app
            .execute_contract(
                Addr::unchecked(AGENT0),
                contract_addr.clone(),
                &proxy_call_msg,
                &vec![],
            )
            .unwrap();
        let mut has_required_attributes: bool = true;
        let mut has_submsg_method: bool = false;
        let mut has_reply_success: bool = false;
        let attributes = vec![
            ("method", "proxy_call"),
            ("agent", AGENT0),
            ("slot_id", "12346"),
            ("slot_kind", "Block"),
            ("task_hash", task_id_str.as_str().clone()),
        ];

        // check all attributes are covered in response, and match the expected values
        for (k, v) in attributes.iter() {
            let mut attr_key: Option<String> = None;
            let mut attr_value: Option<String> = None;
            for e in res.clone().events {
                for a in e.attributes {
                    if e.ty == "wasm" && a.clone().key == k.to_string() && attr_key.is_none() {
                        attr_key = Some(a.clone().key);
                        attr_value = Some(a.clone().value);
                    }
                    if e.ty == "wasm"
                        && a.clone().key == "method"
                        && a.clone().value == "withdraw_agent_balance"
                    {
                        has_submsg_method = true;
                    }
                    if e.ty == "reply"
                        && a.clone().key == "mode"
                        && a.clone().value == "handle_success"
                    {
                        has_reply_success = true;
                    }
                }
            }

            // flip bool if none found, or value doesnt match
            if let Some(_key) = attr_key {
                if let Some(value) = attr_value {
                    if v.to_string() != value {
                        has_required_attributes = false;
                    }
                } else {
                    has_required_attributes = false;
                }
            } else {
                has_required_attributes = false;
            }
        }
        assert!(has_required_attributes);
        assert!(has_submsg_method);
        assert!(has_reply_success);

        Ok(())
    }

    #[test]
    fn proxy_callback_fail_cases() -> StdResult<()> {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();
        let proxy_call_msg = ExecuteMsg::ProxyCall {};
        let task_id_str =
            "96003a7938c1ac9566fec1be9b0cfa97a56626a574940ef5968364ef4d30c15a".to_string();

        // Doing this msg since its the easiest to guarantee success in reply
        let validator = String::from("you");
        let amount = coin(3, NATIVE_DENOM);
        let stake = StakingMsg::Delegate { validator, amount };
        let msg: CosmosMsg = stake.clone().into();

        let create_task_msg = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: Some(Boundary::Height {
                    start: None,
                    end: Some(12347_u64.into()),
                }),
                stop_on_fail: true,
                actions: vec![Action {
                    msg,
                    gas_limit: Some(250_000),
                }],
                rules: None,
            },
        };

        // create a task
        let res = app
            .execute_contract(
                Addr::unchecked(ADMIN),
                contract_addr.clone(),
                &create_task_msg,
                &coins(500010, NATIVE_DENOM),
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

        // quick agent register
        let msg = ExecuteMsg::RegisterAgent {
            payable_account_id: Some(Addr::unchecked(AGENT1_BENEFICIARY)),
        };
        app.execute_contract(Addr::unchecked(AGENT0), contract_addr.clone(), &msg, &[])
            .unwrap();
        app.execute_contract(
            Addr::unchecked(contract_addr.clone()),
            contract_addr.clone(),
            &msg,
            &[],
        )
        .unwrap();

        // might need block advancement?!
        app.update_block(add_little_time);

        // execute proxy_call - STOP ON FAIL
        let res = app
            .execute_contract(
                Addr::unchecked(AGENT0),
                contract_addr.clone(),
                &proxy_call_msg,
                &vec![],
            )
            .unwrap();
        let mut has_required_attributes: bool = true;
        let mut has_submsg_method: bool = false;
        let mut has_reply_success: bool = false;
        let attributes = vec![
            ("method", "remove_task"), // the last method
            ("slot_id", "12346"),
            ("slot_kind", "Block"),
            ("task_hash", task_id_str.as_str().clone()),
        ];

        // check all attributes are covered in response, and match the expected values
        for (k, v) in attributes.iter() {
            let mut attr_key: Option<String> = None;
            let mut attr_value: Option<String> = None;
            for e in res.clone().events {
                for a in e.attributes {
                    if e.ty == "wasm" && a.clone().key == k.to_string() {
                        attr_key = Some(a.clone().key);
                        attr_value = Some(a.clone().value);
                    }
                    if e.ty == "transfer"
                        && a.clone().key == "amount"
                        && a.clone().value == "500010atom"
                    {
                        has_submsg_method = true;
                    }
                    if e.ty == "reply"
                        && a.clone().key == "mode"
                        && a.clone().value == "handle_failure"
                    {
                        has_reply_success = true;
                    }
                }
            }

            // flip bool if none found, or value doesnt match
            if let Some(_key) = attr_key {
                if let Some(value) = attr_value {
                    if v.to_string() != value {
                        has_required_attributes = false;
                    }
                } else {
                    has_required_attributes = false;
                }
            } else {
                has_required_attributes = false;
            }
        }
        assert!(has_required_attributes);
        assert!(has_submsg_method);
        assert!(has_reply_success);

        // let task_id_str =
        //     "ce7f88df7816b4cf2d0cd882f189eb81ad66e4a9aabfc1eb5ba2189d73f9929b".to_string();

        // Doing this msg since its the easiest to guarantee success in reply
        let validator = String::from("you");
        let amount = coin(3, NATIVE_DENOM);
        let stake = StakingMsg::Delegate { validator, amount };
        let msg: CosmosMsg = stake.clone().into();

        let create_task_msg = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: Some(Boundary::Height {
                    start: None,
                    end: Some(12347_u64.into()),
                }),
                stop_on_fail: false,
                actions: vec![Action {
                    msg,
                    gas_limit: Some(250_000),
                }],
                rules: None,
            },
        };

        // create the task again
        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &create_task_msg,
            &coins(500010, NATIVE_DENOM),
        )
        .unwrap();

        // might need block advancement?!
        app.update_block(add_little_time);
        app.update_block(add_little_time);

        // execute proxy_call - TASK ENDED
        let res = app
            .execute_contract(
                Addr::unchecked(AGENT0),
                contract_addr.clone(),
                &proxy_call_msg,
                &vec![],
            )
            .unwrap();
        let mut has_required_attributes: bool = true;
        let mut has_submsg_method: bool = false;
        let mut has_reply_success: bool = false;
        let attributes = vec![
            ("method", "remove_task"), // the last method
            ("ended_task", task_id_str.as_str().clone()),
        ];

        // check all attributes are covered in response, and match the expected values
        for (k, v) in attributes.iter() {
            let mut attr_key: Option<String> = None;
            let mut attr_value: Option<String> = None;
            for e in res.clone().events {
                for a in e.attributes {
                    if e.ty == "wasm" && a.clone().key == k.to_string() {
                        attr_key = Some(a.clone().key);
                        attr_value = Some(a.clone().value);
                    }
                    if e.ty == "transfer"
                        && a.clone().key == "amount"
                        && a.clone().value == "500010atom"
                    {
                        has_submsg_method = true;
                    }
                    if e.ty == "reply"
                        && a.clone().key == "mode"
                        && a.clone().value == "handle_failure"
                    {
                        has_reply_success = true;
                    }
                }
            }

            // flip bool if none found, or value doesnt match
            if let Some(_key) = attr_key {
                if let Some(value) = attr_value {
                    if v.to_string() != value {
                        has_required_attributes = false;
                    }
                } else {
                    has_required_attributes = false;
                }
            } else {
                has_required_attributes = false;
            }
        }
        assert!(has_required_attributes);
        assert!(has_submsg_method);
        assert!(has_reply_success);

        Ok(())
    }

    #[test]
    fn proxy_callback_block_slots() -> StdResult<()> {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();
        let proxy_call_msg = ExecuteMsg::ProxyCall {};
        let task_id_str =
            "dcbe1820cda5783a78afd66b68df4609c3fbce8e07f1f22c9585ae1ae5cf3289".to_string();

        // Doing this msg since its the easiest to guarantee success in reply
        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_binary(&ExecuteMsg::WithdrawReward {})?,
            funds: coins(1, NATIVE_DENOM),
        });

        let create_task_msg = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: None,
                stop_on_fail: false,
                actions: vec![Action {
                    msg,
                    gas_limit: Some(250_000),
                }],
                rules: None,
            },
        };

        // create a task
        let res = app
            .execute_contract(
                Addr::unchecked(ADMIN),
                contract_addr.clone(),
                &create_task_msg,
                &coins(500010, NATIVE_DENOM),
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

        // quick agent register
        let msg = ExecuteMsg::RegisterAgent {
            payable_account_id: Some(Addr::unchecked(AGENT1_BENEFICIARY)),
        };
        app.execute_contract(Addr::unchecked(AGENT0), contract_addr.clone(), &msg, &[])
            .unwrap();
        app.execute_contract(
            Addr::unchecked(contract_addr.clone()),
            contract_addr.clone(),
            &msg,
            &[],
        )
        .unwrap();

        // might need block advancement?!
        app.update_block(add_little_time);

        // execute proxy_call
        let res = app
            .execute_contract(
                Addr::unchecked(AGENT0),
                contract_addr.clone(),
                &proxy_call_msg,
                &vec![],
            )
            .unwrap();
        let mut has_required_attributes: bool = true;
        let mut has_submsg_method: bool = false;
        let mut has_reply_success: bool = false;
        let attributes = vec![
            ("method", "proxy_callback"),
            ("slot_id", "12347"),
            ("slot_kind", "Block"),
            ("task_hash", task_id_str.as_str().clone()),
        ];

        // check all attributes are covered in response, and match the expected values
        for (k, v) in attributes.iter() {
            let mut attr_key: Option<String> = None;
            let mut attr_value: Option<String> = None;
            for e in res.clone().events {
                for a in e.attributes {
                    if e.ty == "wasm" && a.clone().key == k.to_string() {
                        attr_key = Some(a.clone().key);
                        attr_value = Some(a.clone().value);
                    }
                    if e.ty == "wasm"
                        && a.clone().key == "method"
                        && a.clone().value == "withdraw_agent_balance"
                    {
                        has_submsg_method = true;
                    }
                    if e.ty == "reply"
                        && a.clone().key == "mode"
                        && a.clone().value == "handle_success"
                    {
                        has_reply_success = true;
                    }
                }
            }

            // flip bool if none found, or value doesnt match
            if let Some(_key) = attr_key {
                if let Some(value) = attr_value {
                    if v.to_string() != value {
                        has_required_attributes = false;
                    }
                } else {
                    has_required_attributes = false;
                }
            } else {
                has_required_attributes = false;
            }
        }
        assert!(has_required_attributes);
        assert!(has_submsg_method);
        assert!(has_reply_success);

        Ok(())
    }

    #[test]
    fn proxy_callback_time_slots() -> StdResult<()> {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();
        let proxy_call_msg = ExecuteMsg::ProxyCall {};
        let task_id_str =
            "c7905cb9e5d620ae61b06cae6fb2bf3afa0ba0b290c1d48da626d0b7f68c293c".to_string();

        // Doing this msg since its the easiest to guarantee success in reply
        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_binary(&ExecuteMsg::WithdrawReward {})?,
            funds: coins(1, NATIVE_DENOM),
        });

        let create_task_msg = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Cron("0 * * * * *".to_string()),
                boundary: None,
                stop_on_fail: false,
                actions: vec![Action {
                    msg,
                    gas_limit: Some(250_000),
                }],
                rules: None,
            },
        };

        // create a task
        let res = app
            .execute_contract(
                Addr::unchecked(ADMIN),
                contract_addr.clone(),
                &create_task_msg,
                &coins(500010, NATIVE_DENOM),
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

        // quick agent register
        let msg = ExecuteMsg::RegisterAgent {
            payable_account_id: Some(Addr::unchecked(AGENT1_BENEFICIARY)),
        };
        app.execute_contract(Addr::unchecked(AGENT0), contract_addr.clone(), &msg, &[])
            .unwrap();
        app.execute_contract(
            Addr::unchecked(contract_addr.clone()),
            contract_addr.clone(),
            &msg,
            &[],
        )
        .unwrap();

        // might need block advancement?!
        app.update_block(add_one_duration_of_time);

        // execute proxy_call
        let res = app
            .execute_contract(
                Addr::unchecked(AGENT0),
                contract_addr.clone(),
                &proxy_call_msg,
                &vec![],
            )
            .unwrap();
        let mut has_required_attributes: bool = true;
        let mut has_submsg_method: bool = false;
        let mut has_reply_success: bool = false;
        let attributes = vec![
            ("method", "proxy_callback"),
            ("slot_id", "1571797860000000000"),
            ("slot_kind", "Cron"),
            ("task_hash", task_id_str.as_str().clone()),
        ];

        // check all attributes are covered in response, and match the expected values
        for (k, v) in attributes.iter() {
            let mut attr_key: Option<String> = None;
            let mut attr_value: Option<String> = None;
            for e in res.clone().events {
                for a in e.attributes {
                    if e.ty == "wasm" && a.clone().key == k.to_string() {
                        attr_key = Some(a.clone().key);
                        attr_value = Some(a.clone().value);
                    }
                    if e.ty == "wasm"
                        && a.clone().key == "method"
                        && a.clone().value == "withdraw_agent_balance"
                    {
                        has_submsg_method = true;
                    }
                    if e.ty == "reply"
                        && a.clone().key == "mode"
                        && a.clone().value == "handle_success"
                    {
                        has_reply_success = true;
                    }
                }
            }

            // flip bool if none found, or value doesnt match
            if let Some(_key) = attr_key {
                if let Some(value) = attr_value {
                    if v.to_string() != value {
                        has_required_attributes = false;
                    }
                } else {
                    has_required_attributes = false;
                }
            } else {
                has_required_attributes = false;
            }
        }
        assert!(has_required_attributes);
        assert!(has_submsg_method);
        assert!(has_reply_success);

        Ok(())
    }

    #[test]
    fn proxy_call_several_tasks() -> StdResult<()> {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();
        let proxy_call_msg = ExecuteMsg::ProxyCall {};

        // Doing this msg since its the easiest to guarantee success in reply
        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_binary(&ExecuteMsg::WithdrawReward {})?,
            funds: coins(1, NATIVE_DENOM),
        });

        let msg2 = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_binary(&ExecuteMsg::WithdrawReward {})?,
            funds: coins(2, NATIVE_DENOM),
        });

        let msg3 = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_binary(&ExecuteMsg::WithdrawReward {})?,
            funds: coins(3, NATIVE_DENOM),
        });

        let create_task_msg = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: None,
                stop_on_fail: false,
                actions: vec![Action {
                    msg,
                    gas_limit: Some(250_000),
                }],
                rules: None,
            },
        };

        let create_task_msg2 = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: None,
                stop_on_fail: false,
                actions: vec![Action {
                    msg: msg2,
                    gas_limit: Some(250_000),
                }],
                rules: None,
            },
        };

        let create_task_msg3 = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: None,
                stop_on_fail: false,
                actions: vec![Action {
                    msg: msg3,
                    gas_limit: Some(250_000),
                }],
                rules: None,
            },
        };

        // create two tasks in the same block
        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &create_task_msg,
            &coins(500_010, NATIVE_DENOM),
        )
        .unwrap();

        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &create_task_msg2,
            &coins(500_010, NATIVE_DENOM),
        )
        .unwrap();

        // the third task is created in another block
        app.update_block(add_little_time);

        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &create_task_msg3,
            &coins(500_010, NATIVE_DENOM),
        )
        .unwrap();

        // quick agent register
        let msg = ExecuteMsg::RegisterAgent {
            payable_account_id: Some(Addr::unchecked(AGENT1_BENEFICIARY)),
        };
        app.execute_contract(Addr::unchecked(AGENT0), contract_addr.clone(), &msg, &[])
            .unwrap();
        app.execute_contract(
            Addr::unchecked(contract_addr.clone()),
            contract_addr.clone(),
            &msg,
            &[],
        )
        .unwrap();

        // need block advancement
        app.update_block(add_little_time);

        // execute proxy_call's
        let res = app.execute_contract(
            Addr::unchecked(AGENT0),
            contract_addr.clone(),
            &proxy_call_msg,
            &vec![],
        );
        assert!(res.is_ok());

        let res = app.execute_contract(
            Addr::unchecked(AGENT0),
            contract_addr.clone(),
            &proxy_call_msg,
            &vec![],
        );
        assert!(res.is_ok());

        let res = app.execute_contract(
            Addr::unchecked(AGENT0),
            contract_addr.clone(),
            &proxy_call_msg,
            &vec![],
        );
        assert!(res.is_ok());
        Ok(())
    }

    #[test]
    fn check_bank_msg() {
        let (mut app, cw_template_contract) = proper_instantiate();
        let contract_addr = cw_template_contract.addr();

        let to_address = String::from("not_you");
        let amount = coin(3, "atom");
        let send = BankMsg::Send {
            to_address,
            amount: vec![amount],
        };
        let msg: CosmosMsg = send.clone().into();
        let gas_limit = 150_000;
        let agent_fee = 5;

        let create_task_msg = ExecuteMsg::CreateTask {
            task: TaskRequest {
                interval: Interval::Immediate,
                boundary: None,
                stop_on_fail: false,
                actions: vec![Action {
                    msg,
                    gas_limit: Some(gas_limit),
                }],
                rules: None,
            },
        };
        // create 1 token off task
        let amount_for_one_task = gas_limit + agent_fee;
        // create a task
        let res = app.execute_contract(
            Addr::unchecked(ANYONE),
            contract_addr.clone(),
            &create_task_msg,
            &coins(u128::from(amount_for_one_task * 2), "atom"),
        );
        assert!(res.is_ok());

        // quick agent register
        let msg = ExecuteMsg::RegisterAgent {
            payable_account_id: Some(Addr::unchecked(AGENT1_BENEFICIARY)),
        };
        app.execute_contract(Addr::unchecked(AGENT0), contract_addr.clone(), &msg, &[])
            .unwrap();

        app.update_block(add_little_time);

        app.execute_contract(
            Addr::unchecked(AGENT0),
            contract_addr.clone(),
            &ExecuteMsg::ProxyCall {},
            &[],
        )
        .unwrap();
    }
}
