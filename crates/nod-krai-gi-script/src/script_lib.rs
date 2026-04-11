use bevy_ecs::prelude::*;
use crossbeam_queue::SegQueue;
use mlua::{Function, Table};
pub(crate) use nod_krai_gi_data::scene::{LuaContext, LuaEvt, ScriptCommand};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub trait ScriptLib: Send + Sync + 'static {
    fn add_extra_group_suite(&self, ctx: Table, group_id: u32, suite_id: u32);
    fn remove_extra_group_suite(&self, ctx: Table, group_id: u32, suite_id: u32);

    // group variable methods
    fn get_group_variable_value(&self, group_id: u32, name: &str) -> i32;
    fn set_group_variable_value(&self, group_id: u32, name: &str, value: i32) -> i32;
    fn change_group_variable_value(&self, group_id: u32, name: &str, delta: i32) -> i32;

    // refresh group
    fn refresh_group(&self, group_id: u32, suite_id: u32);
    fn del_worktop_option_by_group_id(&self, group_id: u32, config_id: u32, option: u32);
    fn set_worktop_options_by_group_id(&self, group_id: u32, config_id: u32, option_list: Vec<u32>);

    // gadget state methods
    fn get_gadget_state_by_config_id(&self, uid: u32, group_id: u32, config_id: u32) -> i32;
    fn set_gadget_state_by_config_id(&self, uid: u32, group_id: u32, config_id: u32, state: u32);

    fn get_group_monster_count_by_config_id(&self, uid: u32, group_id: u32) -> i32;

    // challenge methods
    fn active_challenge(
        &self,
        group_id: u32,
        source: u32,
        challenge_id: u32,
        challenge_index: u32,
        param1: u32,
        param2: u32,
        param3: u32,
        param4: u32,
    );
    fn stop_challenge(&self, group_id: u32, challenge_index: u32, is_success: bool);
    fn add_challenge_progress(&self, group_id: u32, challenge_index: u32, progress: u32);
}

pub struct GroupVariableStore {
    // group_id -> (variable_name -> value)
    pub variables: RwLock<HashMap<u32, HashMap<String, i32>>>,
}

impl GroupVariableStore {
    pub fn new() -> Self {
        Self {
            variables: RwLock::new(HashMap::new()),
        }
    }

    pub fn init_group_variables(&self, group_id: u32, vars: HashMap<String, i32>) {
        self.variables.write().unwrap().insert(group_id, vars);
    }

    pub fn remove_group(&self, group_id: u32) {
        self.variables.write().unwrap().remove(&group_id);
    }

    pub fn get_variable(&self, group_id: u32, name: &str) -> i32 {
        self.variables
            .read()
            .unwrap()
            .get(&group_id)
            .and_then(|vars| vars.get(name))
            .copied()
            .unwrap_or(0)
    }

    pub fn set_variable(&self, group_id: u32, name: &str, value: i32) -> i32 {
        let mut store = self.variables.write().unwrap();
        if let Some(vars) = store.get_mut(&group_id) {
            if let Some(val) = vars.get_mut(name) {
                *val = value;
                return 0;
            }
        }
        -1
    }

    pub fn change_variable(&self, group_id: u32, name: &str, delta: i32) -> i32 {
        let mut store = self.variables.write().unwrap();
        if let Some(vars) = store.get_mut(&group_id) {
            if let Some(val) = vars.get_mut(name) {
                *val += delta;
                return 0;
            }
        }
        -1
    }

    pub fn reset_group_variables(&self, group_id: u32, vars: &[(String, i32)]) {
        let mut store = self.variables.write().unwrap();
        if let Some(group_vars) = store.get_mut(&group_id) {
            for (name, value) in vars {
                group_vars.insert(name.clone(), *value);
            }
        }
    }
}

#[derive(Resource, Clone)]
pub struct BevyScriptLib {
    pub queue: Arc<SegQueue<ScriptCommand>>,
    pub variable_store: Arc<GroupVariableStore>,
}

impl ScriptLib for BevyScriptLib {
    fn add_extra_group_suite(&self, ctx: Table, group_id: u32, suite_id: u32) {
        self.queue.push(ScriptCommand::AddExtraGroupSuite {
            ctx,
            group_id,
            suite_id,
        });
    }

    fn remove_extra_group_suite(&self, ctx: Table, group_id: u32, suite_id: u32) {
        self.queue.push(ScriptCommand::RemoveExtraGroupSuite {
            ctx,
            group_id,
            suite_id,
        });
    }

    fn get_group_variable_value(&self, group_id: u32, name: &str) -> i32 {
        self.variable_store.get_variable(group_id, name)
    }

    fn set_group_variable_value(&self, group_id: u32, name: &str, value: i32) -> i32 {
        self.variable_store.set_variable(group_id, name, value)
    }

    fn change_group_variable_value(&self, group_id: u32, name: &str, delta: i32) -> i32 {
        self.variable_store.change_variable(group_id, name, delta)
    }

    fn refresh_group(&self, group_id: u32, suite_id: u32) {
        self.queue
            .push(ScriptCommand::RefreshGroup { group_id, suite_id });
    }

    fn del_worktop_option_by_group_id(&self, group_id: u32, config_id: u32, option: u32) {
        self.queue.push(ScriptCommand::DelWorktopOptionByGroupId {
            group_id,
            config_id,
            option,
        });
    }

    fn set_worktop_options_by_group_id(
        &self,
        group_id: u32,
        config_id: u32,
        option_list: Vec<u32>,
    ) {
        self.queue.push(ScriptCommand::SetWorktopOptionsByGroupId {
            group_id,
            config_id,
            option_list,
        });
    }

    fn get_gadget_state_by_config_id(&self, uid: u32, group_id: u32, config_id: u32) -> i32 {
        nod_krai_gi_data::scene::group_entity_state_cache::get_group_entity_state_cache()
            .get_gadget_state(uid, group_id, config_id)
            .map(|s| s.gadget_state as i32)
            .unwrap_or(-1)
    }

    fn set_gadget_state_by_config_id(&self, _uid: u32, group_id: u32, config_id: u32, state: u32) {
        self.queue.push(ScriptCommand::SetGadgetStateByConfigId {
            group_id,
            config_id,
            state,
        });
    }

    fn get_group_monster_count_by_config_id(&self, uid: u32, group_id: u32) -> i32 {
        nod_krai_gi_data::scene::group_entity_state_cache::get_group_entity_state_cache()
            .get_alive_monster_count(uid, group_id) as i32
    }

    fn active_challenge(
        &self,
        group_id: u32,
        source: u32,
        challenge_id: u32,
        challenge_index: u32,
        param1: u32,
        param2: u32,
        param3: u32,
        param4: u32,
    ) {
        self.queue.push(ScriptCommand::ActiveChallenge {
            group_id,
            source,
            challenge_id,
            challenge_index,
            param1,
            param2,
            param3,
            param4,
        });
    }

    fn stop_challenge(&self, group_id: u32, challenge_index: u32, is_success: bool) {
        self.queue.push(ScriptCommand::StopChallenge {
            group_id,
            challenge_index,
            is_success,
        });
    }

    fn add_challenge_progress(&self, group_id: u32, challenge_index: u32, progress: u32) {
        self.queue.push(ScriptCommand::AddChallengeProgress {
            group_id,
            challenge_index,
            progress,
        });
    }
}

pub fn call_lua_trigger_condition(
    func: &Function,
    context: LuaContext,
    evt: LuaEvt,
) -> mlua::Result<bool> {
    match func.call((context, evt)) {
        Ok(ret) => Ok(ret),
        Err(e) => {
            tracing::debug!("Lua trigger function returned error: {:?}", e);
            Err(e)
        }
    }
}

pub fn call_lua_trigger_action(
    func: &Function,
    context: LuaContext,
    evt: LuaEvt,
) -> mlua::Result<i32> {
    match func.call((context, evt)) {
        Ok(ret) => Ok(ret),
        Err(e) => {
            tracing::debug!("Lua trigger function returned error: {:?}", e);
            Err(e)
        }
    }
}

pub fn call_lua_on_client_execute_req(
    func: &Function,
    context: LuaContext,
    param1: u32,
    param2: u32,
    param3: u32,
) -> mlua::Result<bool> {
    match func.call((context, param1, param2, param3)) {
        Ok(ret) => Ok(ret),
        Err(e) => {
            tracing::debug!("Lua trigger function returned error: {:?}", e);
            Err(e)
        }
    }
}

pub fn call_lua_on_be_hurt(
    func: &Function,
    context: LuaContext,
    param1: u32,
    param2: u32,
    param3: bool,
) -> mlua::Result<Option<bool>>{
    func.call((context, param1, param2, param3))
}
