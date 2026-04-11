use bevy_ecs::prelude::*;
use nod_krai_gi_data::scene::group_spatial_cache::get_or_init_spatial_cache;
use nod_krai_gi_data::scene::script_cache::SCENE_GROUP_COLLECTION;
use nod_krai_gi_data::scene::{EventType, Position, ScriptCommand};
use nod_krai_gi_event::combat::PlayerMoveEvent;
use nod_krai_gi_event::lua::{LuaTriggerEvent, ScriptCommandEvent};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

#[derive(Resource)]
pub struct GroupLoadManager {
    pub groups: HashSet<u32>,
    pub player_groups: HashMap<u32, HashSet<u32>>,
    pub last_load: HashMap<u32, Instant>,
    pub last_region_entry: HashMap<u32, Instant>,
    pub player_regions: HashMap<u32, HashSet<(u32, u32)>>,
    pub cd: Duration,
    pub region_entry_cd: Duration,
}

impl Default for GroupLoadManager {
    fn default() -> Self {
        Self {
            groups: HashSet::new(),
            player_groups: HashMap::new(),
            last_load: HashMap::new(),
            last_region_entry: HashMap::new(),
            player_regions: HashMap::new(),
            cd: Duration::from_millis(3000),
            region_entry_cd: Duration::from_millis(1000),
        }
    }
}

fn get_group_position(group_id: u32) -> (u32, Position) {
    let none_pos = Position {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let scene_group_collection_clone = std::sync::Arc::clone(SCENE_GROUP_COLLECTION.get().unwrap());

    let scene_block_collection_clone = std::sync::Arc::clone(
        nod_krai_gi_data::scene::script_cache::SCENE_BLOCK_COLLECTION
            .get()
            .unwrap(),
    );

    match scene_group_collection_clone.get(&group_id) {
        None => (0, none_pos),
        Some(scene_group_template) => match scene_group_template.value() {
            None => (0, none_pos),
            Some(scene_group_template) => {
                let scene_id = scene_group_template.base_info.scene_id;
                let block_id = scene_group_template.base_info.block_id;
                let Some(block) = scene_block_collection_clone.get(&(scene_id, block_id)) else {
                    return (0, none_pos);
                };
                let Some(block_group) = block.groups.iter().find(|g| g.id == group_id) else {
                    return (0, none_pos);
                };
                (scene_id, block_group.pos.clone())
            }
        },
    }
}

pub fn handle_player_move_for_group_loading(
    mut events: MessageReader<PlayerMoveEvent>,
    mut group_load_manager: ResMut<GroupLoadManager>,
    mut script_command_events: MessageWriter<ScriptCommandEvent>,
) {
    for ev in events.read() {
        tracing::trace!(
            "PlayerMoveEvent: uid={}, scene={}, pos={:?}",
            ev.0,
            ev.1,
            ev.2
        );

        let uid = ev.0;
        let scene_id = ev.1;
        let pos = Position::from(ev.2);
        let is_dungeon_load = ev.3;

        let now = Instant::now();

        let allow = is_dungeon_load
            || group_load_manager
                .last_load
                .get(&uid)
                .map(|t| now.duration_since(*t) >= group_load_manager.cd)
                .unwrap_or(true);

        if !allow {
            continue;
        }

        group_load_manager.last_load.insert(uid, now);

        let position = [pos.x, pos.y, pos.z];
        let max_squared_radius = if is_dungeon_load {
            4000.0f32 * 4000.0f32
        } else {
            1000.0f32 * 1000.0f32
        };
        let nearby_group_ids = if let Some(cache) =
            get_or_init_spatial_cache(scene_id, "./assets/lua", "./assets/cache")
        {
            cache.query_nearby_groups_rtree(position, max_squared_radius)
        } else {
            vec![]
        };

        let mut group_block_map: HashMap<u32, (u32, f32)> = HashMap::new();

        let spatial_cache = get_or_init_spatial_cache(scene_id, "./assets/lua", "./assets/cache");

        for group_id in &nearby_group_ids {
            if let Some(cache) = &spatial_cache {
                if let Some(group_info) = cache.scene_groups.get(group_id) {
                    group_block_map
                        .insert(*group_id, (group_info.block_id, group_info.vision_range));
                }
            }
        }

        let load_target: HashSet<u32> = nearby_group_ids
            .iter()
            .filter(|group_id| {
                if group_load_manager.groups.contains(*group_id) {
                    return false;
                }
                true
            })
            .copied()
            .collect();

        let mut other_player_groups: HashSet<u32> = HashSet::new();

        for (player_uid, player_group_set) in group_load_manager.player_groups.iter() {
            if *player_uid != uid {
                other_player_groups.extend(player_group_set);
            }
        }

        if !group_load_manager.player_groups.contains_key(&uid) {
            group_load_manager.player_groups.insert(uid, HashSet::new());
        }

        let unload_target: HashSet<u32> = group_load_manager
            .groups
            .iter()
            .filter(|group_id| {
                !other_player_groups.contains(*group_id) && {
                    let (this_scene_id, this_position) = get_group_position(**group_id);
                    if this_scene_id != scene_id {
                        return true;
                    }

                    if let Some(cache) = &spatial_cache {
                        if let Some(group_info) = cache.scene_groups.get(group_id) {
                            let dx = group_info.center[0] - pos.x;
                            let dy = group_info.center[1] - pos.y;
                            let dz = group_info.center[2] - pos.z;
                            let dist_squared = dx * dx + dy * dy + dz * dz;
                            let unload_distance = group_info.vision_range + 100.0;
                            let unload_distance_squared = unload_distance * unload_distance;
                            return dist_squared > unload_distance_squared;
                        }
                    }

                    let default_unload_distance = 180.0f32; // 80 + 100
                    this_position.distance_squared(&pos)
                        > default_unload_distance * default_unload_distance
                }
            })
            .map(|id| *id)
            .collect();

        for group_id in load_target {
            match group_block_map.get(&group_id) {
                None => {}
                Some((block_id, _vision_range)) => {
                    tracing::debug!(
                        ">>> Loading scene {} block {} group {} ",
                        scene_id,
                        block_id,
                        group_id,
                    );
                    group_load_manager.groups.insert(group_id);
                    group_load_manager
                        .player_groups
                        .get_mut(&uid)
                        .unwrap()
                        .insert(group_id);
                    script_command_events.write(ScriptCommandEvent {
                        command: ScriptCommand::LoadGroup {
                            scene_id,
                            block_id: *block_id,
                            group_id,
                        },
                    });
                }
            }
        }

        for group_id in unload_target {
            tracing::debug!(">>> Unloading group {}", group_id);
            group_load_manager.groups.remove(&group_id);
            group_load_manager
                .player_groups
                .get_mut(&uid)
                .unwrap()
                .remove(&group_id);
            script_command_events.write(ScriptCommandEvent {
                command: ScriptCommand::UnloadGroup { group_id },
            });
        }

        tracing::debug!("now groups {:?}", group_load_manager.groups)
    }
}

pub fn handle_player_move_for_region_entry(
    mut events: MessageReader<PlayerMoveEvent>,
    mut group_load_manager: ResMut<GroupLoadManager>,
    mut lua_trigger_events: MessageWriter<LuaTriggerEvent>,
) {
    for ev in events.read() {
        let uid = ev.0;
        let scene_id = ev.1;
        let pos = Position::from(ev.2);

        let now = Instant::now();

        let allow = group_load_manager
            .last_region_entry
            .get(&uid)
            .map(|t| now.duration_since(*t) >= group_load_manager.region_entry_cd)
            .unwrap_or(true);

        if !allow {
            continue;
        }

        group_load_manager.last_region_entry.insert(uid, now);

        let player_groups = group_load_manager.player_groups.get(&uid).cloned();
        let mut current_regions: HashSet<(u32, u32)> = HashSet::new();

        if let Some(player_groups) = player_groups {
            let scene_group_collection =
                std::sync::Arc::clone(SCENE_GROUP_COLLECTION.get().unwrap());
            for group_id in &player_groups {
                if let Some(scene_group_template) = scene_group_collection.get(group_id) {
                    if let Some(template) = scene_group_template.value() {
                        for region in &template.regions {
                            let region_dx = region.pos.x - pos.x;
                            let region_dy = region.pos.y - pos.y;
                            let region_dz = region.pos.z - pos.z;
                            let region_dist_squared = region_dx * region_dx
                                + region_dy * region_dy
                                + region_dz * region_dz;
                            let radius = region.radius.unwrap_or(5.0);
                            let radius_squared = radius * radius;
                            if region_dist_squared <= radius_squared {
                                current_regions.insert((*group_id, region.config_id));
                            }
                        }
                    }
                }
            }
        }

        let player_regions = group_load_manager.player_regions.entry(uid).or_default();

        for (group_id, region_config_id) in player_regions.iter() {
            if !current_regions.contains(&(*group_id, *region_config_id)) {
                tracing::debug!(
                    "Player {} left region {} in group {}",
                    uid,
                    region_config_id,
                    group_id
                );
                lua_trigger_events.write(LuaTriggerEvent {
                    group_id: *group_id,
                    event_type: EventType::EventLeaveRegion,
                    evt: nod_krai_gi_data::scene::LuaEvt {
                        param1: *region_config_id,
                        param2: *region_config_id,
                        param3: 0,
                        source_eid: 0,
                        target_eid: 0,
                    },
                });
            }
        }

        for (group_id, region_config_id) in current_regions.iter() {
            if !player_regions.contains(&(*group_id, *region_config_id)) {
                tracing::debug!(
                    "Player {} entered region {} in scene {} group {}",
                    uid,
                    region_config_id,
                    scene_id,
                    group_id
                );
                lua_trigger_events.write(LuaTriggerEvent {
                    group_id: *group_id,
                    event_type: EventType::EventEnterRegion,
                    evt: nod_krai_gi_data::scene::LuaEvt {
                        param1: *region_config_id,
                        param2: *region_config_id,
                        param3: 0,
                        source_eid: 0,
                        target_eid: 0,
                    },
                });
            }
        }

        *player_regions = current_regions;
    }
}
