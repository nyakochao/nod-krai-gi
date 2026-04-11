use bevy_ecs::entity::Entity;
use bevy_ecs::message::Message;

#[derive(Message)]
pub struct GadgetInteractEvent(pub u32, pub u32, pub u32);

#[derive(Message)]
pub struct GadgetStateChangeEvent {
    pub entity: Entity,
    pub state_id: u32,
    pub previous_state_id: Option<u32>,
}

#[derive(Message)]
pub struct SetWorktopOptionsEvent {
    pub player_uid: u32,
    pub group_id: u32,
    pub config_id: u32,
    pub gadget_entity_id: u32,
    pub option_list: Vec<u32>,
    pub del_option: u32,
}
