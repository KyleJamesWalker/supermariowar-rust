//! Port of src/smw/gamemodes/CardCollection.cpp

use crate::common::game_mode::{game_mode_collection, CGameMode, CGameModeTrait};
use crate::common::math::trig::{cos, sin};
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::moving_object_types::movingobject_collectioncard;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::{self as gm, remove_players_but_team, setup_score_board, show_score_board};
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::main::{players, score, score_cnt};
use crate::smw::net_random::{self, Ev};
use crate::smw::objects::moving::mo_collection_card::MO_CollectionCard;
use crate::smw::player::CPlayer;
use std::any::Any;

//Collection (collect cards for points)
pub struct CGM_Collection {
    pub cgame_mode: CGameMode,

    timer: i16,
}
impl_base!(CGM_Collection => cgame_mode: CGameMode);

impl CGM_Collection {
    pub fn new() -> Self {
        let mut this = CGM_Collection { cgame_mode: CGameMode::new(), timer: 0 };
        this.goal = 30;
        this.gamemode = game_mode_collection;

        this.setup_mode_strings("Card Collection", "Points", 10);
        this
    }

    pub fn release_card(&mut self, mut player: Ptr<CPlayer>) {
        if player.score().subscore[0] > 0 {
            let pos = Vec2s::new((player.center_x() as i32 - 16) as i16, (player.center_y() as i32 - 16) as i16);
            if !net_random::event(Ev::ReleaseCard, &[player.globalID as i32, pos.x as i32, pos.y as i32]) {
                self.drop_card(player, pos);
            }
        }
    }

    fn drop_card(&mut self, mut player: Ptr<CPlayer>, pos: Vec2s) {
        {
            let speed: f32 = 7.0f32 + RANDOM_INT(9) as f32 / 2.0f32;
            let angle: f32 = -(RANDOM_INT(314) as f32) / 100.0f32;
            let vel = Vec2f::new(speed * cos(angle), speed * sin(angle));

            player.score().subscore[0] -= 1;

            let iCardMask: i16 = (3i32 << ((player.score().subscore[0] as i32) << 1)) as i16;
            let iCardMaskInverted: i16 = !iCardMask;

            let iValue: i16 = ((player.score().subscore[1] as i32 & iCardMask as i32) >> ((player.score().subscore[0] as i32) << 1)) as i16;

            player.score().subscore[1] &= iCardMaskInverted;

            unsafe {
                objectcontainer[1].add(Ptr::new_box(MO_CollectionCard::new(Ptr::from_mut(&mut rm.spr_collectcards), 1, iValue, 30, vel, pos)));
            }
        }
    }
}

fn spawn_card() {
    unsafe {
        let iRandom: i16 = RANDOM_INT(5) as i16;
        let mut iRandomCard: i16 = 0;
        if iRandom == 4 {
            iRandomCard = 2;
        } else if iRandom >= 2 {
            iRandomCard = 1;
        }

        objectcontainer[1].add(Ptr::new_box(MO_CollectionCard::new(Ptr::from_mut(&mut rm.spr_collectcards), 0, iRandomCard, 0, Vec2f::zero(), Vec2s::zero())));
    }
}

pub fn net_spawn_card() {
    spawn_card();
}

pub fn net_release_card(mut player: Ptr<CPlayer>, x: i16, y: i16) {
    unsafe {
        if let Some(this) = game_values.gamemode.as_any().downcast_mut::<CGM_Collection>() {
            if !player.is_null() && player.score().subscore[0] > 0 {
                this.drop_card(player, Vec2s::new(x, y));
            }
        }
    }
}

impl CGameModeTrait for CGM_Collection {
    fn gm(&self) -> &CGameMode {
        &self.cgame_mode
    }
    fn gm_mut(&mut self) -> &mut CGameMode {
        &mut self.cgame_mode
    }
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn init(&mut self) {
        gm::cgamemode_init(self);
        self.timer = 0;

        //Zero out the number of cards teams have
        unsafe {
            for iScore in 0..score_cnt as usize {
                score[iScore].subscore[0] = 0;
                score[iScore].subscore[1] = 0;
            }
        }
    }

    fn think(&mut self) {
        unsafe {
            self.timer += 1;
            if self.timer >= game_values.gamemodesettings.collection.rate {
                self.timer = 0;

                let mut iPowerupQuantity: i16 = game_values.gamemodemenusettings.collection.quantity;

                if 5 < iPowerupQuantity {
                    iPowerupQuantity = players.len().wrapping_add(iPowerupQuantity as usize).wrapping_sub(7) as i16;
                }

                if objectcontainer[1].count_moving_types(movingobject_collectioncard) < iPowerupQuantity as usize && !net_random::event(Ev::CollectionCard, &[]) {
                    spawn_card();
                }
            }

            if self.gameover {
                self.displayplayertext();
                return;
            }

            //Check if this team has collected 3 cards
            for iScore in 0..score_cnt as usize {
                if score[iScore].subscore[0] >= 3 {
                    score[iScore].subscore[2] += 1;
                    if score[iScore].subscore[2] >= game_values.gamemodemenusettings.collection.banktime {
                        let mut iPoints: i16 = 1;

                        if (score[iScore].subscore[1] & 63) == 0 {
                            //All Mushrooms
                            iPoints = 2;
                        } else if (score[iScore].subscore[1] & 63) == 21 {
                            //All Flowers
                            iPoints = 3;
                        } else if (score[iScore].subscore[1] & 63) == 42 {
                            //All Stars
                            iPoints = 5;
                        }

                        score[iScore].adjust_score(iPoints);

                        score[iScore].subscore[0] = 0;
                        score[iScore].subscore[1] = 0;
                        score[iScore].subscore[2] = 0;
                    }
                }
            }

            for &player in players.iter() {
                self.check_winner(player);
            }
        }
    }

    fn playerkilledplayer(&mut self, _player: Ptr<CPlayer>, other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        //Causes a card to come out of the player
        self.release_card(other);

        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        gm::cgamemode_playerkilledself(self, player, style);

        //Causes a card to come out of the player
        self.release_card(player);

        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, mut player: Ptr<CPlayer>, iType: i16) {
        if !self.gameover {
            player.score().adjust_score(iType);
            self.check_winner(player);
        }
    }

    fn check_winner(&mut self, mut player: Ptr<CPlayer>) -> PlayerKillType {
        if self.goal > -1 {
            if player.score().score >= self.goal {
                player.score().set_score(self.goal);
                self.winningteam = player.get_team_id();
                self.gameover = true;

                setup_score_board(false);
                show_score_board();
                remove_players_but_team(self.winningteam);

                unsafe {
                    for iScore in 0..score_cnt as usize {
                        score[iScore].subscore[0] = 0;
                        score[iScore].subscore[1] = 0;
                        score[iScore].subscore[2] = 0;
                    }
                }
            } else if player.score().score as f64 >= self.goal as f64 * 0.8 && !self.playedwarningsound {
                self.playwarningsound();
            }
        }

        PlayerKillType::Normal
    }
}
