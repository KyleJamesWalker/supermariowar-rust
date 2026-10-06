//! Port of src/smw/gamemodes/Chicken.cpp

use crate::common::eyecandy::{EC_GravText, EC_SingleAnimation};
use crate::common::game_mode::*;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::ClipEdge;
use crate::common::global_constants::{PHOFFSET, PWOFFSET, VELJUMP, VELMOVING_CHICKEN};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::*;
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

//capture the chicken
pub struct CGM_Chicken {
    pub cgame_mode: CGameMode,

    pub m_chicken: Ptr<CPlayer>,
}
impl_base!(CGM_Chicken => cgame_mode: CGameMode);

/// `static_cast<ClipEdge>(short)`.
pub(crate) fn clip_edge_from(v: i16) -> ClipEdge {
    match v as u8 {
        0 => ClipEdge::Top,
        1 => ClipEdge::Right,
        2 => ClipEdge::Bottom,
        _ => ClipEdge::Left,
    }
}

//capture the chicken
//one player is the chicken
//if he is killed the attacker becomes the chicken.
//get points for being the chicken
impl CGM_Chicken {
    pub fn new() -> Self {
        let mut this = CGM_Chicken { cgame_mode: CGameMode::new(), m_chicken: Ptr::null() };
        this.goal = 200;
        this.gamemode = game_mode_chicken;

        this.setup_mode_strings("Chicken", "Points", 50);
        this
    }

    pub fn chicken(&self) -> Ptr<CPlayer> {
        self.m_chicken
    }

    pub fn clear_chicken(&mut self) {
        self.m_chicken = Ptr::null();
    }
}

impl CGameModeTrait for CGM_Chicken {
    crate::impl_cgamemode_plumbing!();

    //called once when the game is started
    fn init(&mut self) {
        cgamemode_init(self);
        self.clear_chicken();
    }

    fn think(&mut self) {
        if self.gameover {
            self.displayplayertext();
            return;
        }

        if !self.m_chicken.is_null() {
            let mut chicken = self.m_chicken;
            if chicken.get_vel_x() > VELMOVING_CHICKEN {
                chicken.velx = VELMOVING_CHICKEN;
            } else if chicken.get_vel_x() < -VELMOVING_CHICKEN {
                chicken.velx = -VELMOVING_CHICKEN;
            }

            static mut counter: i16 = 0;

            if chicken.isready() && !chicken.is_tanooki_statue() {
                unsafe {
                    counter += 1;
                    if counter >= game_values.pointspeed {
                        counter = 0;
                        chicken.score().adjust_score(1);
                        self.check_winner(chicken);
                    }
                }
            }
        }
    }

    fn draw_foreground(&mut self) {
        unsafe {
            //Draw the chicken indicator around the chicken
            if game_values.gamemodesettings.chicken.usetarget && !self.gameover && !self.m_chicken.is_null() {
                let chicken = self.m_chicken;
                if chicken.iswarping() {
                    rm.spr_chicken.draw_clip(
                        chicken.left_x() as i32 - PWOFFSET - 16,
                        chicken.top_y() as i32 - PHOFFSET - 16,
                        &SDL_Rect { x: 0, y: 0, w: 64, h: 64 },
                        clip_edge_from(chicken.get_warp_state()),
                        chicken.get_warp_plane() as i32,
                    );
                } else if chicken.isready() {
                    rm.spr_chicken.draw(chicken.center_x() as i32 - 32, chicken.center_y() as i32 - 32);
                }
            }
        }
    }

    fn playerkilledplayer(&mut self, mut inflictor: Ptr<CPlayer>, mut other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        unsafe {
            if self.m_chicken.is_null() || other == self.m_chicken {
                self.m_chicken = inflictor;
                eyecandy[2].emplace(EC_GravText::new(
                    Ptr::from_mut(&mut rm.game_font_large),
                    inflictor.center_x(),
                    inflictor.bottom_y(),
                    "Chicken!".to_string(),
                    (-(VELJUMP as f64) * 1.5) as f32,
                ));
                //eyecandy[2].emplace<EC_SingleAnimation>(&rm->spr_fireballexplosion, inflictor.centerX() - 16, inflictor.centerY() - 16, 3, 8));
                eyecandy[2].emplace(EC_SingleAnimation::new(
                    Ptr::from_mut(&mut rm.spr_poof),
                    (inflictor.center_x() as i32 - 24) as i16,
                    (inflictor.center_y() as i32 - 24) as i16,
                    4,
                    5,
                ));
                if_sound_on_play(&mut rm.sfx_transform);

                if other == self.m_chicken {
                    other.set_corpse_type(1); //flag to use chicken corpse sprite
                }
            } else if inflictor == self.m_chicken {
                if !self.gameover {
                    inflictor.score().adjust_score(5);
                    return self.check_winner(inflictor);
                }
            }
        }

        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, mut player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        cgamemode_playerkilledself(self, player, style);

        if self.m_chicken == player {
            player.set_corpse_type(1); //flag to use chocobo corpse sprite

            if !self.gameover {
                self.m_chicken = Ptr::null();
            }
        }

        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, mut player: Ptr<CPlayer>, iType: i16) {
        if !self.gameover {
            player.score().adjust_score((10 * iType as i32) as i16);
            self.check_winner(player);
        }
    }

    fn check_winner(&mut self, mut player: Ptr<CPlayer>) -> PlayerKillType {
        if self.goal == -1 {
            return PlayerKillType::Normal;
        }

        if player.score().score >= self.goal {
            player.score().set_score(self.goal);
            self.winningteam = player.get_team_id();
            self.gameover = true;

            setup_score_board(false);
            show_score_board();
            remove_players_but_team(self.winningteam);
            return PlayerKillType::Removed;
        } else if player.score().score as f64 >= self.goal as f64 * 0.8 && !self.playedwarningsound {
            self.playwarningsound();
        }

        PlayerKillType::Normal
    }
}
