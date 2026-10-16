//! Port of src/smw/gamemodes/CardCollection.cpp

use crate::smw::players::PlayerId;
use crate::game::Game;
use crate::common::game_mode::{game_mode_collection, CGameMode, CGameModeTrait};
use crate::common::math::trig::{cosf, sinf};
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::moving_object_types::movingobject_collectioncard;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::{self as gm, remove_players_but_team, setup_score_board, show_score_board};

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

    pub fn release_card(&mut self, g: &mut Game, mut player: PlayerId) {
        if g.player(player).score().subscore[0] > 0 {
            let pos = Vec2s::new((g.player(player).center_x() as i32 - 16) as i16, (g.player(player).center_y() as i32 - 16) as i16);
            if !net_random::event(g, Ev::ReleaseCard, &[g.player(player).globalID as i32, pos.x as i32, pos.y as i32]) {
                self.drop_card(g, player, pos);
            }
        }
    }

    fn drop_card(&mut self, g: &mut Game, mut player: PlayerId, pos: Vec2s) {
        let mut gp = g.ptr();
        {
            let speed: f32 = 7.0f32 + RANDOM_INT(gp.get(), 9) as f32 / 2.0f32;
            let angle: f32 = -(RANDOM_INT(gp.get(), 314) as f32) / 100.0f32;
            let vel = Vec2f::new(speed * cosf(angle), speed * sinf(angle));

            g.player(player).score().subscore[0] -= 1;

            let iCardMask: i16 = (3i32 << ((g.player(player).score().subscore[0] as i32) << 1)) as i16;
            let iCardMaskInverted: i16 = !iCardMask;

            let iValue: i16 = ((g.player(player).score().subscore[1] as i32 & iCardMask as i32) >> ((g.player(player).score().subscore[0] as i32) << 1)) as i16;

            g.player(player).score().subscore[1] &= iCardMaskInverted;

            let obj = Ptr::new_box(MO_CollectionCard::new(g, gp.get().rm.spr_collectcards, 1, iValue, 30, vel, pos));
            gp.get().objectcontainer[1].add(g, obj);
        }
    }
}

fn spawn_card(g: &mut Game) {
        let mut gp = g.ptr();
    let iRandom: i16 = RANDOM_INT(gp.get(), 5) as i16;
    let mut iRandomCard: i16 = 0;
    if iRandom == 4 {
        iRandomCard = 2;
    } else if iRandom >= 2 {
        iRandomCard = 1;
    }

    let obj = Ptr::new_box(MO_CollectionCard::new(g, gp.get().rm.spr_collectcards, 0, iRandomCard, 0, Vec2f::zero(), Vec2s::zero()));
    gp.get().objectcontainer[1].add(g, obj);
}

pub fn net_spawn_card(g: &mut Game) {
    spawn_card(g);
}

pub fn net_release_card(g: &mut Game, mut player: PlayerId, x: i16, y: i16) {
    if let Some(this) = g.game_values.gamemode.get().as_any().downcast_mut::<CGM_Collection>() {
        if !player.is_none() && g.player(player).score().subscore[0] > 0 {
            this.drop_card(g, player, Vec2s::new(x, y));
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

    fn init(&mut self, g: &mut Game) {
        gm::cgamemode_init(g, self);
        self.timer = 0;

        //Zero out the number of cards teams have
        for iScore in 0..g.score_cnt as usize {
            g.score[iScore].subscore[0] = 0;
            g.score[iScore].subscore[1] = 0;
        }
    }

    fn think(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.timer += 1;
        if self.timer >= g.game_values.gamemodesettings.collection.rate {
            self.timer = 0;

            let mut iPowerupQuantity: i16 = g.game_values.gamemodemenusettings.collection.quantity;

            if 5 < iPowerupQuantity {
                iPowerupQuantity = g.players.len().wrapping_add(iPowerupQuantity as usize).wrapping_sub(7) as i16;
            }

            if g.objectcontainer[1].count_moving_types(movingobject_collectioncard) < iPowerupQuantity as usize && !net_random::event(g, Ev::CollectionCard, &[]) {
                spawn_card(g);
            }
        }

        if self.gameover {
            self.displayplayertext(g);
            return;
        }

        //Check if this team has collected 3 cards
        for iScore in 0..g.score_cnt as usize {
            if g.score[iScore].subscore[0] >= 3 {
                g.score[iScore].subscore[2] += 1;
                if g.score[iScore].subscore[2] >= g.game_values.gamemodemenusettings.collection.banktime {
                    let mut iPoints: i16 = 1;

                    if (g.score[iScore].subscore[1] & 63) == 0 {
                        //All Mushrooms
                        iPoints = 2;
                    } else if (g.score[iScore].subscore[1] & 63) == 21 {
                        //All Flowers
                        iPoints = 3;
                    } else if (g.score[iScore].subscore[1] & 63) == 42 {
                        //All Stars
                        iPoints = 5;
                    }

                    gp.score[iScore].adjust_score(g, iPoints);

                    g.score[iScore].subscore[0] = 0;
                    g.score[iScore].subscore[1] = 0;
                    g.score[iScore].subscore[2] = 0;
                }
            }
        }

        for &player in gp.players.iter() {
            self.check_winner(g, player);
        }
    }

    fn playerkilledplayer(&mut self, g: &mut Game, _player: PlayerId, other: PlayerId, _style: KillStyle) -> PlayerKillType {
        //Causes a card to come out of the player
        self.release_card(g, other);

        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, g: &mut Game, player: PlayerId, style: KillStyle) -> PlayerKillType {
        gm::cgamemode_playerkilledself(g, self, player, style);

        //Causes a card to come out of the player
        self.release_card(g, player);

        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, g: &mut Game, mut player: PlayerId, iType: i16) {
        if !self.gameover {
            g.player(player).score().adjust_score(g, iType);
            self.check_winner(g, player);
        }
    }

    fn check_winner(&mut self, g: &mut Game, mut player: PlayerId) -> PlayerKillType {
        if self.goal > -1 {
            if g.player(player).score().score >= self.goal {
                g.player(player).score().set_score(g, self.goal);
                self.winningteam = g.player(player).get_team_id();
                self.gameover = true;

                setup_score_board(g, false);
                show_score_board(g);
                remove_players_but_team(g, self.winningteam);

                for iScore in 0..g.score_cnt as usize {
                    g.score[iScore].subscore[0] = 0;
                    g.score[iScore].subscore[1] = 0;
                    g.score[iScore].subscore[2] = 0;
                }
            } else if g.player(player).score().score as f64 >= self.goal as f64 * 0.8 && !self.playedwarningsound {
                self.playwarningsound(g);
            }
        }

        PlayerKillType::Normal
    }
}
