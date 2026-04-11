use crate::common::PlayerSceneStates;
use bevy_ecs::prelude::*;
use nod_krai_gi_entity::{
    avatar::{AvatarQueryReadOnly, CurrentPlayerAvatarMarker},
    common::Visible,
};
use nod_krai_gi_event::scene::*;
use nod_krai_gi_message::output::MessageOutput;
use nod_krai_gi_proto::dy_parser::{replace_out_i32, replace_out_u32};
use nod_krai_gi_proto::normal::EnterSceneDoneRsp;
use nod_krai_gi_proto::retcode::Retcode;

pub fn on_enter_scene_done(
    mut commands: Commands,
    mut reader: MessageReader<EnterSceneDoneEvent>,
    avatars: Query<(Entity, AvatarQueryReadOnly), With<CurrentPlayerAvatarMarker>>,
) {
    for event in reader.read() {
        let uid = event.0;

        let Some((cur_player_avatar, _)) = avatars
            .iter()
            .find(|(_, data)| data.owner_player_uid.0 == uid)
        else {
            tracing::error!("cur_player_avatar None");
            continue;
        };

        commands.entity(cur_player_avatar).insert(Visible);
    }
}

pub fn enter_scene_done_send_rsp(
    mut enter_scene_done_events: MessageReader<EnterSceneDoneEvent>,
    player_scene_states: Res<PlayerSceneStates>,
    message_output: Res<MessageOutput>,
    world_version_config: Res<WorldVersionConfig>,
) {
    for event in enter_scene_done_events.read() {
        let uid = event.0;

        let Some(player_scene_state) = player_scene_states.get(&uid) else {
            continue;
        };

        message_output.send(
            uid,
            "EnterSceneDoneRsp",
            EnterSceneDoneRsp {
                retcode: replace_out_i32(
                    world_version_config.protocol_version.as_str(),
                    "EnterSceneDoneRsp.retcode",
                    Retcode::RetSucc.into(),
                ),
                enter_scene_token: replace_out_u32(
                    world_version_config.protocol_version.as_str(),
                    "EnterSceneDoneRsp.enter_scene_token",
                    player_scene_state.enter_scene_token(),
                ),
            },
        );
    }
}
