use crate::common::PlayerSceneStates;
use bevy_ecs::prelude::*;
use common::player_cache::cache_set_is_tp;
use nod_krai_gi_event::scene::*;
use nod_krai_gi_message::output::MessageOutput;
use nod_krai_gi_proto::dy_parser::{replace_out_i32, replace_out_u32};
use nod_krai_gi_proto::retcode::Retcode;

pub fn on_post_enter_scene(
    mut reader: MessageReader<PostEnterSceneEvent>,
    player_scene_states: Res<PlayerSceneStates>,
    message_output: Res<MessageOutput>,
    world_version_config: Res<WorldVersionConfig>,
) {
    for PostEnterSceneEvent(uid) in reader.read() {
        cache_set_is_tp(*uid, false);

        let Some(player_scene_state) = player_scene_states.get(&uid) else {
            continue;
        };

        message_output.send(
            *uid,
            "PostEnterSceneRsp",
            nod_krai_gi_proto::normal::PostEnterSceneRsp {
                retcode: replace_out_i32(
                    world_version_config.protocol_version.as_str(),
                    "PostEnterSceneRsp.retcode",
                    Retcode::RetSucc.into(),
                ),
                enter_scene_token: replace_out_u32(
                    world_version_config.protocol_version.as_str(),
                    "PostEnterSceneRsp.enter_scene_token",
                    player_scene_state.enter_scene_token(),
                ),
            },
        );
    }
}
