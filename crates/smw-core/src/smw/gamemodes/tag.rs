//! Port of src/smw/gamemodes/Tag.cpp

use crate::game::Game;
use crate::common::eyecandy::{EC_GravText, EC_SingleAnimation};
use crate::common::game_mode::*;
use crate::common::game_values::if_sound_on_play;
use crate::common::global_constants::VELJUMP;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::*;

use crate::smw::net_random::{self, Ev};
use crate::smw::player::CPlayer;

pub struct CGM_Tag {
    pub cgame_mode: CGameMode,

    pub m_tagged: Ptr<CPlayer>,    /// Frames toward the next point; never reset between games, like the C++ function-local static.
    counter: i16,
}
impl_base!(CGM_Tag => cgame_mode: CGameMode);

/// The `j`/`k` loop shared by `think` and `playerkilledself`.
unsafe fn tag_warning_needed(g: &mut Game, goal: i16) -> bool {
    let mut countscore: i16 = 0;
    let mut playwarning = false;
    for j in 0..game().score_cnt {
        for k in 0..game().score_cnt {
            if j == k {
                continue;
            }

            countscore = (countscore as i32 + g.score[k as usize].score as i32) as i16;
        }

        if countscore as f64 <= goal as f64 * 0.2 {
            playwarning = true;
            break;
        }

        countscore = 0;
    }
    playwarning
}

//tag mode (leper mode suggestion from ziotok)
//one player is "it"
//if he killes another player, they become "it"
//the player that is "it" loses life until dead.
//the "it" player is chosen at random.  Someone is
//always "it".
impl CGM_Tag {
    fn reassign(&mut self, g: &mut Game) {
        let fGetHighest = !self.fReverseScoring;
        self.m_tagged = self.get_highest_score_player(g, fGetHighest);
    }

    pub fn new() -> Self {
        let mut this = CGM_Tag { cgame_mode: CGameMode::new(), m_tagged: Ptr::null(), counter: 0 };
        this.goal = 200;
        this.gamemode = game_mode_tag;

        this.setup_mode_strings("Tag", "Points", 50);
        this
    }

    pub fn tagged(&self) -> Ptr<CPlayer> {
        self.m_tagged
    }

    pub fn set_tagged(&mut self, player: Ptr<CPlayer>) {
        self.m_tagged = player;
    }
}

impl CGameModeTrait for CGM_Tag {
    crate::impl_cgamemode_plumbing!();

    fn init(&mut self, g: &mut Game) {
        cgamemode_init(g, self);
        self.fReverseScoring = self.goal == -1;

        unsafe {
            for iScore in 0..game().score_cnt {
                if self.fReverseScoring {
                    g.score[iScore as usize].set_score(0);
                } else {
                    g.score[iScore as usize].set_score(self.goal);
                }
            }
        }

        self.set_tagged(Ptr::null());
    }

    fn think(&mut self, g: &mut Game) {
        if self.gameover {
            self.displayplayertext(g);
        } else {
            if self.m_tagged.is_null() && !net_random::event(g, Ev::TagReassign, &[]) {
                self.reassign(g);
            }
            if self.m_tagged.is_null() {
                return;
            }

            let mut tagged = self.m_tagged;
            if tagged.isready() {
                unsafe {
                    self.counter += 1;
                    if self.counter >= g.game_values.pointspeed {
                        self.counter = 0;

                        if self.fReverseScoring {
                            tagged.score().adjust_score(g, 1);
                        } else {
                            tagged.score().adjust_score(g, -1);
                        }
                    }
                }
            }

            if self.fReverseScoring {
                return;
            }

            let playwarning = unsafe { tag_warning_needed(g, self.goal) };

            if playwarning && !self.playedwarningsound {
                self.playwarningsound(g);
            }

            let mut tagged = self.m_tagged;
            if tagged.score().score <= 0 {
                remove_team(g, tagged.get_team_id());
                self.m_tagged = Ptr::null();
            }
        }
    }

    fn playerkilledplayer(&mut self, g: &mut Game, mut inflictor: Ptr<CPlayer>, mut other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        unsafe {
            if inflictor == self.m_tagged {
                self.m_tagged = other;
                inflictor.shield().reset(g);
                g.eyecandy[2].emplace(EC_GravText::new(
                    Ptr::from_mut(&mut g.rm.game_font_large),
                    other.center_x(),
                    other.bottom_y(),
                    "Tagged!".to_string(),
                    (-(VELJUMP as f64) * 1.5) as f32,
                ));
                g.eyecandy[2].emplace(EC_SingleAnimation::new(
                    Ptr::from_mut(&mut g.rm.spr_fireballexplosion),
                    (other.center_x() as i32 - 16) as i16,
                    (other.center_y() as i32 - 16) as i16,
                    3,
                    8,
                ));
                if_sound_on_play(g, &mut game().rm.sfx_transform);
            }

            if !self.gameover {
                if self.fReverseScoring {
                    other.score().adjust_score(g, 5);
                    return PlayerKillType::Normal;
                } else {
                    other.score().adjust_score(g, -5);
                }

                let mut countscore: i16 = 0;
                for k in 0..game().score_cnt {
                    if Ptr::from_mut(inflictor.score()) == g.score[k as usize] {
                        continue;
                    }

                    countscore = (countscore as i32 + g.score[k as usize].score as i32) as i16;
                }

                if countscore as f64 <= self.goal as f64 * 0.2 && !self.playedwarningsound {
                    self.playwarningsound(g);
                }

                if other.score().score <= 0 {
                    other.score().set_score(0);

                    remove_team(g, other.get_team_id());
                    return PlayerKillType::Removed;
                }
            }
        }

        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, g: &mut Game, mut player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        cgamemode_playerkilledself(self, player, style);

        if !self.gameover {
            if self.fReverseScoring {
                player.score().adjust_score(g, 5);
                return PlayerKillType::Normal;
            } else {
                player.score().adjust_score(g, -5);
            }

            let playwarning = unsafe { tag_warning_needed(g, self.goal) };

            if playwarning && !self.playedwarningsound {
                self.playwarningsound(g);
            }

            if player.score().score <= 0 {
                player.score().set_score(0);
                remove_team(g, player.get_team_id());
                return PlayerKillType::Removed;
            }
        }

        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, g: &mut Game, mut player: Ptr<CPlayer>, iType: i16) {
        if !self.gameover {
            if self.fReverseScoring {
                player.score().adjust_score(g, (-10 * iType as i32) as i16);
            } else {
                player.score().adjust_score(g, (10 * iType as i32) as i16);
            }
        }
    }
}

fn tag_mode() -> Option<&'static mut CGM_Tag> {
    unsafe { game().game_values.gamemode.get().as_any().downcast_mut::<CGM_Tag>() }
}

pub fn net_reassign() {
    if let Some(this) = tag_mode() {
        if this.m_tagged.is_null() {
            this.reassign(game().get());
        }
    }
}

pub fn net_state() -> String {
    match tag_mode() {
        Some(this) if !this.m_tagged.is_null() => format!("tagged{}", this.m_tagged.globalID),
        _ => "tagged-".to_string(),
    }
}
