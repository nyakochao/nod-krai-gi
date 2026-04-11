use crate::excel::ChallengeType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChallengeState {
    #[default]
    None,
    Active,
    Paused,
    Finished,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChallengeFinishReason {
    #[default]
    None,
    Success,
    Fail,
    Timeout,
    Interrupted,
}

#[derive(Debug, Clone)]
pub struct ActiveChallenge {
    pub is_active: bool,
    pub group_id: u32,
    pub source: u32,
    pub challenge_id: u32,
    param1: u32,
    param2: u32,
    param3: u32,
    param4: u32,
    pub challenge_index: u32,
    pub challenge_type: ChallengeType,
    pub state: ChallengeState,
    pub finish_reason: ChallengeFinishReason,
    pub time_cost: u32,    // s
    pub target_count: u32, // s
}

impl ActiveChallenge {
    pub fn new(
        group_id: u32,
        source: u32,
        challenge_id: u32,
        param1: u32,
        param2: u32,
        param3: u32,
        param4: u32,
        challenge_index: u32,
        challenge_type: ChallengeType,
    ) -> Self {
        let mut s = ActiveChallenge {
            is_active: false,
            source,
            group_id,
            challenge_id,
            param1,
            param2,
            param3,
            param4,
            challenge_index,
            challenge_type,
            state: ChallengeState::None,
            finish_reason: ChallengeFinishReason::None,
            time_cost: 0,
            target_count: 0,
        };
        s.group_id = s.get_real_group();
        s
    }

    pub fn start(&mut self) {
        self.state = ChallengeState::Active;
        self.is_active = true;
    }

    pub fn pause(&mut self) {
        if self.state == ChallengeState::Active {
            self.state = ChallengeState::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.state == ChallengeState::Paused {
            self.state = ChallengeState::Active;
        }
    }

    pub fn finish(&mut self, reason: ChallengeFinishReason) {
        self.state = ChallengeState::Finished;
        self.is_active = false;
        self.finish_reason = reason;
    }

    pub fn is_success(&self) -> bool {
        if self.state != ChallengeState::Active {
            return false;
        }
        match self.challenge_type {
            ChallengeType::ChallengeKillCount => self.target_count >= self.param2,
            ChallengeType::ChallengeKillCountInTime | ChallengeType::ChallengeKillCountFast => {
                self.target_count >= self.param3
            }
            ChallengeType::ChallengeSurvive | ChallengeType::ChallengeSurviveInTime => {
                self.time_cost >= self.param1
            }
            ChallengeType::ChallengeTimeFly => self.target_count >= self.param2,
            ChallengeType::ChallengeKillMonsterInTime => false,
            ChallengeType::ChallengeTriggerInTime => self.target_count >= self.param4,
            ChallengeType::ChallengeKillCountGuardHp => self.target_count >= self.param2,
            _ => false,
        }
    }

    pub fn is_failed(&self) -> bool {
        if self.state != ChallengeState::Active {
            return false;
        }
        match self.challenge_type {
            ChallengeType::ChallengeKillCount => false,
            ChallengeType::ChallengeKillCountInTime | ChallengeType::ChallengeKillCountFast => {
                self.time_cost > self.param1
            }
            ChallengeType::ChallengeSurvive | ChallengeType::ChallengeSurviveInTime => false,
            ChallengeType::ChallengeTimeFly => self.time_cost > self.param3,
            ChallengeType::ChallengeKillMonsterInTime => self.time_cost > self.param1,
            ChallengeType::ChallengeTriggerInTime => self.time_cost > self.param1,
            ChallengeType::ChallengeKillCountGuardHp => false,
            _ => false,
        }
    }

    pub fn get_time_progress(&self) -> Option<u32> {
        match self.challenge_type {
            ChallengeType::ChallengeKillCountInTime
            | ChallengeType::ChallengeKillCountFast
            | ChallengeType::ChallengeKillMonsterInTime
            | ChallengeType::ChallengeTriggerInTime => {
                Some(self.param1.saturating_sub(self.time_cost))
            }
            ChallengeType::ChallengeTimeFly => Some(self.param3.saturating_sub(self.time_cost)),
            ChallengeType::ChallengeSurvive | ChallengeType::ChallengeSurviveInTime => {
                Some(self.time_cost)
            }
            _ => None,
        }
    }

    pub fn add_time_limit(&mut self, delta: u32) {
        match self.challenge_type {
            ChallengeType::ChallengeKillCountInTime
            | ChallengeType::ChallengeKillCountFast
            | ChallengeType::ChallengeKillMonsterInTime
            | ChallengeType::ChallengeTriggerInTime => {
                self.param1 += delta;
            }
            ChallengeType::ChallengeTimeFly => self.param3 += delta,
            _ => {}
        }
    }

    pub fn get_real_group(&self) -> u32 {
        match self.challenge_type {
            ChallengeType::ChallengeKillCount
            | ChallengeType::ChallengeTimeFly
            | ChallengeType::ChallengeKillCountGuardHp => self.param1,
            ChallengeType::ChallengeKillCountInTime
            | ChallengeType::ChallengeKillCountFast
            | ChallengeType::ChallengeKillMonsterInTime => self.param2,
            _ => self.group_id,
        }
    }

    pub fn get_initial_param_list(&self) -> Vec<u32> {
        match self.challenge_type {
            ChallengeType::ChallengeKillCount => {
                vec![self.param2, self.param1]
            }
            ChallengeType::ChallengeKillCountInTime | ChallengeType::ChallengeKillCountFast => {
                vec![self.param3, self.param1]
            }
            ChallengeType::ChallengeSurvive | ChallengeType::ChallengeKillMonsterInTime => {
                vec![self.param1]
            }
            ChallengeType::ChallengeTimeFly => {
                vec![self.param2, self.param3, self.param4]
            }
            ChallengeType::ChallengeTriggerInTime => {
                vec![self.param1, self.param4]
            }
            ChallengeType::ChallengeKillCountGuardHp => {
                vec![self.param2, self.param3]
            }
            _ => vec![],
        }
    }
}
