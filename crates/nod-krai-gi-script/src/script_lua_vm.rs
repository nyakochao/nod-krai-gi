use crate::script_lib::BevyScriptLib;
use crate::script_lib_handle::LuaScriptLibHandle;
use bevy_ecs::prelude::*;
use mlua::Lua;
use nod_krai_gi_data::scene::script_cache::SCENE_LUA_VM;
use std::sync::Arc;

#[derive(Resource, Clone)]
pub struct LuaRuntime {
    pub lua: Lua,
}

impl LuaRuntime {
    pub fn new(script_lib: Arc<BevyScriptLib>, protocol_version: String) -> Self {
        let lua = get_lua(script_lib, protocol_version);

        Self { lua }
    }
}

pub fn get_lua(script_lib: Arc<BevyScriptLib>, protocol_version: String) -> Lua {
    let lua = SCENE_LUA_VM.get().unwrap().clone();

    let globals = lua.globals();

    let lib_handle = LuaScriptLibHandle {
        script_lib,
        protocol_version,
    };
    let result = globals.set("ScriptLib", lib_handle);

    match result {
        Ok(_) => {}
        Err(err) => {
            tracing::debug!("SceneGroupRuntime init_group_lua_vm  fail {}", err);
            std::process::exit(0);
        }
    }

    let gadget_dir = std::path::Path::new("./assets/lua/gadget");

    if let Ok(entries) = std::fs::read_dir(gadget_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("lua") {
                if let Ok(code) = common::string_util::read_utf8_no_bom(&path) {
                    let code = code.replace("ScriptLib.", "ScriptLib:");
                    let script_name = path.file_name().unwrap().to_string_lossy().to_string();
                    tracing::debug!("Loading gadget lua: {}", script_name.clone());

                    let env = lua.create_table().unwrap();
                    let mt = lua.create_table().unwrap();
                    mt.set("__index", globals.clone()).unwrap();
                    env.set_metatable(Some(mt)).unwrap();

                    let chunk = lua.load(&code).set_name(script_name.clone());
                    chunk.set_environment(env.clone()).exec().unwrap();

                    lua.globals().set(script_name.clone(), env).unwrap();
                }
            }
        }
    } else {
        tracing::error!("Failed to read gadget lua directory");
    }

    lua
}
