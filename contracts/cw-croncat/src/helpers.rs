use crate::state::Config;
use crate::ContractError::AgentNotRegistered;
use crate::{ContractError, CwCroncat};
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Coin, CosmosMsg, Env, StdResult, Storage, SubMsg, SubMsgResult,
    WasmMsg,
};
use cw20::{Cw20CoinVerified, Cw20ExecuteMsg};
use cw_croncat_core::msg::ExecuteMsg;
use cw_croncat_core::types::AgentStatus;
pub use cw_croncat_core::types::{GenericBalance, Task};
//use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cmp;
use std::ops::Div;
//use std::str::FromStr;
pub(crate) fn vect_difference<T: std::clone::Clone + std::cmp::PartialEq>(
    v1: &[T],
    v2: &[T],
) -> Vec<T> {
    v1.iter().filter(|&x| !v2.contains(x)).cloned().collect()
}

// pub(crate) fn from_raw_str(value: &str) -> Option<Coin> {
//     let re = Regex::new(r"^([0-9.]+)([a-z][a-z0-9]*)$").unwrap();
//     assert!(re.is_match(value));
//     let caps = re.captures(value)?;
//     let amount = caps.get(1).map_or("", |m| m.as_str());
//     let denom = caps.get(2).map_or("", |m| m.as_str());
//     assert!(denom.len() < 3 || denom.len() > 128);
//     Some(Coin::new(u128::from_str(amount).unwrap(), denom))
// }

// Helper to distribute funds/tokens
pub(crate) fn send_tokens(
    to: &Addr,
    balance: &GenericBalance,
) -> StdResult<(Vec<SubMsg>, GenericBalance)> {
    let native_balance = &balance.native;
    let mut coins: GenericBalance = GenericBalance::default();
    let mut msgs: Vec<SubMsg> = if native_balance.is_empty() {
        vec![]
    } else {
        coins.native = balance.native.clone();
        vec![SubMsg::new(BankMsg::Send {
            to_address: to.into(),
            amount: native_balance.to_vec(),
        })]
    };

    let cw20_balance = &balance.cw20;
    let cw20_msgs: StdResult<Vec<_>> = cw20_balance
        .iter()
        .map(|c| {
            let msg = Cw20ExecuteMsg::Transfer {
                recipient: to.into(),
                amount: c.amount,
            };
            let exec = SubMsg::new(WasmMsg::Execute {
                contract_addr: c.address.to_string(),
                msg: to_binary(&msg)?,
                funds: vec![],
            });
            Ok(exec)
        })
        .collect();
    coins.cw20 = balance.cw20.clone();
    msgs.append(&mut cw20_msgs?);
    Ok((msgs, coins))
}

/// has_cw_coins returns true if the list of CW20 coins has at least the required amount
pub(crate) fn has_cw_coins(coins: &[Cw20CoinVerified], required: &Cw20CoinVerified) -> bool {
    coins
        .iter()
        .find(|c| c.address == required.address)
        .map(|m| m.amount >= required.amount)
        .unwrap_or(false)
}
pub trait ReplyMsgParser {
    fn transferred_bank_tokens(&self) -> Vec<cosmwasm_std::Coin>;
}
impl ReplyMsgParser for cosmwasm_std::Reply {
    fn transferred_bank_tokens(&self) -> Vec<cosmwasm_std::Coin> {
        if let SubMsgResult::Ok(res) = &self.result {
            res.events
                .iter()
                .filter(|ev| ev.ty == "transfer")
                .flat_map(|ev| {
                    ev.attributes
                        .iter()
                        .filter_map(|attr| {
                            attr.key.eq("amount").then(|| {
                                // I really don't want to put regex here, it's gonna increase binary size way too much
                                let n = attr.value.chars().position(|c| c.is_alphabetic()).unwrap();
                                let (amount, denom) = attr.value.split_at(n);
                                Coin {
                                    amount: amount.parse().unwrap(),
                                    denom: denom.to_owned(),
                                }
                            })
                        })
                        .collect::<Vec<Coin>>()
                })
                .collect()
        } else {
            Vec::new()
        }
    }
}

impl<'a> CwCroncat<'a> {
    pub fn get_agent_status(
        &self,
        storage: &dyn Storage,
        env: Env,
        account_id: Addr,
    ) -> Result<AgentStatus, ContractError> {
        let c: Config = self.config.load(storage)?;
        let block_time = env.block.time.seconds();
        // Check for active
        let active = self.agent_active_queue.load(storage)?;
        if active.contains(&account_id) {
            return Ok(AgentStatus::Active);
        }

        // Pending
        let pending: Vec<Addr> = self.agent_pending_queue.load(storage)?;
        // If agent is pending, Check if they should get nominated to checkin to become active
        let agent_status: AgentStatus = if pending.contains(&account_id) {
            // Load config's task ratio, total tasks, active agents, and agent_nomination_begin_time.
            // Then determine if this agent is considered "Nominated" and should call CheckInAgent
            let min_tasks_per_agent = c.min_tasks_per_agent;
            let total_tasks = self
                .task_total(storage)
                .expect("Unexpected issue getting task total");
            let num_active_agents = self.agent_active_queue.load(storage).unwrap().len() as u64;
            let agent_position = pending
                .iter()
                .position(|address| address == &account_id)
                .unwrap();

            // If we should allow a new agent to take over
            let num_agents_to_accept =
                self.agents_to_let_in(&min_tasks_per_agent, &num_active_agents, &total_tasks);
            let agent_nomination_begin_time = self.agent_nomination_begin_time.load(storage)?;
            match agent_nomination_begin_time {
                Some(begin_time) if num_agents_to_accept > 0 => {
                    let time_difference = block_time - begin_time.seconds();

                    let max_index = cmp::max(
                        time_difference.div(c.agent_nomination_duration as u64),
                        num_agents_to_accept - 1,
                    );
                    if agent_position as u64 <= max_index {
                        AgentStatus::Nominated
                    } else {
                        AgentStatus::Pending
                    }
                }
                _ => {
                    // Not their time yet
                    AgentStatus::Pending
                }
            }
        } else {
            // This should not happen. It means the address is in self.agents
            // but not in the pending or active queues
            // Note: if your IDE highlights the below as problematic, you can ignore
            return Err(AgentNotRegistered {});
        };
        Ok(agent_status)
    }

    pub fn agents_to_let_in(
        &self,
        max_tasks: &u64,
        num_active_agents: &u64,
        total_tasks: &u64,
    ) -> u64 {
        let num_tasks_covered = num_active_agents * max_tasks;
        if total_tasks > &num_tasks_covered {
            // It's possible there are more "covered tasks" than total tasks,
            // so use saturating subtraction to hit zero and not go below
            let total_tasks_needing_agents = total_tasks.saturating_sub(num_tasks_covered);
            let remainder = if total_tasks_needing_agents % max_tasks == 0 {
                0
            } else {
                1
            };
            total_tasks_needing_agents / max_tasks + remainder
        } else {
            0
        }
    }
}

/// CwTemplateContract is a wrapper around Addr that provides a lot of helpers
/// for working with this.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CwTemplateContract(pub Addr);

impl CwTemplateContract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
        let msg = to_binary(&msg.into())?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds: vec![],
        }
        .into())
    }

    // /// Get Count
    // pub fn count<Q, T, CQ>(&self, querier: &Q) -> StdResult<CountResponse>
    // where
    //     Q: Querier,
    //     T: Into<String>,
    //     CQ: CustomQuery,
    // {
    //     let msg = QueryMsg::GetCount {};
    //     let query = WasmQuery::Smart {
    //         contract_addr: self.addr().into(),
    //         msg: to_binary(&msg)?,
    //     }
    //     .into();
    //     let res: CountResponse = QuerierWrapper::<CQ>::new(querier).query(&query)?;
    //     Ok(res)
    // }
}

#[cfg(test)]
pub mod test_helpers {
    use cosmwasm_std::{
        coins,
        testing::{mock_env, mock_info},
        DepsMut, Empty, Response, StdResult,
    };
    use cw_croncat_core::msg::InstantiateMsg;

    use crate::CwCroncat;

    pub fn mock_init(store: &CwCroncat, deps: DepsMut<Empty>) -> StdResult<Response> {
        let msg = InstantiateMsg {
            denom: "atom".to_string(),
            owner_id: None,
            gas_base_fee: None,
            agent_nomination_duration: Some(360),
        };
        let info = mock_info("creator", &coins(1000, "meow"));
        store.instantiate(deps, mock_env(), info.clone(), msg)
    }
}
