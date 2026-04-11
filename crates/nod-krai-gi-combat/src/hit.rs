use bevy_ecs::prelude::*;
use nod_krai_gi_entity::common::{
    ConfigId, EntityById, FightProperties, GroupId, OwnerPlayerUID, ProtocolEntityID,
};
use nod_krai_gi_entity::gadget::GadgetID;
use nod_krai_gi_event::combat::*;
use nod_krai_gi_event::lua::OnBeHurtEvent;
use nod_krai_gi_event::scene::WorldVersionConfig;
use nod_krai_gi_proto::normal::ProtEntityType;

pub fn deal_damage_on_hit(
    index: Res<EntityById>,
    mut events: MessageReader<EntityBeingHitEvent>,
    mut entities: Query<(
        &mut FightProperties,
        &ProtocolEntityID,
        Option<&OwnerPlayerUID>,
        Option<&GroupId>,
        Option<&ConfigId>,
        Option<&GadgetID>,
    )>,
    mut on_be_hurt_events: MessageWriter<OnBeHurtEvent>,
    world_version_config: Res<WorldVersionConfig>,
) {
    for EntityBeingHitEvent(originator_uid, attack_result) in events.read() {
        let entity_type = attack_result.attacker_id >> world_version_config.ty_value;
        tracing::debug!("entity_type : {}", entity_type);
        if entity_type < ProtEntityType::ProtEntityMax as u32
            && entity_type != ProtEntityType::ProtEntityMpLevel as u32
        {
            let attacker_entity = match index.0.get(&attack_result.attacker_id) {
                Some(e) => *e,
                None => continue,
            };

            let Ok((_, _, attacker_owner, _, _, _)) = entities.get(attacker_entity) else {
                tracing::debug!("attacker with id {} not found", attack_result.attacker_id);
                continue;
            };

            if let Some(owner_uid) = attacker_owner {
                if owner_uid.0 != *originator_uid {
                    tracing::debug!(
                        "fail: entity owner uid mismatch! owner uid: {}, event originator uid: {}",
                        owner_uid.0,
                        originator_uid
                    );
                    continue;
                }
            }
        }

        let defense_entity = match index.0.get(&attack_result.defense_id) {
            Some(e) => *e,
            None => continue,
        };

        let Ok((mut defender_props, _, _, group_id_comp, config_id_comp, gadget_id_comp)) =
            entities.get_mut(defense_entity)
        else {
            tracing::debug!("defender with id {} not found", attack_result.defense_id);
            continue;
        };

        defender_props.change_cur_hp(-attack_result.damage);
        tracing::debug!(
            "attacker (id: {}) dealt {} dmg to defender (id: {})",
            attack_result.attacker_id,
            attack_result.damage,
            attack_result.defense_id
        );

        tracing::debug!("group_id_comp : {}", group_id_comp.is_some());
        tracing::debug!("config_id_comp : {}", config_id_comp.is_some());
        tracing::debug!("gadget_id_comp : {}", gadget_id_comp.is_some());

        let (group_id, config_id, gadget_id) = {
            // 任意一个是 none 退出 必须全部有
            if group_id_comp.is_none() || config_id_comp.is_none() || gadget_id_comp.is_none() {
                continue;
            }
            let group_id = group_id_comp.map(|t| t.0).unwrap_or(0);
            let config_id = config_id_comp.map(|t| t.0).unwrap_or(0);
            let gadget_id = gadget_id_comp.map(|t| t.0).unwrap_or(0);
            (group_id, config_id, gadget_id)
        };

        let Some(lua_name) =
            nod_krai_gi_data::custom::GadgetMapping::get_gadget_lua_name(gadget_id)
        else {
            tracing::debug!("no lua mapping found for gadget_id {}", gadget_id);
            continue;
        };

        tracing::debug!("lua_name: {}", lua_name);

        let lua_context = nod_krai_gi_data::scene::LuaContext {
            scene_id: 0,
            group_id,
            config_id,
            source_entity_id: attack_result.defense_id,
            target_entity_id: attack_result.defense_id,
            uid: *originator_uid,
        };

        on_be_hurt_events.write(OnBeHurtEvent {
            lua_name,
            element_type: attack_result.element_type,
            strike_type: 0,
            is_host: true,
            lua_context,
        });
    }
}
