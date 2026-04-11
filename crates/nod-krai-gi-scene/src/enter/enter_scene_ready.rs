use crate::common::{PlayerSceneStates, ScenePeerManager};
use bevy_ecs::prelude::*;
use nod_krai_gi_event::scene::*;
use nod_krai_gi_message::output::MessageOutput;
use nod_krai_gi_persistence::Players;
use nod_krai_gi_proto::dy_parser::{replace_out_i32, replace_out_u32};
use nod_krai_gi_proto::retcode::Retcode;

pub fn on_enter_scene_ready(
    mut reader: MessageReader<EnterSceneReadyEvent>,
    message_output: Res<MessageOutput>,
    player_scene_states: Res<PlayerSceneStates>,
    players: Res<Players>,
    mut peer_manager: ResMut<ScenePeerManager>,
    world_version_config: Res<WorldVersionConfig>,
) {
    for event in reader.read() {
        let uid = event.0;
        let Some(player_info) = players.get(uid) else {
            continue;
        };
        let Some(ref player_scene_bin) = player_info.scene_bin else {
            continue;
        };

        let Some(player_scene_state) = player_scene_states.get(&uid) else {
            continue;
        };

        let enter_scene_token = player_scene_state.enter_scene_token();

        let peer_id = peer_manager.get_or_add_peer(uid);
        if peer_manager.peer_count() == 1 {
            peer_manager.make_host(peer_id);
        }

        message_output.send(
            uid,
            "EnterScenePeerNotify",
            nod_krai_gi_proto::normal::EnterScenePeerNotify {
                enter_scene_token: replace_out_u32(
                    world_version_config.protocol_version.as_str(),
                    "EnterScenePeerNotify.enter_scene_token",
                    enter_scene_token,
                ),
                peer_id,
                host_peer_id: peer_manager.host_peer_id(),
                dest_scene_id: player_scene_bin.my_cur_scene_id,
            },
        );

        message_output.send(
            uid,
            "EnterSceneReadyRsp",
            nod_krai_gi_proto::normal::EnterSceneReadyRsp {
                retcode: replace_out_i32(
                    world_version_config.protocol_version.as_str(),
                    "EnterSceneReadyRsp.retcode",
                    Retcode::RetSucc.into(),
                ),
                enter_scene_token: replace_out_u32(
                    world_version_config.protocol_version.as_str(),
                    "EnterSceneReadyRsp.enter_scene_token",
                    enter_scene_token,
                ),
            },
        );
    }
}
