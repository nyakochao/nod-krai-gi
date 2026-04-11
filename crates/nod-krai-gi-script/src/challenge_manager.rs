use bevy_ecs::prelude::*;
use nod_krai_gi_data::excel::ChallengeType;
use nod_krai_gi_data::scene::challenge::{ActiveChallenge, ChallengeFinishReason, ChallengeState};
use nod_krai_gi_data::scene::{EventType, LuaEvt};
use nod_krai_gi_event::lua::{
    ChallengeFinishEvent, ChallengeProgressEvent, ChallengeStartEvent, LuaTriggerEvent,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Resource, Default)]
pub struct ChallengeManager {
    pub active_challenges: HashMap<u32, ActiveChallenge>,
    pub challenge_index_counter: u32,
}

impl ChallengeManager {
    pub fn new() -> Self {
        Self {
            active_challenges: HashMap::new(),
            challenge_index_counter: 0,
        }
    }

    pub fn update_time_by_index(
        &mut self,
        index: u32,
        delta: u32,
    ) -> Option<ChallengeProgressEvent> {
        tracing::debug!("[ChallengeManager] Updating time {} by {}", index, delta);

        if let Some(challenge) = self.active_challenges.get_mut(&index) {
            challenge.add_time_limit(delta);

            tracing::debug!("[ChallengeManager] add_time_limit");

            let time_progress = challenge.get_time_progress();

            tracing::debug!("[ChallengeManager] time_progress {:?}", time_progress);

            match time_progress {
                Some(value) => {
                    return Some(ChallengeProgressEvent {
                        group_id: challenge.group_id,
                        challenge_index: index,
                        challenge_type: challenge.challenge_type,
                        param_index: 2,
                        value,
                    });
                }
                None => {}
            }
        }

        None
    }

    pub fn kill_monster(
        &mut self,
        challenge_index: u32,
        param: u32,
    ) -> Option<ChallengeProgressEvent> {
        match self.active_challenges.get_mut(&challenge_index) {
            None => None,
            Some(challenge) => {
                if challenge.state != ChallengeState::Active {
                    return None;
                }

                challenge.target_count += param;

                Some(ChallengeProgressEvent {
                    group_id: challenge.group_id,
                    challenge_index,
                    challenge_type: challenge.challenge_type,
                    param_index: 1,
                    value: challenge.target_count,
                })
            }
        }
    }

    pub fn get_next_challenge_index(&mut self) -> u32 {
        self.challenge_index_counter += 1;
        self.challenge_index_counter
    }

    pub fn start_challenge(&mut self, mut challenge: ActiveChallenge) -> ChallengeStartEvent {
        let index = challenge.challenge_index;
        let group_id = challenge.group_id;
        let challenge_id = challenge.challenge_id;
        let param_list = challenge.get_initial_param_list();
        challenge.start();
        self.active_challenges.insert(index, challenge);
        tracing::debug!(
            "[ChallengeManager] Started challenge {} with index {}, param_list: {:?}",
            challenge_id,
            index,
            param_list
        );
        ChallengeStartEvent {
            group_id,
            challenge_id,
            challenge_index: index,
            param_list,
        }
    }

    pub fn stop_challenge(
        &mut self,
        challenge_index: u32,
        is_success: bool,
    ) -> Option<(ActiveChallenge, ChallengeFinishEvent)> {
        if let Some(challenge) = self.active_challenges.get_mut(&challenge_index) {
            let reason = if is_success {
                ChallengeFinishReason::Success
            } else {
                ChallengeFinishReason::Fail
            };
            challenge.finish(reason);
            tracing::debug!(
                "[ChallengeManager] Stopped challenge {} with result: {:?}",
                challenge.challenge_id,
                reason
            );
        }
        self.active_challenges
            .remove(&challenge_index)
            .map(|challenge| {
                let event = ChallengeFinishEvent {
                    group_id: challenge.group_id,
                    challenge_id: challenge.challenge_id,
                    challenge_index,
                    is_success,
                    time_cost: challenge.time_cost,
                };
                (challenge, event)
            })
    }

    pub fn update_challenges(&mut self) -> Vec<ChallengeUpdateResult> {
        let mut results = Vec::new();

        let indices: Vec<u32> = self.active_challenges.keys().copied().collect();

        for index in indices {
            if let Some(challenge) = self.active_challenges.get_mut(&index) {
                if challenge.state != ChallengeState::Active {
                    continue;
                }

                results.push(self.update_challenges_by_index(index));
            }
        }

        results
    }

    pub fn update_challenges_by_index(&mut self, index: u32) -> ChallengeUpdateResult {
        if let Some(challenge) = self.active_challenges.get_mut(&index) {
            if challenge.is_success() {
                challenge.finish(ChallengeFinishReason::Success);
                return ChallengeUpdateResult::Success {
                    challenge_index: index,
                    group_id: challenge.group_id,
                    challenge_id: challenge.challenge_id,
                    time_cost: challenge.time_cost,
                };
            }

            if challenge.is_failed() {
                let reason = if challenge.is_failed() {
                    ChallengeFinishReason::Fail
                } else {
                    ChallengeFinishReason::Timeout
                };
                challenge.finish(reason);
                return ChallengeUpdateResult::Failed {
                    challenge_index: index,
                    group_id: challenge.group_id,
                    challenge_id: challenge.challenge_id,
                    time_cost: challenge.time_cost,
                };
            }
        }
        ChallengeUpdateResult::Running {}
    }

    pub fn remove_group_challenges(&mut self, group_id: u32) -> Vec<ChallengeFinishEvent> {
        let mut events = Vec::new();

        for (index, challenge) in self.active_challenges.iter() {
            if challenge.group_id == group_id {
                events.push(ChallengeFinishEvent {
                    group_id: challenge.group_id,
                    challenge_id: challenge.challenge_id,
                    challenge_index: *index,
                    is_success: false,
                    time_cost: challenge.time_cost,
                });
            }
        }

        self.active_challenges
            .retain(|_, challenge| challenge.group_id != group_id);

        events
    }

    pub fn get_challenge_for_monster_kill(&self, group_id: u32) -> Option<u32> {
        for (index, challenge) in &self.active_challenges {
            if challenge.group_id != group_id {
                continue;
            }

            let challenge_type = challenge.challenge_type;

            let matches = match challenge_type {
                ChallengeType::ChallengeKillCount
                | ChallengeType::ChallengeKillCountInTime
                | ChallengeType::ChallengeKillMonsterInTime => true,
                _ => false,
            };

            if matches {
                return Some(*index);
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub enum ChallengeUpdateResult {
    Running {},
    Success {
        challenge_index: u32,
        group_id: u32,
        challenge_id: u32,
        time_cost: u32,
    },
    Failed {
        challenge_index: u32,
        group_id: u32,
        challenge_id: u32,
        time_cost: u32,
    },
}

#[derive(Resource)]
pub struct ChallengeTimer {
    last_tick: Instant,
    interval: Duration,
}

impl Default for ChallengeTimer {
    fn default() -> Self {
        Self {
            last_tick: Instant::now(),
            interval: Duration::from_secs(1), // 1 秒
        }
    }
}

pub fn challenge_timer_system(
    mut timer: ResMut<ChallengeTimer>,
    mut challenge_manager: ResMut<ChallengeManager>,
    mut lua_trigger_events: MessageWriter<LuaTriggerEvent>,
    mut challenge_finish_events: MessageWriter<ChallengeFinishEvent>,
) {
    let now = Instant::now();
    if now.duration_since(timer.last_tick) >= timer.interval {
        timer.last_tick = now;
        {
            let results = challenge_manager.update_challenges();

            for result in results {
                match result {
                    ChallengeUpdateResult::Running { .. } => {}
                    ChallengeUpdateResult::Success {
                        challenge_index,
                        group_id,
                        challenge_id,
                        time_cost,
                    } => {
                        lua_trigger_events.write(LuaTriggerEvent {
                            group_id,
                            event_type: EventType::EventChallengeSuccess,
                            evt: LuaEvt {
                                ..Default::default()
                            },
                        });
                        lua_trigger_events.write(LuaTriggerEvent {
                            group_id,
                            event_type: EventType::EventDungeonSettle,
                            evt: LuaEvt {
                                param1: 1,
                                ..Default::default()
                            },
                        });
                        challenge_finish_events.write(ChallengeFinishEvent {
                            group_id,
                            challenge_id,
                            challenge_index,
                            is_success: true,
                            time_cost,
                        });
                    }
                    ChallengeUpdateResult::Failed {
                        challenge_index,
                        group_id,
                        challenge_id,
                        time_cost,
                    } => {
                        lua_trigger_events.write(LuaTriggerEvent {
                            group_id,
                            event_type: EventType::EventChallengeFail,
                            evt: LuaEvt {
                                ..Default::default()
                            },
                        });
                        lua_trigger_events.write(LuaTriggerEvent {
                            group_id,
                            event_type: EventType::EventDungeonSettle,
                            evt: LuaEvt {
                                param1: 0,
                                ..Default::default()
                            },
                        });
                        challenge_finish_events.write(ChallengeFinishEvent {
                            group_id,
                            challenge_id,
                            challenge_index,
                            is_success: false,
                            time_cost,
                        });
                    }
                }
            }
        }
    }
}
