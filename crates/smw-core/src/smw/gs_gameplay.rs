//! Port of src/smw/GSGameplay.cpp
//!
//! `_DEBUG`-only code (autotest, debug hotkeys, autokill) is not ported: the reference build is Release.

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::keys::*;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use smw_platform::InputEvent;
use crate::common::eyecandy::{
    CEyecandyContainer, EC_Announcement, EC_Bubble, EC_Cloud, EC_Ghost, EC_Leaf, EC_Rain, EC_SingleAnimation, EC_Snow, SpotlightManager,
};
use crate::common::eyecandy_styles::{AwardStyle, ScoreboardStyle};
use crate::common::file_list::MusicCategory;
use crate::common::game::App;
use crate::common::game_mode::*;
use crate::common::game_mode_settings::GameModeSettings;
use crate::common::game_values::{if_sound_on_play, AppState};
use crate::common::gameplay_styles::TeamCollisionStyle;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::gfx::{gfx_changefullscreen, gfx_take_screenshot};
use crate::common::global_constants::*;
use crate::common::input::COutputControl;
use crate::common::io_block::IO_BlockTrait;
use crate::common::map::MapBlock;
use crate::common::match_types::MatchType;
use crate::common::math::trig::{cosf, sinf};
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::moving_object_types::*;
use crate::common::object_base::CObjectTrait;
use crate::common::player_kill_styles::KillStyle;
use crate::common::random_number_generator::{RANDOM_BOOL, RANDOM_INT};
use crate::common::sfx::{sfxMusic, sfx_stopallsounds};
use crate::globals::*;
use crate::smw::ai::CPlayerAI;
use crate::smw::game_state::{GameState, GameStateManager};
use crate::smw::gamemodes::chicken::CGM_Chicken;
use crate::smw::gamemodes::game_mode::{setup_score_board, show_score_board};
use crate::smw::gamemodes::tag::CGM_Tag;
use crate::smw::gs_menu::MenuState;

use crate::common_netplay::protocol_definitions::NET_GAMESTATE_FRAMES_TO_SEND;
use crate::smw::net::{Net_IndexedPlayerData};
use crate::smw::net_random::{self, Ev};
use crate::smw::object_container::CObjectContainer;
use crate::smw::objects::blocks::bounce_block::B_BounceBlock;
use crate::smw::objects::blocks::breakable_block::B_BreakableBlock;
use crate::smw::objects::blocks::donut_block::B_DonutBlock;
use crate::smw::objects::blocks::flip_block::B_FlipBlock;
use crate::smw::objects::blocks::note_block::{B_NoteBlock, NoteBlockType};
use crate::smw::objects::blocks::on_off_switch_block::B_OnOffSwitchBlock;
use crate::smw::objects::blocks::powerup_block::B_PowerupBlock;
use crate::smw::objects::blocks::switch_block::B_SwitchBlock;
use crate::smw::objects::blocks::throw_block::B_ThrowBlock;
use crate::smw::objects::blocks::view_block::B_ViewBlock;
use crate::smw::objects::blocks::weapon_breakable_block::{B_WeaponBreakableBlock, WeaponDamageType};
use crate::smw::objects::carriable::co_flag::CO_Flag;
use crate::smw::objects::carriable::co_kuribo_shoe::CO_KuriboShoe;
use crate::smw::objects::carriable::co_shell::CO_Shell;
use crate::smw::objects::carriable::co_spike::CO_Spike;
use crate::smw::objects::carriable::co_spring::CO_Spring;
use crate::smw::objects::carriable::co_throw_block::CO_ThrowBlock;
use crate::smw::objects::carriable::co_throw_box::CO_ThrowBox;
use crate::smw::objects::moving::mo_bullet_bill::MO_BulletBill;
use crate::smw::objects::moving::mo_coin::MO_Coin;
use crate::smw::objects::moving::mo_pirhana_plant::MO_PirhanaPlant;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::objects::mystery_mushroom_temp_player::MysteryMushroomTempPlayer;
use crate::smw::objects::switch_color::SwitchColor;
use crate::smw::objects::throw_block_type::ThrowBlockType;
use crate::smw::player::{get_player_from_global_id, player_killed_player, CPlayer, PlayerDeathStyle, PlayerState};
use std::io::Write;

pub const COUNTDOWN_START_INDEX: i16 = 4;


pub static scorepowerupoffsets: [[i16; 3]; 3] = [[37, 0, 0], [71, 89, 0], [105, 123, 141]];

/// Constructors of this file's class-type globals, plus the link-time `musicfinished` hook in sfx.
pub fn init_globals(g: &mut Game) {
    g.spotlightManager.init(SpotlightManager::new());
    g.sfx.musicfinished = g.sfx.musicfinished;
}

pub struct GameplayState {
    iCountDownState: i16,
    iCountDownTimer: i16,
    iWindTimer: i16,
    dNextWind: f32,
    iScoreTextOffset: [i16; 4],

    pub respawnCount: [i16; 4],
    respawnanimationtimer: [i16; 4],
    respawnanimationframe: [i16; 4],

    previous_playerKeys: COutputControl,

    //Vars that keep track of spinning the screen
    spinangle: f32,
    spinspeed: f32,
    spindirection: i16,
    spintimer: i16,
    pub _alias: Aliased,
}

impl GameplayState {
    pub const scoreoffsets: [i16; 3] = [2, 36, 70];
    pub const g_iPowerupToIcon: [i16; 8] = [80, 176, 272, 304, 336, 368, 384, 400];

    fn new() -> Self {
        let mut s = GameplayState { _alias: Aliased::new(),
            iCountDownState: 0,
            iCountDownTimer: 0,
            iWindTimer: 0,
            dNextWind: 0.0,
            iScoreTextOffset: [0; 4],
            respawnCount: [0; 4],
            respawnanimationtimer: [0; 4],
            respawnanimationframe: [0; 4],
            previous_playerKeys: COutputControl::default(),
            spinangle: 0.0,
            spinspeed: 0.0,
            spindirection: 0,
            spintimer: 0,
        };

        for p in 0..4usize {
            s.respawnCount[p] = 0;
            s.respawnanimationtimer[p] = 0;
            s.respawnanimationframe[p] = 0;
        }

        //Vars that keep track of spinning the screen
        s.spinangle = 0.0f32;
        s.spinspeed = 0.0f32;
        s.spindirection = 1;
        s.spintimer = 0;
        s
    }

    pub fn instance(g: &mut Game) -> &'static mut GameplayState {
        // Safety net until globals::init_globals calls gs_gameplay::init_globals.
        if !g.spotlightManager.is_initialized() {
            init_globals(g);
        }
        g.ptr().get().gps.get_or_insert_with(GameplayState::new)
    }

    /// Not in the C++: the fields a replay checkpoint (smw/checkpoint.rs) saves and restores.
    pub fn checkpoint(&mut self, s: &mut crate::smw::checkpoint::Snap) {
        s.io(&mut self.iCountDownState);
        s.io(&mut self.iCountDownTimer);
        s.io(&mut self.iWindTimer);
        s.io(&mut self.dNextWind);
        s.io(&mut self.iScoreTextOffset);
        s.io(&mut self.respawnCount);
        s.io(&mut self.respawnanimationtimer);
        s.io(&mut self.respawnanimationframe);
        s.io(&mut self.spinangle);
        s.io(&mut self.spinspeed);
        s.io(&mut self.spindirection);
        s.io(&mut self.spintimer);
    }

    /// Replay-harness `G`/`P`/`T`/`O` records (docs/REPLAY.md).
    pub fn harness_dump(&mut self, g: &mut Game, out: &mut impl Write) -> std::io::Result<()> {
        let gm = g.mode();
        writeln!(out, "G mode={} gameover={} winner={}", gm.gamemode as i32, gm.gameover as i32, gm.winningteam)?;

        for i in 0..g.players.len() {
            let p = g.players[i];
            let s = g.player(p).score(g).score as i32;
            writeln!(
                out,
                "P id={} team={} ix={} iy={} fx={} fy={} velx={} vely={} state={} score={} powerup={} inair={}",
                g.player(p).globalID,
                g.player(p).teamID,
                g.player(p).ix,
                g.player(p).iy,
                format!("{:.4}", g.player(p).fx as f64),
                format!("{:.4}", g.player(p).fy as f64),
                format!("{:.4}", g.player(p).velx as f64),
                format!("{:.4}", g.player(p).vely as f64),
                g.player(p).state as i32,
                s,
                g.player(p).powerup,
                g.player(p).inair as i32
            )?;
        }
        if let Some(line) = crate::smw::gamemodes::star::harness_record(g) {
            writeln!(out, "{}", line)?;
        }

        writeln!(
            out,
            "O noncol={} obj0={} obj1={} obj2={} ec0={} ec1={} ec2={}",
            g.noncolcontainer.list().len(),
            g.objectcontainer[0].list().len(),
            g.objectcontainer[1].list().len(),
            g.objectcontainer[2].list().len(),
            g.eyecandy[0].eyecandies.len(),
            g.eyecandy[1].eyecandies.len(),
            g.eyecandy[2].eyecandies.len()
        )
    }
}

//-----------------------------------------------------------------------------
// THE GAME LOOP
//-----------------------------------------------------------------------------

const fn rc(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect { x, y, w, h }
}

const iCountDownNumbers: [[[Rect; 2]; 4]; 4] = [
    [
        [rc(0, 0, 64, 64), rc(288, 208, 64, 64)],
        [rc(0, 64, 48, 48), rc(296, 216, 48, 48)],
        [rc(192, 64, 32, 32), rc(304, 224, 32, 32)],
        [rc(0, 112, 16, 16), rc(312, 232, 16, 16)],
    ],
    [
        [rc(64, 0, 64, 64), rc(288, 208, 64, 64)],
        [rc(48, 64, 48, 48), rc(296, 216, 48, 48)],
        [rc(224, 64, 32, 32), rc(304, 224, 32, 32)],
        [rc(16, 112, 16, 16), rc(312, 232, 16, 16)],
    ],
    [
        [rc(128, 0, 64, 64), rc(288, 208, 64, 64)],
        [rc(96, 64, 48, 48), rc(296, 216, 48, 48)],
        [rc(192, 96, 32, 32), rc(304, 224, 32, 32)],
        [rc(32, 112, 16, 16), rc(312, 232, 16, 16)],
    ],
    [
        [rc(192, 0, 64, 64), rc(288, 208, 64, 64)],
        [rc(144, 64, 48, 48), rc(296, 216, 48, 48)],
        [rc(224, 96, 32, 32), rc(304, 224, 32, 32)],
        [rc(48, 112, 16, 16), rc(312, 232, 16, 16)],
    ],
];

const iCountDownTimes: [i16; 28] = [3, 3, 3, 15, 3, 3, 3, 3, 3, 3, 15, 3, 3, 3, 3, 3, 3, 15, 3, 3, 3, 3, 3, 3, 45, 3, 3, 3];
const iCountDownRectSize: [i16; 28] = [3, 2, 1, 0, 1, 2, 3, 3, 2, 1, 0, 1, 2, 3, 3, 2, 1, 0, 1, 2, 3, 3, 2, 1, 0, 1, 2, 3];
const iCountDownRectGroup: [i16; 28] = [0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3];
const iCountDownAnnounce: [i16; 28] = [-1, -1, -1, 12, -1, -1, -1, -1, -1, -1, 13, -1, -1, -1, -1, -1, -1, 14, -1, -1, -1, -1, -1, -1, 15, -1, -1, -1];

#[rustfmt::skip]
const g_iCollisionMap: [[i16; MOVINGOBJECT_LAST as usize]; MOVINGOBJECT_LAST as usize] = [
//   0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], //movingobject_none = 0
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], //movingobject_powerup = 1
    [0,0,0,1,1,0,0,1,1,0,0,0,1,1,0,0,0,0,0,0,0,1,0,1,1,0,0,0,0,0,1,0,0,1], //movingobject_fireball = 2
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], //movingobject_goomba = 3
    [0,0,0,1,1,0,0,0,0,0,0,0,1,1,0,0,0,0,1,0,0,0,0,1,1,0,0,0,0,0,1,0,0,0], //movingobject_bulletbill = 4
    [0,0,0,1,1,0,0,1,1,0,0,0,1,1,0,0,0,0,0,0,0,1,0,1,1,0,0,0,0,0,1,0,0,1], //movingobject_hammer = 5
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], //movingobject_poisonpowerup = 6
    [0,0,0,1,1,0,0,1,0,0,0,0,1,1,0,0,0,0,1,0,1,1,0,1,1,0,0,0,0,0,1,0,0,0], //movingobject_shell = 7
    [0,0,0,1,1,0,0,1,1,0,0,0,1,1,0,0,0,0,1,0,1,1,0,1,1,0,0,0,0,0,1,0,0,0], //movingobject_throwblock = 8
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0], //movingobject_egg = 9
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], //movingobject_star = 10
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0], //movingobject_flag = 11
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], //movingobject_cheepcheep = 12
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], //movingobject_koopa = 13
    [0,0,0,1,1,0,0,1,1,0,0,0,1,1,0,0,0,0,0,0,0,1,0,1,1,0,0,0,0,0,1,0,0,1], //movingobject_boomerang = 14
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], //movingobject_carried = 15
    [0,0,0,1,0,0,0,1,1,0,0,0,1,1,0,0,0,0,0,0,0,0,0,1,1,0,0,0,0,0,0,0,0,1], //movingobject_iceblast = 16
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], //movingobject_bomb = 17
    [0,0,0,1,0,0,0,0,0,0,0,0,1,1,0,0,0,0,0,0,0,0,0,1,1,0,0,0,0,0,0,0,0,0], //movingobject_podobo = 18
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], //movingobject_treasurechest = 19
    [0,0,0,1,1,0,0,0,0,0,0,0,1,1,0,0,0,0,0,0,0,1,0,1,1,0,0,0,0,0,1,0,0,0], //movingobject_attackzone = 20
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], //movingobject_pirhanaplant = 21
    [0,0,0,1,1,0,0,1,1,0,0,0,1,1,0,0,0,0,0,0,0,1,0,1,1,0,0,0,0,0,1,0,0,1], //movingobject_explosion = 22
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], //movingobject_buzzybeetle = 23
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], //movingobject_spiny = 24
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], //movingobject_phantokey = 25
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], //movingobject_flagbase = 26
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], //movingobject_yoshi = 27
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], //movingobject_coin = 28
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], //movingobject_collectioncard = 29
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], //movingobject_sledgebrother = 30
    [0,0,0,1,1,0,0,1,1,0,0,0,1,1,0,0,0,0,0,0,0,1,0,1,1,0,0,0,0,0,0,0,0,1], //movingobject_sledgehammer = 31
    [0,0,0,1,1,0,0,1,1,0,0,0,1,1,0,0,0,0,0,0,0,1,0,1,1,0,0,0,0,0,0,0,0,1], //movingobject_superfireball = 32
    [0,0,0,1,1,0,0,1,1,0,0,0,1,1,0,0,0,0,0,0,1,1,0,1,1,0,0,0,0,0,1,0,0,1], //movingobject_throwbox = 33
];

//handles a collision between a powerup and an object
pub fn collisionhandler_o2o(g: &mut Game, o1: Ptr<dyn IO_MovingObjectTrait>, mut o2: Ptr<dyn IO_MovingObjectTrait>) {
    o2.collide_object(g, o1);
}

//Must only be called after organizeteams() is called
pub fn lookup_team_id_sub(g: &mut Game, id: i16, teamID: Option<&mut i16>, subTeamID: Option<&mut i16>) -> i16 {
    for i in 0..g.score_cnt {
        for j in 0..g.game_values.teamcounts[i as usize] {
            if g.game_values.teamids[i as usize][j as usize] == id {
                if let Some(t) = teamID {
                    *t = i;
                }

                if let Some(s) = subTeamID {
                    *s = j;
                }

                return i;
            }
        }
    }

    if let Some(t) = teamID {
        *t = -1;
    }

    if let Some(s) = subTeamID {
        *s = -1;
    }

    -1
}

pub fn lookup_team_id(g: &mut Game, id: i16) -> i16 {
    lookup_team_id_sub(g, id, None, None)
}

pub fn musicfinished(g: &mut Game) {
    let mut gp = g.ptr();
    if !g.game_values.music {
        return;
    }

    if g.game_values.appstate == AppState::Game && !g.mode().gameover {
        return;
    }

    if g.sfx.fResumeMusic {
        gp.rm.backgroundmusic[3].play(g, false, false);
    }
}

pub fn get_mode_icon_index_from_mode(mut iMode: i16) -> i16 {
    if iMode as i32 == game_mode_pipe_minigame {
        iMode = 25;
    } else if iMode as i32 == game_mode_boss_minigame {
        iMode = 26;
    } else if iMode as i32 == game_mode_boxes_minigame {
        iMode = 27;
    }

    iMode
}


fn draw_rect(g: &mut Game, sprite: &gfxSprite, x: i32, y: i32, sx: i32, sy: i32, w: i32, h: i32) {
    sprite.draw_src(g, x, y, &Rect { x: sx, y: sy, w, h });
}

/// `static_cast<MusicCategory>(id)`.
fn music_category(id: i16) -> MusicCategory {
    MusicCategory::ALL[id as usize]
}

/// `sfxMusic(path)`, which throws on failure.
fn load_music(g: &mut Game, path: &std::path::Path) -> sfxMusic {
    sfxMusic::from_file(g, path).unwrap_or_else(|e| std::panic::panic_any(e))
}

impl GameplayState {
    //
    // INIT
    //
    fn create_players(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        //Create players for this game
        for iPlayer in 0..4i16 {
            let p = iPlayer as usize;
            self.respawnCount[p] = 0;

            if g.game_values.singleplayermode == -1 || g.game_values.singleplayermode == iPlayer {
                if g.game_values.playercontrol[p] > 0 {
                    let mut teamid: i16 = 0;
                    let mut subteamid: i16 = 0;
                    lookup_team_id_sub(g, iPlayer, Some(&mut teamid), Some(&mut subteamid));

                    let mut ai: Option<Box<CPlayerAI>> = None;
                    if g.game_values.playercontrol[p] == 2 {
                        ai = Some(Box::new(CPlayerAI::new()));
                    }

                    let localID = g.players.len() as i16;
                    gp.get().players.push(CPlayer::new(
                        g,
                        iPlayer,
                        localID,
                        teamid,
                        subteamid,
                        g.game_values.colorids[p],
                        gp.get().rm.spr_player[p],
                        teamid as usize,
                        p,
                        ai.map(|a| a as Box<dyn crate::smw::ai::CPlayerAITrait>),
                    ));
                } else if !g.game_values.keeppowerup {
                    //Reset off player's stored powerups if they are not playing
                    g.game_values.storedpowerups[p] = -1;
                }
            }

            //If the gamemode allows stored powerups, then assign the game stored slot to the powerup this player has
            if g.mode().has_stored_powerups() {
                g.game_values.gamepowerups[p] = g.game_values.storedpowerups[p];
            } else {
                g.game_values.gamepowerups[p] = -1;
            }

            g.game_values.bulletbilltimer[p] = 0;
            g.game_values.bulletbillspawntimer[p] = 0;
        }
    }

    fn init_score_display_position(&mut self, g: &mut Game) {
        let mut totalspace: i16 = 0;
        for i in 0..g.score_cnt as usize {
            totalspace = (totalspace as i32 + 56 + g.game_values.teamcounts[i] as i32 * 34) as i16;
        }
        totalspace = (totalspace as i32 + 20 * (g.score_cnt as i32 - 1)) as i16;

        for i in 0..g.score_cnt as usize {
            let mut s = g.score[i].as_ptr();
            if g.game_values.scoreboardstyle == ScoreboardStyle::Top || g.game_values.scoreboardstyle == ScoreboardStyle::Bottom {
                s.x = ((App::screenWidth - totalspace as i32) >> 1) as i16;

                for k in 0..i {
                    s.x = (s.x as i32 + 76 + g.game_values.teamcounts[k] as i32 * 34) as i16;
                }

                let mut iScoreOffsetY: i16 = 0;
                if g.mode().gamemode == game_mode_health || g.mode().gamemode == game_mode_collection {
                    iScoreOffsetY = 18;
                }

                s.y = (5 + if g.game_values.scoreboardstyle == ScoreboardStyle::Bottom { 429 - iScoreOffsetY as i32 } else { 0 }) as i16;
            } else {
                let mut iScoreOffsetY: i16 = 0;
                if g.mode().gamemode == game_mode_health || g.mode().gamemode == game_mode_collection {
                    iScoreOffsetY = 18;
                }

                s.x = (5 + (574 - (34 * g.game_values.teamcounts[i] as i32)) * (i as i32 % 2)) as i16;
                s.y = (5 + (429 - iScoreOffsetY as i32) * if i > 1 { 1 } else { 0 }) as i16;
            }

            s.fromx = s.x;
            s.fromy = s.y;
            s.place = i as i16;
            s.order = -1;
        }
    }

    fn init_eye_candy(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let mut rm = g.rm;
        for iEyeCandyLayer in 0..3usize {
            //Clouds
            if g.g_map.eyecandy[iEyeCandyLayer] & 1 != 0 {
                for _i in 0..4 {
                    let mut velx: f32; //speed of cloud, small clouds are slower than big ones
                    let srcy: i16;
                    let w: i16;
                    let h: i16;

                    if RANDOM_BOOL(gp.get()) {
                        velx = ((RANDOM_INT(gp.get(), 51) - 25) as i16) as f32 / 10.0f32; //big clouds: -3 - +3 pixel/frame
                        srcy = 0;
                        w = 60;
                        h = 28;
                    } else {
                        velx = ((RANDOM_INT(gp.get(), 41) - 20) as i16) as f32 / 10.0f32; //small clouds: -2 - +2 pixel/frame
                        srcy = 28;
                        w = 28;
                        h = 12;
                    }

                    velx = if velx < 0.5f32 && velx > -0.5f32 { 1.0f32 } else { velx }; //no static clouds please

                    //add cloud to eyecandy array
                    let x = RANDOM_INT(gp.get(), App::screenWidth) as f32;
                    let y = RANDOM_INT(gp.get(), 100) as f32;
                    g.eyecandy[iEyeCandyLayer].emplace(EC_Cloud::new(&gp.rm, g.rm.spr_clouds, x, y, velx, 0, srcy, w, h));
                }
            }

            //Ghosts
            if g.g_map.eyecandy[iEyeCandyLayer] & 2 != 0 {
                for _i in 0..8 {
                    let iGhostSrcY: i16 = (((RANDOM_INT(gp.get(), 3) as i16) as i32) << 5) as i16; //ghost type
                    let mut velx: f32 = ((RANDOM_INT(gp.get(), 51) - 25) as i16) as f32 / 10.0f32; //big clouds: -3 - +3 pixel/frame

                    velx = if velx < 0.5f32 && velx > -0.5f32 { if RANDOM_INT(gp.get(), 1) != 0 { 1.0f32 } else { -1.0f32 } } else { velx }; //no static clouds please

                    //add cloud to eyecandy array
                    let x = RANDOM_INT(gp.get(), App::screenWidth) as f32;
                    let y = RANDOM_INT(gp.get(), 100) as f32;
                    g.eyecandy[iEyeCandyLayer].emplace(EC_Ghost::new(&gp.rm, 
                        g.rm.spr_ghosts,
                        x,
                        y,
                        velx,
                        8,
                        2,
                        if velx < 0.0f32 { 64 } else { 0 },
                        iGhostSrcY,
                        32,
                        32,
                    ));
                }
            }

            //Leaves
            if g.g_map.eyecandy[iEyeCandyLayer] & 4 != 0 {
                for _i in 0..15 {
                    let x = RANDOM_INT(gp.get(), App::screenWidth) as f32;
                    let y = RANDOM_INT(gp.get(), App::screenHeight) as f32;
                    gp.eyecandy[iEyeCandyLayer].emplace(EC_Leaf::new(g, rm.spr_leaves, x, y));
                }
            }

            //Snow
            if g.g_map.eyecandy[iEyeCandyLayer] & 8 != 0 {
                for _i in 0..15 {
                    let x = RANDOM_INT(gp.get(), App::screenWidth) as f32;
                    let y = RANDOM_INT(gp.get(), App::screenHeight) as f32;
                    gp.eyecandy[iEyeCandyLayer].emplace(EC_Snow::new(g, rm.spr_snow, x, y, 0));
                }
            }

            //Fish
            let iFishWeights: [i16; 8] = [20, 20, 15, 10, 10, 5, 10, 10];
            let iFishSettings: [[i16; 4]; 8] =
                [[0, 0, 64, 44], [0, 44, 64, 44], [0, 44, 48, 44], [32, 32, 16, 12], [32, 44, 16, 12], [32, 16, 16, 28], [32, 0, 32, 28], [32, 44, 32, 28]];
            if g.g_map.eyecandy[iEyeCandyLayer] & 16 != 0 {
                for i in 0..8i16 {
                    let mut velx: f32 = ((RANDOM_INT(gp.get(), 41) - 20) as i16) as f32 / 10.0f32;
                    velx = if velx < 0.5f32 && velx > -0.5f32 { 1.0f32 } else { velx }; //Keep fish from moving too slowly

                    let mut srcx = iFishSettings[0][0];
                    let mut srcy = iFishSettings[0][1];
                    let mut w = iFishSettings[0][2];
                    let mut h = iFishSettings[0][3];

                    let iRandomFish = RANDOM_INT(gp.get(), 100) as i16;

                    let mut iFishWeightCount: i16 = 0;
                    for iFish in 0..8usize {
                        iFishWeightCount += iFishWeights[iFish];

                        if iRandomFish < iFishWeightCount {
                            srcx = iFishSettings[iFish][0];
                            srcy = iFishSettings[iFish][1];
                            w = iFishSettings[iFish][2];
                            h = iFishSettings[iFish][3];
                            break;
                        }
                    }

                    //add cloud to eyecandy array
                    let iPossibleY: i16 = ((App::screenHeight - h as i32) / 10) as i16;
                    let dDestY: f32 = (RANDOM_INT(gp.get(), iPossibleY as i32) + iPossibleY as i32 * i as i32) as f32;
                    let x = RANDOM_INT(gp.get(), App::screenWidth) as f32;
                    g.eyecandy[iEyeCandyLayer].emplace(EC_Cloud::new(&g.rm, 
                        g.rm.spr_fish,
                        x,
                        dDestY,
                        velx,
                        (srcx as i32 + if velx > 0.0f32 { 64 } else { 0 }) as i16,
                        srcy,
                        w,
                        h,
                    ));
                }
            }

            //Rain
            if g.g_map.eyecandy[iEyeCandyLayer] & 32 != 0 {
                for _i in 0..20 {
                    let x = RANDOM_INT(gp.get(), App::screenWidth) as f32;
                    let y = RANDOM_INT(gp.get(), App::screenHeight) as f32;
                    gp.eyecandy[iEyeCandyLayer].emplace(EC_Rain::new(g, rm.spr_rain, x, y));
                }
            }

            //Bubbles
            if g.g_map.eyecandy[iEyeCandyLayer] & 64 != 0 {
                for _i in 0..10 {
                    let x = RANDOM_INT(gp.get(), App::screenWidth) as f32;
                    let y = RANDOM_INT(gp.get(), App::screenHeight) as f32;
                    gp.eyecandy[iEyeCandyLayer].emplace(EC_Bubble::new(g, rm.spr_rain, x, y));
                }
            }
        }
    }

    fn init_run_game(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        g.y_shake = 0;
        g.x_shake = 0;

        //Reset the screen spin variables
        self.spinangle = 0.0f32;
        self.spinspeed = 0.0f32;
        self.spindirection = 1;
        self.spintimer = 0;

        g.game_values.reset_secret_counters();

        //Reset the keys each time we switch from menu to game and back
        g.game_values.playerInput.reset_keys();

        self.create_players(g);
        gp.game_values.reset_gameplay_settings(g);
        self.init_score_display_position(g);
        self.init_eye_candy(g);

        self.iWindTimer = 0;
        self.dNextWind = (RANDOM_INT(gp.get(), 41) - 20) as f32 / 4.0f32;
        g.game_values.flags.gamewindx = ((RANDOM_INT(gp.get(), 41)) - 20) as f32 / 4.0f32;

        //Initialize players after game init has finished
        for i in 0..g.players.len() {
            let mut player = g.players[i];
            g.player(player).init(g);
        }
    }
}

//
// RUNNING
//

pub fn count_alive_teams(g: &mut Game, lastteam: Option<&mut i16>) -> i16 {
    let mut findlastteam: i16 = 0;

    let mut teamalive: [bool; 4] = [false, false, false, false];
    for player in g.players.iter() {
        if !g.player(*player).isdead() {
            teamalive[g.player(*player).teamID as usize] = true;
        }
    }

    let mut numteams: i16 = 0;
    for k in 0..4i16 {
        if teamalive[k as usize] {
            findlastteam = k;
            numteams += 1;
        }
    }

    if let Some(lastteam) = lastteam {
        if numteams == 1 {
            *lastteam = findlastteam;
        } else {
            *lastteam = -1;
        }
    }

    numteams
}

impl GameplayState {
    fn clean_dead_players(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let mut fCheckForGameOver = false;

        let gmChicken: Ptr<CGM_Chicken> = match g.mode().get().as_any().downcast_mut::<CGM_Chicken>() {
            Some(g) => Ptr::from_mut(g),
            None => Ptr::null(),
        };
        let gmTag: Ptr<CGM_Tag> = match g.mode().get().as_any().downcast_mut::<CGM_Tag>() {
            Some(g) => Ptr::from_mut(g),
            None => Ptr::null(),
        };

        // The index still advances after a removal, so the player shifted into slot i is skipped this frame (as in C++).
        let mut i: usize = 0;
        while i < g.players.len() {
            if g.player(g.players[i]).state == PlayerState::Dead {
                fCheckForGameOver = true;

                let globalID = g.player(g.players[i]).globalID as usize;
                crate::smw::net_outcomes::note_death(g, globalID as i16, PlayerDeathStyle::Jump as i32, true);
                if self.respawnCount[globalID] <= 0 {
                    g.player(gp.players[i]).die(g, PlayerDeathStyle::Jump, true, false);
                }

                self.respawnCount[globalID] = 0;

                if !gmTag.is_null() && gmTag.get().tagged() == g.players[i] {
                    gmTag.get().set_tagged(PlayerId::NONE);
                }

                if !gmChicken.is_null() && gmChicken.get().chicken() == g.players[i] {
                    gmChicken.get().clear_chicken();
                }

                // C++ leaves this dangling and reads freed memory when the flag reaches a base.
                for &obj in g.objectcontainer[1].list() {
                    let mut obj = obj;
                    if let Some(flag) = obj.as_any().downcast_mut::<CO_Flag>() {
                        if flag.owner_throw == g.players[i] {
                            flag.owner_throw = PlayerId::NONE;
                        }
                    }
                }

                let removed = g.players[i];
                g.players.remove(removed);

                let mut j = i;
                while j < g.players.len() - 1 {
                    g.players[j] = g.players[j + 1];
                    g.player(g.players[j]).localID = j as i16;
                    j += 1;
                }

                g.players.pop();
            }
            i += 1;
        }

        if fCheckForGameOver && g.mode().gamemode != game_mode_bonus && g.mode().gamemode != game_mode_boss_minigame {
            let mut lastteam: i16 = -1;
            if !g.mode().gameover && count_alive_teams(g, Some(&mut lastteam)) <= 1 {
                g.mode().gameover = true;
                g.mode().winningteam = lastteam;
                setup_score_board(g, true);
                show_score_board(g);
            }
        }
    }
}

pub fn check_wind_event(g: &mut Game, iWindTimer: &mut i16, dNextWind: &mut f32) {
    let mut gp = g.ptr();
    if *iWindTimer <= 0 {
        //Then trigger next wind event
        if g.game_values.flags.gamewindx < *dNextWind {
            g.game_values.flags.gamewindx += 0.02f32;

            if g.game_values.flags.gamewindx >= *dNextWind {
                *iWindTimer = ((RANDOM_INT(gp.get(), 60)) + 30) as i16;
            }
        } else if g.game_values.flags.gamewindx >= *dNextWind {
            g.game_values.flags.gamewindx -= 0.02f32;

            if g.game_values.flags.gamewindx <= *dNextWind {
                *iWindTimer = ((RANDOM_INT(gp.get(), 60)) + 30) as i16;
            }
        }
    } else {
        *iWindTimer -= 1;
        if *iWindTimer <= 0 {
            *dNextWind = ((RANDOM_INT(gp.get(), 41)) - 20) as f32 / 4.0f32;
        }
    }
}

pub fn clean_dead_non_player_objects(g: &mut Game) {
    g.eyecandy[0].clean_dead_objects();
    g.eyecandy[1].clean_dead_objects();
    g.eyecandy[2].clean_dead_objects();

    g.objectcontainer[2].clean_dead_objects();
    g.objectcontainer[1].clean_dead_objects();
    g.objectcontainer[0].clean_dead_objects();
    g.noncolcontainer.clean_dead_objects();
}

pub fn animate_during_countdown(g: &mut Game) {
        let mut gp = g.ptr();
    //Move platforms
    gp.g_map.update_platforms(g);

    clean_dead_non_player_objects(g);

    //Keep updating map hazards
    gp.noncolcontainer.update(g);
    gp.objectcontainer[0].update(g);
    gp.objectcontainer[1].update(g);
    gp.objectcontainer[2].update(g);

    gp.eyecandy[0].update(g);
    gp.eyecandy[1].update(g);
    gp.eyecandy[2].update(g);

    gp.g_map.update(g);
}

pub fn net_pow_kills(g: &mut Game, ids: &[i32]) {
    let mut gp = g.ptr();
    let pKillPlayers: Vec<PlayerId> = ids.iter().filter_map(|&id| g.players.iter().copied().find(|p| g.player(*p).globalID as i32 == id)).collect();
    let iNumKillPlayers = pKillPlayers.len() as i16;
    if iNumKillPlayers == 0 {
        return;
    }
    let mut iRandPlayer = RANDOM_INT(gp.get(), iNumKillPlayers as i32) as i16;
    for _iPlayer in 0..iNumKillPlayers {
        player_killed_player(
            g,
            g.game_values.flags.screenshakeplayerid,
            pKillPlayers[iRandPlayer as usize],
            PlayerDeathStyle::Jump,
            KillStyle::Pow,
            false,
            false,
        );

        iRandPlayer += 1;
        if iRandPlayer >= iNumKillPlayers {
            iRandPlayer = 0;
        }
    }
}

pub fn shake_screen(g: &mut Game) {
        let mut gp = g.ptr();
    if g.game_values.flags.screenshaketimer <= 0 {
        //Make sure we zero out the shake value after it is done
        g.x_shake = 0;
        return;
    }

    g.game_values.flags.screenshaketimer -= 1;

    if g.shakeleft {
        g.x_shake -= 2;
        if g.x_shake <= -2 {
            g.shakeleft = false;
        }
    } else {
        g.x_shake += 2;
        if g.x_shake >= 2 {
            g.shakeleft = true;
        }
    }

    //Kill players touching the ground (or in air for MOd blocks)
    let mut iNumKillPlayers: i16 = 0;
    let mut pKillPlayers: [PlayerId; 4] = [PlayerId::NONE; 4];

    let mut killer1 = get_player_from_global_id(g, g.game_values.flags.screenshakeplayerid);

    for i in 0..g.players.len() {
        let player = g.players[i];
        //Don't kill the player that triggered the POW/MOd
        if g.player(player).globalID == g.game_values.flags.screenshakeplayerid {
            continue;
        }

        //Don't kill players on his team either (if friendly fire is off)
        if g.game_values.teamcollision != TeamCollisionStyle::On && g.game_values.flags.screenshaketeamid == g.player(player).teamID {
            continue;
        }

        //Kill other players
        if !g.player(player).is_invincible_on_bottom() && g.player(player).isready() {
            if g.game_values.flags.screenshakekillinair == g.player(player).inair {
                pKillPlayers[iNumKillPlayers as usize] = player;
                iNumKillPlayers += 1;

                if !killer1.is_none() {
                    g.game_values.flags.screenshakekillscount += 1;

                    if g.player(killer1).inair {
                        g.player(killer1).killsinrowinair -= 1;
                    }
                }
            }
        }
    }

    //Randomize the order in which the players are killed
    if iNumKillPlayers > 0 {
        let ids: Vec<i32> = pKillPlayers[..iNumKillPlayers as usize].iter().map(|p| g.player(*p).globalID as i32).collect();
        if !net_random::event(g, Ev::Pow, &ids) {
            net_pow_kills(g, &ids);
        }
    }

    //Kill goombas and koopas
    let n0 = g.objectcontainer[0].list().len();
    for iObj in 0..n0 {
        let mut obj = g.objectcontainer[0].list()[iObj];
        if let Some(mo) = obj.as_io_moving_object() {
            let mut movingobject = mo.as_mo_ptr();
            let r#type: MovingObjectType = movingobject.get_moving_object_type();

            if (r#type == movingobject_goomba || r#type == movingobject_koopa || r#type == movingobject_buzzybeetle || r#type == movingobject_spiny)
                && g.game_values.flags.screenshakekillinair == movingobject.inair
            {
                let mut killer = get_player_from_global_id(g, g.game_values.flags.screenshakeplayerid);

                if !killer.is_none() {
                    if !g.mode().gameover {
                        g.player(killer).score(g).adjust_score(g, 1);
                    }

                    if_sound_on_play(g, &mut gp.rm.sfx_kicksound);
                    movingobject.as_walking_enemy().unwrap().die_and_drop_shell(g, true, true);

                    g.game_values.flags.screenshakekillscount += 1;

                    if g.player(killer).inair {
                        g.player(killer).killsinrowinair -= 1;
                    }
                }
            }
        }
    }

    //Destroy throw blocks and flip shells over
    let n1 = g.objectcontainer[1].list().len();
    for iObj in 0..n1 {
        let mut obj = g.objectcontainer[1].list()[iObj];
        if let Some(mo) = obj.as_io_moving_object() {
            let mut movingobject = mo.as_mo_ptr();
            if g.game_values.flags.screenshakekillinair == movingobject.inair {
                let t = movingobject.get_moving_object_type();
                if t == movingobject_shell {
                    let shell: &mut CO_Shell = movingobject.as_any().downcast_mut::<CO_Shell>().unwrap();
                    if shell.frozen || shell.owner.is_none() || g.player(shell.owner).inair == g.game_values.flags.screenshakekillinair {
                        shell.flip(g); //also breaks shells if frozen
                    }
                } else if t == movingobject_throwblock {
                    let throwblock: &mut CO_ThrowBlock = movingobject.as_any().downcast_mut::<CO_ThrowBlock>().unwrap();
                    if throwblock.frozen || throwblock.owner.is_none() || g.player(throwblock.owner).inair == g.game_values.flags.screenshakekillinair {
                        movingobject.die(g);
                    }
                } else if t == movingobject_throwbox {
                    let throwbox: &mut CO_ThrowBox = movingobject.as_any().downcast_mut::<CO_ThrowBox>().unwrap();
                    if throwbox.frozen {
                        movingobject.die(g);
                    }
                } else if t == movingobject_pirhanaplant {
                    let plant: &mut MO_PirhanaPlant = movingobject.as_any().downcast_mut::<MO_PirhanaPlant>().unwrap();
                    plant.kill_plant(g);
                } else if t == movingobject_bulletbill {
                    movingobject.die(g);
                }
            }
        }
    }

    //Add kills in row for kills from pow and mod
    if g.game_values.flags.screenshakekillscount > 1 && g.game_values.awardstyle != AwardStyle::None {
        g.game_values.flags.screenshakekillscount = 0;

        let mut killer2 = get_player_from_global_id(g, g.game_values.flags.screenshakeplayerid);

        if !killer2.is_none() {
            g.player(killer2).add_kills_in_row_in_air_award(g);
        }
    }
}

impl GameplayState {
    //Move the screen in a small circle
    fn spin_screen(&mut self, g: &mut Game) {
        if !g.game_values.spinscreen {
            return;
        }

        if self.spindirection == 0 || self.spindirection == 2 {
            self.spintimer += 1;
            if self.spintimer >= 300 {
                self.spindirection += 1;
                self.spintimer = 0;
            }
        } else if self.spindirection == 1 {
            self.spinspeed += 0.0008f32;

            if self.spinspeed >= 0.05f32 {
                self.spinspeed = 0.05f32;
                self.spindirection += 1;
            }
        } else {
            self.spinspeed -= 0.0008f32;

            if self.spinspeed <= -0.05f32 {
                self.spinspeed = -0.05f32;
                self.spindirection = 0;
            }
        }

        self.spinangle += self.spinspeed;

        if self.spinangle >= TWO_PI {
            self.spinangle -= TWO_PI;
        } else if self.spinangle < 0.0f32 {
            self.spinangle += TWO_PI;
        }

        let mut shakey: f32 = self.spinspeed * App::screenWidth as f32 * sinf(self.spinangle);
        if shakey < 0.0f32 {
            shakey -= 1.0f32;
        }

        g.x_shake = (self.spinspeed * App::screenWidth as f32 * cosf(self.spinangle)) as i16;
        g.y_shake = shakey as i16;
    }
}

pub fn handle_p2p_collisions(g: &mut Game) {
    let mut gp = g.ptr();
    //Player to player collisions
    let mut i: usize = 0;
    while i < g.players.len() {
        let mut player1 = g.players[i];
        assert!(!player1.is_none());
        if g.player(player1).state > PlayerState::Dead {
            let mut j = i + 1;
            while j < g.players.len() {
                let player2 = g.players[j];
                assert!(!player2.is_none());
                if g.player(player2).state > PlayerState::Dead {
                    if coldec_player2player(g, player1, player2) {
                        if g.netplay.active {
                            gp.netplay.client.local_gamehost.send_p2p_collision_event(g, g.player(player1), g.player(player2));
                        }

                        g.player(player1).collides_with_player(g, player2);

                        //if player was killed by another player, continue with next player for collision detection
                        if g.player(player1).state <= PlayerState::Dead {
                            break;
                        }
                    }
                }
                j += 1;
            }
        }
        i += 1;
    }
}

pub fn handle_p2obj_collisions(g: &mut Game) {
    //Collide player with objects
    let nPlayers = g.players.len();
    for iPlayer in 0..nPlayers {
        let mut player = g.players[iPlayer];
        if g.player(player).state != PlayerState::Ready {
            continue;
        }

        //Collide with objects
        for iLayer in 0..3usize {
            let n = g.objectcontainer[iLayer].list().len();
            for iObj in 0..n {
                let obj = g.objectcontainer[iLayer].list()[iObj];
                if !obj.is_dead() {
                    if coldec_player2obj(g, player, obj) {
                        if g.player(player).collides_with_object(g, obj) {
                            break;
                        }
                    }
                }
            }

            //if the object killed the player, then continue with the other players
            if g.player(player).state != PlayerState::Ready {
                break;
            }

            //If player collided with a swap mushroom, the break from colliding with everything else
            if g.game_values.flags.swapplayers {
                return;
            }
        }
    }
}

pub fn handle_obj2obj_collisions(g: &mut Game) {
    for iLayer1 in 0..3usize {
        let iContainerEnd1 = g.objectcontainer[iLayer1].list().len();
        for iObject1 in 0..iContainerEnd1 {
            let mut object1 = g.objectcontainer[iLayer1].list()[iObject1];

            let movingobject1: Ptr<dyn IO_MovingObjectTrait> = match object1.as_io_moving_object() {
                Some(m) => m.as_mo_ptr(),
                None => continue,
            };

            for iLayer2 in iLayer1..3usize {
                let iContainerEnd2 = g.objectcontainer[iLayer2].list().len();
                let start = if iLayer1 == iLayer2 { iObject1 + 1 } else { 0 };
                for iObject2 in start..iContainerEnd2 {
                    let mut object2 = g.objectcontainer[iLayer2].list()[iObject2];

                    if object2.is_dead() {
                        continue;
                    }

                    let movingobject2: Ptr<dyn IO_MovingObjectTrait> = match object2.as_io_moving_object() {
                        Some(m) => m.as_mo_ptr(),
                        None => continue,
                    };

                    let iType1 = movingobject1.get_moving_object_type() as usize;
                    let iType2 = movingobject2.get_moving_object_type() as usize;
                    if g_iCollisionMap[iType1][iType2] != 0 {
                        if coldec_obj2obj(movingobject1.get().as_object_ptr(), movingobject2.get().as_object_ptr()) {
                            collisionhandler_o2o(g, movingobject1, movingobject2);
                        }
                    } else if g_iCollisionMap[iType2][iType1] != 0 {
                        if coldec_obj2obj(movingobject2.get().as_object_ptr(), movingobject1.get().as_object_ptr()) {
                            collisionhandler_o2o(g, movingobject2, movingobject1);
                        }
                    }

                    // Returns from the whole function, not just the object1 loop (as in C++).
                    if object1.is_dead() {
                        return;
                    }
                }
            }
        }
    }
}

impl GameplayState {
    fn draw_scoreboard(&mut self, g: &mut Game, iScoreTextOffset: [i16; 4]) {
        let mut gp = g.ptr();
        g.g_iWinningPlayer = -1;

        //Draw scoreboards for all games (except special cases where we have a single player walking the map)
        if g.game_values.singleplayermode == -1 {
            let mut highestScore: i16 = 0;

            let fReverseScoring = g.mode().get_reverse_scoring();
            if fReverseScoring {
                highestScore = 32000;
            }

            for i in 0..g.score_cnt {
                let mut scoreValue: i32 = g.score[i as usize].score as i32;
                if g.mode().gamemode == game_mode_boxes_minigame {
                    scoreValue = g.score[i as usize].subscore[0] as i32;
                }

                if (scoreValue > highestScore as i32 && !fReverseScoring) || (scoreValue < highestScore as i32 && fReverseScoring) {
                    highestScore = scoreValue as i16;
                    g.g_iWinningPlayer = i;
                } else if scoreValue == highestScore as i32 {
                    g.g_iWinningPlayer = -1;
                }
            }

            //big end game scoreboard (sorted)
            if g.game_values.flags.showscoreboard {
                let gameovertext: String;
                let winningteam = g.mode().winningteam;
                if winningteam > -1 {
                    if g.game_values.teamcounts[winningteam as usize] == 1 {
                        gameovertext = format!("Player {} Wins!", g.game_values.teamids[winningteam as usize][0] as i32 + 1);
                    } else {
                        gameovertext = format!("Team {} Wins!", winningteam as i32 + 1);
                    }
                } else {
                    gameovertext = "Tie Game".to_string();
                }

                gp.rm.game_font_large.draw_centered(g, App::screenWidth / 2, 90, &gameovertext);
            }

            //in game scoreboards
            for i in 0..g.score_cnt as usize {
                let s = g.score[i].as_ptr();
                let teamcount = g.game_values.teamcounts[i];
                let gm = g.mode().gamemode;
                if gm == game_mode_health || gm == game_mode_collection || gm == game_mode_boxes_minigame {
                    gp.rm.sprite(gp.rm.spr_shade[(teamcount - 1) as usize]).draw(g, s.x as i32, s.y as i32);
                } else {
                    draw_rect(g, &gp.rm.sprite(gp.rm.spr_shade[(teamcount - 1) as usize]), s.x as i32, s.y as i32, 0, 0, 256, 41);
                }

                for k in 0..teamcount as usize {
                    let globalID = g.game_values.teamids[i][k];
                    let gid = globalID as usize;

                    //If player is respawning, draw an animated egg counter
                    if self.respawnCount[gid] > 0 && !g.mode().gameover {
                        self.respawnanimationtimer[gid] += 1;
                        if self.respawnanimationtimer[gid] > 8 {
                            self.respawnanimationtimer[gid] = 0;
                            self.respawnanimationframe[gid] += 32;

                            if self.respawnanimationframe[gid] > 32 {
                                self.respawnanimationframe[gid] = 0;
                            }
                        }

                        let scorex: i16 = (s.x as i32 + Self::scoreoffsets[k] as i32) as i16;
                        let scorey: i16 = (s.y as i32 + 2) as i16;
                        draw_rect(g, &gp.rm.sprite(gp.rm.spr_egg), scorex as i32, scorey as i32, self.respawnanimationframe[gid] as i32, (g.game_values.colorids[gid] as i32) << 5, 32, 32);
                        draw_rect(
                            g,
                            &gp.rm.sprite(gp.rm.spr_eggnumbers),
                            scorex as i32,
                            scorey as i32,
                            ((self.respawnCount[gid] as i32 - 1) >> 1) << 5,
                            (g.game_values.colorids[gid] as i32) << 5,
                            32,
                            32,
                        );
                    } else {
                        //otherwise draw the player's skin in the scoreboard
                        let iScoreboardSprite: i32;
                        if g.mode().gameover {
                            if g.g_iWinningPlayer as i32 != i as i32 {
                                iScoreboardSprite = PGFX_DEADFLYING;
                            } else {
                                iScoreboardSprite = PGFX_JUMPING_R;
                            }
                        } else {
                            iScoreboardSprite = PGFX_STANDING_R;
                        }

                        //Search for player state to display
                        let player = get_player_from_global_id(g, globalID);

                        if !player.is_none() && !g.mode().gameover {
                            let iScoreOffsetX: i16 = (s.x as i32 + Self::scoreoffsets[k] as i32) as i16;
                            let iScoreOffsetY: i16 = (s.y as i32 + 2) as i16;
                            let (ox, oy) = (iScoreOffsetX as i32, iScoreOffsetY as i32);

                            if g.player(player).ownerPlayerID > -1 {
                                draw_rect(g, gp.rm.sprite(gp.rm.spr_ownedtags), ox - 8, oy - 8, g.player(player).ownerColorOffsetX as i32, 0, 48, 48);
                            }

                            draw_rect(g, gp.rm.sprite(g.player(player).get_scoreboard_sprite()[iScoreboardSprite as usize]), ox, oy, g.player(player).iSrcOffsetX as i32, 0, 32, 32);

                            //Display jail if player is jailed
                            if g.player(player).jail.is_active() {
                                draw_rect(g, &gp.rm.sprite(gp.rm.spr_jail), ox - 6, oy - 6, (g.player(player).jail.get_color() as i32 + 1) * 44, 0, 44, 44);
                            }

                            //Display current powerup if player is using one
                            if g.player(player).powerup > 0 {
                                draw_rect(g, &gp.rm.sprite(gp.rm.spr_storedpowerupsmall), ox, oy + 16, Self::g_iPowerupToIcon[(g.player(player).powerup - 1) as usize] as i32, 0, 16, 16);
                            }

                            //Display tanooki powerup if player has it
                            if g.player(player).tanookisuit.is_on() {
                                draw_rect(g, &gp.rm.sprite(gp.rm.spr_storedpowerupsmall), ox + 16, oy + 16, App::screenWidth / 2, 0, 16, 16);
                            }
                        } else {
                            draw_rect(
                                g,
                                &gp.rm.sprite(gp.rm.spr_player[gid][iScoreboardSprite as usize]),
                                s.x as i32 + Self::scoreoffsets[k] as i32,
                                s.y as i32 + 2,
                                0,
                                0,
                                32,
                                32,
                            );
                        }

                        //give crown to player(s) with most kills
                        if g.g_iWinningPlayer as i32 == i as i32 {
                            gp.rm.sprite(gp.rm.spr_crown).draw(g, s.x as i32 + Self::scoreoffsets[k] as i32 + 12, s.y as i32 - 4);
                        }
                    }

                    let storedpowerupid = g.game_values.gamepowerups[gid];

                    //Draw stored powerup
                    if storedpowerupid != -1 {
                        if !g.game_values.flags.swapplayers {
                            draw_rect(
                                g,
                                &gp.rm.sprite(gp.rm.spr_storedpowerupsmall),
                                s.x as i32 + scorepowerupoffsets[(teamcount - 1) as usize][k] as i32,
                                s.y as i32 + 25,
                                storedpowerupid as i32 * 16,
                                0,
                                16,
                                16,
                            );
                        }
                    }
                }

                //Draw hearts for health mode
                if gm == game_mode_health {
                    let iLife: i16 = s.subscore[0];
                    let iMax: i16 = s.subscore[1];
                    let iHeartX: i16 = (s.x as i32 + scorepowerupoffsets[(teamcount - 1) as usize][0] as i32 - 32) as i16;

                    let mut iHeart: i16 = 0;
                    while iHeart < iLife {
                        let dx = iHeartX as i32 + iHeart as i32 * 8;
                        if iHeart == iMax - 1 && iHeart % 2 == 0 {
                            draw_rect(g, &gp.rm.sprite(gp.rm.spr_scorehearts), dx, s.y as i32 + 43, 32, 0, 8, 16);
                        } else {
                            draw_rect(g, &gp.rm.sprite(gp.rm.spr_scorehearts), dx, s.y as i32 + 43, if iHeart % 2 != 0 { 8 } else { 0 }, 0, 8, 16);
                        }
                        iHeart += 1;
                    }

                    let mut iHeart: i16 = iLife;
                    while iHeart < iMax {
                        let dx = iHeartX as i32 + iHeart as i32 * 8;
                        if iHeart == iMax - 1 && iHeart % 2 == 0 {
                            draw_rect(g, &gp.rm.sprite(gp.rm.spr_scorehearts), dx, s.y as i32 + 43, 40, 0, 8, 16);
                        } else {
                            draw_rect(g, &gp.rm.sprite(gp.rm.spr_scorehearts), dx, s.y as i32 + 43, if iHeart % 2 != 0 { 24 } else { 16 }, 0, 8, 16);
                        }
                        iHeart += 1;
                    }
                } else if gm == game_mode_collection {
                    //Draw cards for collection mode
                    //Flash collected cards if 3 have been collected
                    if s.subscore[0] < 3 || s.subscore[2] % 20 < 10 {
                        let iNumCards: i16 = s.subscore[0];
                        let mut iCardValues: i16 = s.subscore[1];
                        let iCardX: i16 = (s.x as i32 + scorepowerupoffsets[(teamcount - 1) as usize][0] as i32 - 20) as i16;

                        for iCard in 0..iNumCards {
                            draw_rect(g, &gp.rm.sprite(gp.rm.spr_scorecards), iCardX as i32 + iCard as i32 * 20, s.y as i32 + 43, (iCardValues as i32 & 3) << 4, 0, 16, 16);
                            iCardValues >>= 2;
                        }
                    }
                } else if gm == game_mode_boxes_minigame {
                    //Draw coins for boxes minigame
                    let iNumCoins: i16 = s.subscore[0];
                    let iCoinX: i16 = (s.x as i32 + scorepowerupoffsets[(teamcount - 1) as usize][0] as i32 - 32) as i16;

                    let mut iCoin: i16 = 0;
                    while iCoin < iNumCoins {
                        draw_rect(g, &gp.rm.sprite(gp.rm.spr_scorecoins), iCoinX as i32 + iCoin as i32 * 16, s.y as i32 + 43, 0, 0, 16, 16);
                        iCoin += 1;
                    }

                    for iEmptyCoin in iCoin..5 {
                        draw_rect(g, &gp.rm.sprite(gp.rm.spr_scorecoins), iCoinX as i32 + iEmptyCoin as i32 * 16, s.y as i32 + 43, 16, 0, 16, 16);
                    }
                }

                let iScoreX: i16 = (s.x as i32 + iScoreTextOffset[i] as i32) as i16;
                let iScoreY: i16 = (s.y as i32 + 4) as i16;
                let (sx, sy) = (iScoreX as i32, iScoreY as i32);

                draw_rect(g, &gp.rm.sprite(gp.rm.spr_scoretext), sx, sy, s.iDigitLeft as i32, if s.iDigitLeft == 0 { 16 } else { 0 }, 16, 16);
                draw_rect(g, &gp.rm.sprite(gp.rm.spr_scoretext), sx + 18, sy, s.iDigitMiddle as i32, if s.iDigitLeft == 0 && s.iDigitMiddle == 0 { 16 } else { 0 }, 16, 16);
                draw_rect(g, &gp.rm.sprite(gp.rm.spr_scoretext), sx + 36, sy, s.iDigitRight as i32, 0, 16, 16);
            }
        }
    }

    fn draw_screen_fade(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if g.game_values.screenfadespeed != 0 {
            gp.g_map.update(g);
            g.game_values.screenfade = (g.game_values.screenfade as i32 + g.game_values.screenfadespeed as i32) as i16;

            if g.game_values.screenfade <= 0 {
                g.game_values.screenfadespeed = 0;
                g.game_values.screenfade = 0;

                //display the mode and goal at the start of the game
                //if (game_values.matchtype == MatchType::QuickGame)
                if g.game_values.startmodedisplay && g.game_values.singleplayermode == -1 {
                    let gm = g.mode();
                    let szMode: String = if gm.goal < 0 {
                        format!("{}  {}: X", gm.get_mode_name(), gm.get_goal_name())
                    } else {
                        format!("{}  {}: {}", gm.get_mode_name(), gm.get_goal_name(), gm.goal)
                    };

                    let iMode = get_mode_icon_index_from_mode(gm.gamemode as i16);

                    g.eyecandy[2].emplace(EC_Announcement::new(
                        Ptr::from_mut(&mut g.rm.game_font_large),
                        g.rm.menu_mode_large,
                        szMode,
                        iMode,
                        130,
                        90,
                    ));
                }
            } else if g.game_values.screenfade >= 255 {
                g.game_values.screenfadespeed = 0;
                g.game_values.screenfade = 255;
            }
        }

        if g.game_values.screenfade > 0 {
            g.rm.sprite(g.rm.menu_shade).setalpha(g.game_values.screenfade as u8);
            gp.rm.sprite(gp.rm.menu_shade).draw(g, 0, 0);
        }
    }

    fn draw_screen_shake_background(&mut self, g: &mut Game) {
        unsafe {
            //Draw black "behind" the game if we are shaking/moving the screen
            if g.y_shake > 0 {
                let rect = Rect { x: 0, y: 0, w: App::screenWidth, h: g.y_shake as i32 };
                crate::common::gfx::blit::fill_rect(g.screen, &rect, 0x0); //fill empty area with black
            } else if g.y_shake < 0 {
                let rect = Rect { x: 0, y: App::screenHeight + g.y_shake as i32, w: App::screenWidth, h: App::screenHeight };
                crate::common::gfx::blit::fill_rect(g.screen, &rect, 0x0); //fill empty area with black
            }

            if g.x_shake > 0 {
                let rect = Rect { x: 0, y: 0, w: g.x_shake as i32, h: App::screenHeight };
                crate::common::gfx::blit::fill_rect(g.screen, &rect, 0x0); //fill empty area with black
            } else if g.x_shake < 0 {
                let rect = Rect { x: App::screenWidth + g.x_shake as i32, y: 0, w: App::screenWidth, h: App::screenHeight };
                crate::common::gfx::blit::fill_rect(g.screen, &rect, 0x0); //fill empty area with black
            }
        }
    }

    fn draw_player_swap(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if g.game_values.flags.swapplayers {
            for i in 0..g.players.len() {
                let mut player = g.players[i];
                g.player(player).drawswap(g);

                let storedpowerupid = g.game_values.gamepowerups[g.player(player).get_global_id() as usize];

                if storedpowerupid != -1 {
                    let iPowerupX: i16;
                    let iPowerupY: i16;

                    if g.game_values.swapstyle == 1 {
                        iPowerupX = if g.game_values.flags.swapplayersblink { g.player(player).iOldPowerupX } else { g.player(player).iNewPowerupX };
                        iPowerupY = if g.game_values.flags.swapplayersblink { g.player(player).iOldPowerupY } else { g.player(player).iNewPowerupY };
                    } else {
                        iPowerupX = ((((g.player(player).iNewPowerupX as i32 - g.player(player).iOldPowerupX as i32) as f32 * g.game_values.flags.swapplayersposition) as i16) as i32
                            + g.player(player).iOldPowerupX as i32) as i16;
                        iPowerupY = ((((g.player(player).iNewPowerupY as i32 - g.player(player).iOldPowerupY as i32) as f32 * g.game_values.flags.swapplayersposition) as i16) as i32
                            + g.player(player).iOldPowerupY as i32) as i16;
                    }

                    draw_rect(g, &gp.rm.sprite(gp.rm.spr_storedpowerupsmall), iPowerupX as i32, iPowerupY as i32, storedpowerupid as i32 * 16, 0, 16, 16);
                }
            }

            if g.game_values.swapstyle == 0 {
                if !g.rm.sfx_skid.is_playing() {
                    if_sound_on_play(g, &mut gp.rm.sfx_skid);
                }
            }

            g.game_values.flags.swapplayersblinkcount += 1;
            if g.game_values.flags.swapplayersblinkcount > 10 {
                g.game_values.flags.swapplayersblinkcount = 0;
                g.game_values.flags.swapplayersblink = !g.game_values.flags.swapplayersblink;
            }

            g.game_values.flags.swapplayersposition += 0.02f32;
            if g.game_values.flags.swapplayersposition >= 1.0f32 {
                g.game_values.flags.swapplayersposition = 0.0f32;
                g.game_values.flags.swapplayers = false;
                g.game_values.screenfade = 0;

                if g.game_values.swapstyle == 0 {
                    gp.rm.sfx_skid.stop(g);
                }

                if_sound_on_play(g, &mut gp.rm.sfx_transform);

                if g.game_values.swapstyle == 1 {
                    for i in 0..g.players.len() {
                        let player = g.players[i];
                        g.eyecandy[2].emplace(EC_SingleAnimation::new(&gp.rm, 
                            g.rm.spr_fireballexplosion,
                            (g.player(player).left_x() as i32 + HALFPW - 16) as i16,
                            (g.player(player).top_y() as i32 + HALFPH - 16) as i16,
                            3,
                            8,
                        ));
                    }
                }
            }
        }
    }

    fn draw_back_layer(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        gp.rm.sprite(gp.rm.spr_backmap[gp.g_iCurrentDrawIndex as usize]).draw(g, 0, 0);

        //draw back eyecandy behind players
        gp.g_map.draw_platforms(g, 0);

        gp.eyecandy[0].draw(g);
        gp.noncolcontainer.draw(g);

        gp.mode().draw_background(g);

        gp.objectcontainer[0].draw(g);
    }

    fn draw_middle_layer(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        gp.g_map.draw_platforms(g, 1);

        if !g.game_values.flags.swapplayers {
            for i in 0..g.players.len() {
                let mut player = g.players[i];
                g.player(player).draw(g);
            }
        }

        gp.eyecandy[1].draw(g);

        gp.objectcontainer[1].draw(g);
    }

    fn draw_front_layer(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        gp.g_map.draw_platforms(g, 2);

        if g.game_values.toplayer {
            gp.g_map.drawfrontlayer(g);
        }

        gp.g_map.draw_warp_locks(g);

        gp.g_map.draw_platforms(g, 3);

        gp.objectcontainer[2].draw(g);
        gp.eyecandy[2].draw(g);
        gp.mode().draw_foreground(g);

        gp.g_map.draw_platforms(g, 4);
    }

    fn draw_wind_meter(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if g.game_values.windaffectsplayers {
            let iDisplayWindMeterY: i16 = if g.game_values.scoreboardstyle == ScoreboardStyle::Bottom { 8 } else { 440 };
            draw_rect(g, &gp.rm.sprite(gp.rm.spr_windmeter), 210, iDisplayWindMeterY as i32, 0, 0, 220, 32);
            draw_rect(
                g,
                &gp.rm.sprite(gp.rm.spr_windmeter),
                ((gp.game_values.flags.gamewindx * 20.0f32) as i16) as i32 + App::screenWidth / 2,
                iDisplayWindMeterY as i32 + 6,
                220,
                0,
                12,
                20,
            );
        }
    }

    fn draw_countdown(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        //Draw countdown start timer
        if self.iCountDownState > 0 && g.game_values.screenfade == 0 {
            let idx = (28 - self.iCountDownState) as usize;
            let rects: &[Rect; 2] = &iCountDownNumbers[iCountDownRectGroup[idx] as usize][iCountDownRectSize[idx] as usize];
            gp.rm.sprite(gp.rm.spr_countdown_numbers).draw_src(g, rects[1].x, rects[1].y, &rects[0]);
        }
    }

    fn draw_spotlights(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if g.game_values.spotlights {
            gp.spotlightManager.draw_spotlights(g);
        }
    }

    fn draw_out_of_screen_indicators(&mut self, g: &mut Game) {
        //draw arrows for being above the top of the screen
        for i in 0..g.players.len() {
            let mut player = g.players[i];
            g.player(player).draw_out_of_screen_indicators(g);
        }
    }

    fn draw_everything(&mut self, g: &mut Game, _iCountDownState: i16, iScoreTextOffset: [i16; 4]) {
        self.draw_back_layer(g);
        self.draw_middle_layer(g);
        self.draw_front_layer(g);

        self.draw_spotlights(g);
        self.draw_scoreboard(g, iScoreTextOffset);
        self.draw_wind_meter(g);
        self.draw_out_of_screen_indicators(g);
        self.draw_countdown(g);
        self.draw_screen_fade(g);
        self.draw_player_swap(g);
        self.draw_screen_shake_background(g);
    }
}

pub fn draw_exit_pause_dialog(g: &mut Game) {
        let mut gp = g.ptr();
    if g.game_values.flags.pausegame {
        gp.rm.sprite(gp.rm.spr_dialog).draw(g, 224, 176);
        gp.rm.menu_font_large.draw_centered(g, App::screenWidth / 2, 194, "Pause");

        let iMode = get_mode_icon_index_from_mode(g.mode().gamemode as i16);

        draw_rect(g, &gp.rm.sprite(gp.rm.menu_mode_large), 304, 224, (iMode as i32) << 5, 0, 32, 32);

        let mut szGoal: String = g.mode().get_goal_name().to_string() + ": ";

        if g.mode().goal == -1 {
            szGoal += "Unlimited";
        } else {
            szGoal += &g.mode().goal.to_string();
        }

        gp.rm.menu_font_large.draw_centered(g, App::screenWidth / 2, 264, &szGoal);
    }

    if g.game_values.flags.exitinggame {
        let sw = App::screenWidth as f32;
        let sh = App::screenHeight as f32;
        gp.rm.sprite(gp.rm.spr_dialog).draw(g, (sw * 0.35f32) as i32, (sh * 0.37f32) as i32);
        gp.rm.menu_font_large.draw_centered(g, (sw * 0.5f32) as i32, (sh * 0.46f32 - (g.rm.menu_font_large.get_height() >> 1) as f32) as i32, "Exit Game");

        draw_rect(g, &gp.rm.sprite(gp.rm.spr_dialogbutton), (sw * 0.37f32) as i32, (sh * 0.52f32) as i32, 0, if gp.game_values.flags.exityes { 34 } else { 0 }, 80, 34);
        draw_rect(g, &gp.rm.sprite(gp.rm.spr_dialogbutton), (sw * 0.51f32) as i32, (sh * 0.52f32) as i32, 0, if gp.game_values.flags.exityes { 0 } else { 34 }, 80, 34);

        gp.rm.menu_font_large.draw(
            g,
            (sw * 0.43f32 - (gp.rm.menu_font_large.get_width("Yes") >> 1) as f32) as i32,
            (sh * 0.56f32 - (gp.rm.menu_font_large.get_height() >> 1) as f32) as i32,
            "Yes",
        );
        gp.rm.menu_font_large.draw(
            g,
            (sw * 0.57f32 - (gp.rm.menu_font_large.get_width("No") >> 1) as f32) as i32,
            (sh * 0.56f32 - (gp.rm.menu_font_large.get_height() >> 1) as f32) as i32,
            "No",
        );
    }
}

pub fn is_pause_allowed(g: &mut Game) -> bool {
    !g.game_values.flags.noexit
}

pub fn is_exit_allowed(g: &mut Game) -> bool {
    if !g.game_values.flags.noexit || g.players.is_empty() {
        return true;
    }

    for player in g.players.iter() {
        if g.game_values.playercontrol[g.player(*player).get_global_id() as usize] == 1 {
            return false;
        }
    }

    true
}

pub fn update_score_board(g: &mut Game) {
    let mut gp = g.ptr();
    if g.game_values.matchtype == MatchType::World {
        //If no one won, then nothing on the world map has changed
        if g.mode().winningteam < 0 {
            return;
        }

        //If this was the last stage, signal that the world is over
        if g.game_values.tourstops[g.game_values.tourstopcurrent].fEndStage {
            g.game_values.tournamentwinner = 1;
            gp.rm.backgroundmusic[4].play(g, true, true);
        }

        //Add up all the winnings so far and determine overall place in the standings
        for iScore in 0..g.score_cnt as usize {
            g.game_values.tournament_scores[iScore].wins = 0;
        }

        //Assign the order that players will show up on the scoreboard (ordered by score)
        for iMyScore in 0..g.score_cnt as usize {
            for iTheirScore in 0..g.score_cnt as usize {
                if g.game_values.tournament_scores[iMyScore].total > g.game_values.tournament_scores[iTheirScore].total {
                    g.game_values.tournament_scores[iTheirScore].wins += 1;
                }
            }
        }

        //Give players the item bonuses that were won
        let tourStop = g.game_values.current_tour_stop();

        for iScore in 0..g.score_cnt as usize {
            for iBonus in 0..tourStop.iNumBonuses as usize {
                if tourStop.wsbBonuses[iBonus].iWinnerPlace == g.score[iScore].place {
                    if g.game_values.worldpowerupcount[iScore] < 32 {
                        let c = g.game_values.worldpowerupcount[iScore] as usize;
                        g.game_values.worldpowerupcount[iScore] += 1;
                        g.game_values.worldpowerups[iScore][c] = tourStop.wsbBonuses[iBonus].iBonus;
                    } else {
                        g.game_values.worldpowerups[iScore][31] = tourStop.wsbBonuses[iBonus].iBonus;
                    }
                }
            }
        }

        //Add powerups to player's world item inventory that they held at the end of the game
        for iPlayer in 0..MAX_PLAYERS as i16 {
            if g.game_values.gamepowerups[iPlayer as usize] != -1 {
                let iTeamId = lookup_team_id(g, iPlayer) as usize;
                if g.game_values.worldpowerupcount[iTeamId] < 32 {
                    let c = g.game_values.worldpowerupcount[iTeamId] as usize;
                    g.game_values.worldpowerupcount[iTeamId] += 1;
                    g.game_values.worldpowerups[iTeamId][c] = g.game_values.gamepowerups[iPlayer as usize];
                }
            }
        }
    } else if g.game_values.matchtype == MatchType::Tour {
        if g.mode().winningteam < 0 {
            return;
        }

        //For this game, set the player's place as the type of win
        for iScore in 0..g.score_cnt as usize {
            let cur = g.game_values.tourstopcurrent;
            g.game_values.tournament_scores[iScore].type_[cur] = g.score[iScore].place;
        }

        //Add up all the winnings so far and determine overall place in tour
        for iScore in 0..g.score_cnt as usize {
            g.game_values.tournament_scores[iScore].total = 0;

            let mut iTourStop: usize = 0;
            while iTourStop <= g.game_values.tourstopcurrent {
                let t = g.game_values.tournament_scores[iScore].type_[iTourStop] as i32;
                let pts = g.game_values.tourstops[iTourStop].iPoints as i32;
                g.game_values.tournament_scores[iScore].total = (g.game_values.tournament_scores[iScore].total as i32 + (3 - t) * pts) as i16;
                iTourStop += 1;
            }

            g.game_values.tournament_scores[iScore].wins = 0;
        }

        for iMyScore in 0..g.score_cnt as usize {
            for iTheirScore in 0..g.score_cnt as usize {
                if g.game_values.tournament_scores[iMyScore].total > g.game_values.tournament_scores[iTheirScore].total {
                    g.game_values.tournament_scores[iTheirScore].wins += 1;
                }
            }
        }

        g.game_values.tourstopcurrent += 1;
        if g.game_values.tourstopcurrent >= g.game_values.tourstops.len() {
            //Calculate Tour Winner by counting up 1st, 2nd, 3rd, and 4th place wins

            let mut iWinningTeam: i16 = -2; //Set winning team to -2 to signify a tie between teams
            let mut iWinningScore: i16 = 0;

            for iScore in 0..g.score_cnt {
                let total = g.game_values.tournament_scores[iScore as usize].total;
                if total > iWinningScore {
                    //New winner found
                    iWinningTeam = iScore;
                    iWinningScore = total;
                } else if total == iWinningScore {
                    //Winning position tied
                    iWinningTeam = -2;
                }
            }

            g.game_values.tournamentwinner = iWinningTeam;
            gp.rm.backgroundmusic[4].play(g, true, true);
        }
    } else if g.game_values.matchtype == MatchType::Tournament {
        let mut maxScore: i16 = -1; //Max score for game
        let mut maxTeam: i16 = -1; //Team ID with the max score -> reset to -1 if two teams tied for win

        for i in 0..g.score_cnt {
            let sc = g.score[i as usize].score;
            if sc > maxScore {
                maxScore = sc;
                maxTeam = i;
            } else if sc == maxScore {
                maxTeam = -1;
            }
        }

        if maxTeam > -1 {
            let mt = maxTeam as usize;
            let wins = g.game_values.tournament_scores[mt].wins as usize;
            g.game_values.tournament_scores[mt].type_[wins] = g.currentgamemode;

            g.game_values.tournament_scores[mt].wins += 1;
            if g.game_values.tournament_scores[mt].wins >= g.game_values.tournamentgames {
                g.game_values.tournamentwinner = maxTeam;

                if g.game_values.music {
                    gp.rm.backgroundmusic[4].play(g, true, true);
                }
            }
        }
    }
}

/// NOTE: The elements are ordered!
pub type MapBlockType = i32;
pub const MapBlock_BrickYellow: MapBlockType = 0;
pub const MapBlock_Powerup: MapBlockType = 1;
pub const MapBlock_Donut: MapBlockType = 2;
pub const MapBlock_Flip: MapBlockType = 3;
pub const MapBlock_Bounce: MapBlockType = 4;
pub const MapBlock_NoteGray: MapBlockType = 5;
pub const MapBlock_BrickBlue: MapBlockType = 6;
pub const MapBlock_SwitchToggleRed: MapBlockType = 7;
pub const MapBlock_SwitchToggleGreen: MapBlockType = 8;
pub const MapBlock_SwitchToggleYellow: MapBlockType = 9;
pub const MapBlock_SwitchToggleBlue: MapBlockType = 10;
pub const MapBlock_SwitchBlockRed: MapBlockType = 11;
pub const MapBlock_SwitchBlockGreen: MapBlockType = 12;
pub const MapBlock_SwitchBlockYellow: MapBlockType = 13;
pub const MapBlock_SwitchBlockBlue: MapBlockType = 14;
pub const MapBlock_View: MapBlockType = 15;
pub const MapBlock_BrickRed: MapBlockType = 16;
pub const MapBlock_NoteRed: MapBlockType = 17;
pub const MapBlock_NoteBlue: MapBlockType = 18;
pub const MapBlock_BrickGray: MapBlockType = 19;
pub const MapBlock_WpnBreakFireball: MapBlockType = 20;
pub const MapBlock_WpnBreakFeather: MapBlockType = 21;
pub const MapBlock_WpnBreakShell: MapBlockType = 22;
pub const MapBlock_WpnBreakBomb: MapBlockType = 23;
pub const MapBlock_WpnBreakBoomerang: MapBlockType = 24;
pub const MapBlock_WpnBreakHammer: MapBlockType = 25;
pub const MapBlock_WpnBreakKuriboShoe: MapBlockType = 26;
pub const MapBlock_WpnBreakPWings: MapBlockType = 27;
pub const MapBlock_WpnBreakStar: MapBlockType = 28;
pub const MapBlock_WpnBreakLeaf: MapBlockType = 29;

/// `new T(...)` returned as `IO_Block*`.
fn new_block<T: IO_BlockTrait + 'static>(b: T) -> Ptr<dyn IO_BlockTrait> {
    Ptr::from_box(Box::new(b) as Box<dyn IO_BlockTrait>)
}

pub fn spawn_map_block(g: &mut Game, typeId: i16, drawPos: Vec2s, objdata: &MapBlock, switchStates: &[i16; 4]) -> Ptr<dyn IO_BlockTrait> {
        let mut gp = g.ptr();
        let mut rm = g.rm;
        let mut gp = g.ptr();
    let wbb = || gp.get().rm.spr_weaponbreakableblock;
    match typeId as i32 {
        MapBlock_BrickYellow => new_block(B_BreakableBlock::new(g, rm.spr_breakableblock, drawPos, 4, 10)),
        MapBlock_Powerup => new_block(B_PowerupBlock::new(g, gp.rm.spr_powerupblock, drawPos, 4, 10, objdata.fHidden, &objdata.iSettings)),
        MapBlock_Donut => new_block(B_DonutBlock::new(g, rm.spr_donutblock, drawPos)),
        MapBlock_Flip => new_block(B_FlipBlock::new(g, rm.spr_flipblock, drawPos, objdata.fHidden)),
        MapBlock_Bounce => new_block(B_BounceBlock::new(g, rm.spr_bounceblock, drawPos, objdata.fHidden)),
        MapBlock_NoteGray => new_block(B_NoteBlock::new(g, rm.spr_noteblock, drawPos, 4, 10, NoteBlockType::Gray, objdata.fHidden)),
        MapBlock_BrickBlue => new_block(B_ThrowBlock::new(g, rm.spr_throwblock, drawPos, 4, 10, ThrowBlockType::Blue)),
        MapBlock_SwitchToggleRed => new_block(B_OnOffSwitchBlock::new(g, rm.spr_switchblocks, drawPos, SwitchColor::Red, switchStates[0])),
        MapBlock_SwitchToggleGreen => new_block(B_OnOffSwitchBlock::new(g, rm.spr_switchblocks, drawPos, SwitchColor::Green, switchStates[1])),
        MapBlock_SwitchToggleYellow => new_block(B_OnOffSwitchBlock::new(g, rm.spr_switchblocks, drawPos, SwitchColor::Yellow, switchStates[2])),
        MapBlock_SwitchToggleBlue => new_block(B_OnOffSwitchBlock::new(g, rm.spr_switchblocks, drawPos, SwitchColor::Blue, switchStates[3])),
        MapBlock_SwitchBlockRed => new_block(B_SwitchBlock::new(g, rm.spr_switchblocks, drawPos, SwitchColor::Red, objdata.iSettings[0])),
        MapBlock_SwitchBlockGreen => new_block(B_SwitchBlock::new(g, rm.spr_switchblocks, drawPos, SwitchColor::Green, objdata.iSettings[0])),
        MapBlock_SwitchBlockYellow => new_block(B_SwitchBlock::new(g, rm.spr_switchblocks, drawPos, SwitchColor::Yellow, objdata.iSettings[0])),
        MapBlock_SwitchBlockBlue => new_block(B_SwitchBlock::new(g, rm.spr_switchblocks, drawPos, SwitchColor::Blue, objdata.iSettings[0])),
        MapBlock_View => new_block(B_ViewBlock::new(g, gp.rm.spr_viewblock, drawPos, objdata.fHidden, &objdata.iSettings)),
        MapBlock_BrickRed => new_block(B_ThrowBlock::new(g, rm.spr_throwblock, drawPos, 4, 10, ThrowBlockType::Red)),
        MapBlock_NoteRed => new_block(B_NoteBlock::new(g, rm.spr_noteblock, drawPos, 4, 10, NoteBlockType::Red, objdata.fHidden)),
        MapBlock_NoteBlue => new_block(B_NoteBlock::new(g, rm.spr_noteblock, drawPos, 4, 10, NoteBlockType::Blue, objdata.fHidden)),
        MapBlock_BrickGray => new_block(B_ThrowBlock::new(g, rm.spr_throwblock, drawPos, 4, 10, ThrowBlockType::Gray)),
        MapBlock_WpnBreakFireball => new_block(B_WeaponBreakableBlock::new(g, wbb(), drawPos, WeaponDamageType::Fireball)),
        MapBlock_WpnBreakFeather => new_block(B_WeaponBreakableBlock::new(g, wbb(), drawPos, WeaponDamageType::Feather)),
        MapBlock_WpnBreakShell => new_block(B_WeaponBreakableBlock::new(g, wbb(), drawPos, WeaponDamageType::Shell)),
        MapBlock_WpnBreakBomb => new_block(B_WeaponBreakableBlock::new(g, wbb(), drawPos, WeaponDamageType::Bomb)),
        MapBlock_WpnBreakBoomerang => new_block(B_WeaponBreakableBlock::new(g, wbb(), drawPos, WeaponDamageType::Boomerang)),
        MapBlock_WpnBreakHammer => new_block(B_WeaponBreakableBlock::new(g, wbb(), drawPos, WeaponDamageType::Hammer)),
        MapBlock_WpnBreakKuriboShoe => new_block(B_WeaponBreakableBlock::new(g, wbb(), drawPos, WeaponDamageType::KuriboShoe)),
        MapBlock_WpnBreakPWings => new_block(B_WeaponBreakableBlock::new(g, wbb(), drawPos, WeaponDamageType::PWings)),
        MapBlock_WpnBreakStar => new_block(B_WeaponBreakableBlock::new(g, wbb(), drawPos, WeaponDamageType::Star)),
        MapBlock_WpnBreakLeaf => new_block(B_WeaponBreakableBlock::new(g, wbb(), drawPos, WeaponDamageType::Leaf)),
        _ => Ptr::null(),
    }
}

pub fn load_map_objects(g: &mut Game, fPreview: bool) {
        let mut gp = g.ptr();
        let mut rm = g.rm;
    crate::smw::objecthazard::load_map_hazards(g, fPreview);

    //Clear all the previous switch settings
    for iSwitch in 0..8usize {
        g.g_map.switchBlocks[iSwitch].clear();
    }

    //Add blocks (breakable, note, switch, throwable, etc)
    for x in 0..MAPWIDTH as i16 {
        for y in 0..MAPHEIGHT as i16 {
            let (xu, yu) = (x as usize, y as usize);
            let typeId: i16 = g.g_map.objectdata[xu][yu].iType;
            let objdata = g.g_map.objectdata[xu][yu];
            let switches = g.g_map.iSwitches;
            let mut block = spawn_map_block(g, typeId, Vec2s::new((x as i32 * 32) as i16, (y as i32 * 32) as i16), &objdata, &switches);
            let mut id = crate::smw::object_container::ObjectId::NONE;
            if !block.is_null() {
                let obj = block.as_object_ptr();
                if gp.noncolcontainer.add_dyn(g, obj) {
                    id = obj.id;
                }
            }
            g.g_map.blockdata[xu][yu] = id;

            match typeId as i32 {
                MapBlock_SwitchToggleRed
                | MapBlock_SwitchToggleGreen
                | MapBlock_SwitchToggleYellow
                | MapBlock_SwitchToggleBlue
                | MapBlock_SwitchBlockRed
                | MapBlock_SwitchBlockGreen
                | MapBlock_SwitchBlockYellow
                | MapBlock_SwitchBlockBlue => {
                    g.g_map.switchBlocks[(typeId as i32 - MapBlock_SwitchToggleRed) as usize].push(id);
                }
                _ => {}
            }
        }
    }

    //Scan for throw box objects and add items to them if approprate
    //Add special coins to them for the boxes minigame
    let mut iCountWeight: i16 = 0;
    for iPowerup in 0..NUM_POWERUPS as usize {
        iCountWeight = (iCountWeight as i32 + g.game_values.powerupweights[iPowerup] as i32) as i16;
    }

    let mut iThrowBoxCount: i16 = 0;
    let mut fBoxHasCoin: Option<Vec<bool>> = None;

    if g.mode().gamemode == game_mode_boxes_minigame {
        for item in g.g_map.mapitems.iter() {
            if item.itype == 5 {
                iThrowBoxCount += 1;
            }
        }

        let mut boxes = vec![false; iThrowBoxCount.max(0) as usize];

        //Randomly choose boxes to put 5 coins in
        let mut iItem: i16 = 0;
        while iItem < 5 && iItem < iThrowBoxCount {
            let mut iBoxIndex = RANDOM_INT(gp.get(), iThrowBoxCount as i32) as i16;

            while boxes[iBoxIndex as usize] {
                iBoxIndex += 1;
                if iBoxIndex >= iThrowBoxCount {
                    iBoxIndex = 0;
                }
            }

            boxes[iBoxIndex as usize] = true;
            iItem += 1;
        }
        fBoxHasCoin = Some(boxes);

        //If map has less than 5 boxes, then insert coins into map in random locations
        let iExtraCoinsNeeded: i16 = 5 - iThrowBoxCount;
        for _iExtraCoin in 0..iExtraCoinsNeeded {
            let obj = Ptr::new_box(MO_Coin::new(g, gp.get().rm.spr_coin, Vec2f::zero(), Vec2s::zero(), 2, -1, 2, 0, true));
            gp.get().objectcontainer[1].add(g, obj);
        }
    }

    //Add map objects like springs, shoes and spikes
    let mut iAddThrowBoxIndex: i16 = 0;
    let nItems = g.g_map.mapitems.len();
    for iMapItem in 0..nItems {
        let item = g.g_map.mapitems[iMapItem];
        let iType: i16 = item.itype as i16;
        let pos = Vec2s::new((item.ix as i32 * 32) as i16, (item.iy as i32 * 32) as i16);

        if iType == 0 {
            let obj = Ptr::new_box(CO_Spring::new(g, rm.spr_spring, pos, false));
            gp.objectcontainer[1].add(g, obj);
        } else if iType == 1 {
            let obj = Ptr::new_box(CO_Spike::new(g, rm.spr_spike, pos));
            gp.objectcontainer[1].add(g, obj);
        } else if iType == 2 {
            let obj = Ptr::new_box(CO_KuriboShoe::new(g, rm.spr_kuriboshoe, pos, false));
            gp.objectcontainer[1].add(g, obj);
        } else if iType == 3 {
            let obj = Ptr::new_box(CO_Spring::new(g, rm.spr_spring, pos, true));
            gp.objectcontainer[1].add(g, obj);
        } else if iType == 4 {
            let obj = Ptr::new_box(CO_KuriboShoe::new(g, rm.spr_kuriboshoe, pos, true));
            gp.objectcontainer[1].add(g, obj);
        } else if iType == 5 {
            let mut iItem: i16 = NO_POWERUP as i16;
            if !fPreview {
                let gm = g.mode().gamemode;
                let gms = &g.game_values.gamemodesettings;
                if gm == game_mode_boxes_minigame {
                    if fBoxHasCoin.as_ref().unwrap()[iAddThrowBoxIndex as usize] {
                        iItem = MINIGAME_COIN as i16;
                    }
                } else if gm == game_mode_health && (RANDOM_INT(gp.get(), 100) as i16) < gms.health.percentextralife {
                    iItem = HEALTH_POWERUP as i16;
                } else if (gm == game_mode_timelimit && (RANDOM_INT(gp.get(), 100) as i16) < gms.time.percentextratime)
                    || (gm == game_mode_star && (RANDOM_INT(gp.get(), 100) as i16) < gms.star.percentextratime)
                {
                    iItem = TIME_POWERUP as i16;
                } else if (gm == game_mode_coins && (RANDOM_INT(gp.get(), 100) as i16) < gms.coins.percentextracoin)
                    || (gm == game_mode_greed && (RANDOM_INT(gp.get(), 100) as i16) < gms.greed.percentextracoin)
                {
                    iItem = COIN_POWERUP as i16;
                } else if gm == game_mode_jail && (RANDOM_INT(gp.get(), 100) as i16) < gms.jail.percentkey {
                    iItem = JAIL_KEY_POWERUP as i16;
                } else if iCountWeight > 0 && (RANDOM_INT(gp.get(), 100)) < 40 {
                    let iRandPowerup: i32 = RANDOM_INT(gp.get(), iCountWeight as i32) + 1;
                    iItem = 0;

                    let mut iPowerupWeightCount: i32 = g.game_values.powerupweights[iItem as usize] as i32;

                    while iPowerupWeightCount < iRandPowerup {
                        iItem += 1;
                        iPowerupWeightCount += g.game_values.powerupweights[iItem as usize] as i32;
                    }
                }
            }

            let obj = Ptr::new_box(CO_ThrowBox::new(g, rm.spr_throwbox, pos, iItem));
            gp.objectcontainer[1].add(g, obj);
            iAddThrowBoxIndex += 1;
        }
    }

    drop(fBoxHasCoin);

    //Set all the 1x1 gaps up so players can run across them
    let mut map = g.g_map.ptr();
    map.update_all_tile_gaps(&g.noncolcontainer);
}

pub fn clean_up(g: &mut Game) {
    let mut gp = g.ptr();
    net_random::end_game(g);
    //delete object list
    for player in std::mem::take(&mut *g.players) {
        g.players.remove(player);
    }

    g.eyecandy[0].clean();
    g.eyecandy[1].clean();
    g.eyecandy[2].clean();
    g.spotlightManager.clear_spotlights();

    g.noncolcontainer.clean();

    g.objectcontainer[0].clean();
    g.objectcontainer[1].clean();
    g.objectcontainer[2].clean();

    load_map_objects(g, true);
    g.g_map.clear_warp_locks();
    gp.g_map.reset_platforms(g);

    //Stop all game sounds
    sfx_stopallsounds(g);
    gp.rm.sfx_invinciblemusic.stop(g);
    gp.rm.sfx_slowdownmusic.stop(g);

    g.x_shake = 0;
    g.y_shake = 0;
}

/// Returns true on exit.
pub fn update_exit_pause_dialog(g: &mut Game, iCountDownState: i16) -> bool {
        let mut gp = g.ptr();
    if g.game_values.screenfade == 0 && iCountDownState <= COUNTDOWN_START_INDEX {
        //If the cancel button is pressed
        if g.game_values.flags.forceexittimer > 0 {
            g.game_values.flags.forceexittimer -= 1;
            if g.game_values.flags.forceexittimer <= 0 {
                g.game_values.appstate = AppState::EndGame;
                g.game_values.screenfade = 8;
                g.game_values.screenfadespeed = 8;
            }
        }

        for iPlayer in 0..4usize {
            let playerKeys: Ptr<COutputControl> = Ptr::from_mut(&mut g.game_values.playerInput.outputControls[iPlayer]);

            //If the start key is pressed (pause key)
            if playerKeys.game_start().fPressed && is_pause_allowed(g) {
                if !g.game_values.flags.showscoreboard && !g.game_values.flags.exitinggame {
                    g.game_values.flags.pausegame = !g.game_values.flags.pausegame;

                    if g.game_values.flags.pausegame {
                        g.rm.sprite(g.rm.menu_shade).setalpha(App::menuTransparency as u8);
                        gp.rm.sprite(gp.rm.menu_shade).draw(g, 0, 0);

                        //Stop the pwings sound if it is on
                        if g.rm.sfx_flyingsound.is_playing() {
                            gp.rm.sfx_flyingsound.stop(g);
                        }
                    }

                    if_sound_on_play(g, &mut gp.rm.sfx_pause);
                }
            }

            if (playerKeys.game_cancel().fPressed || (playerKeys.game_start().fPressed && g.mode().gameover)) && is_exit_allowed(g) {
                if g.mode().gameover {
                    if g.game_values.matchtype == MatchType::Tour || g.game_values.matchtype == MatchType::Tournament {
                        update_score_board(g);
                    }

                    clean_up(g);
                    g.game_values.appstate = AppState::Menu;

                    return true;
                } else if !g.game_values.flags.pausegame && !g.game_values.flags.exitinggame {
                    g.rm.sprite(g.rm.menu_shade).setalpha(App::menuTransparency as u8);
                    gp.rm.sprite(gp.rm.menu_shade).draw(g, 0, 0);
                    g.game_values.flags.exitinggame = true;

                    //Reset the keys each time we switch from menu to game and back
                    g.game_values.playerInput.reset_keys();
                }
            }

            //Deal with input to game exit dialog box
            if g.game_values.flags.exitinggame {
                if playerKeys.menu_left().fPressed {
                    g.game_values.flags.exityes = true;
                } else if playerKeys.menu_right().fPressed {
                    g.game_values.flags.exityes = false;
                }

                if playerKeys.menu_select().fPressed {
                    if g.game_values.flags.exityes {
                        clean_up(g);
                        g.game_values.flags.exitinggame = false;
                        g.game_values.flags.exityes = false;
                        g.game_values.appstate = AppState::Menu;
                        return true;
                    } else {
                        g.game_values.flags.exitinggame = false;

                        //Reset the keys each time we switch from menu to game and back
                        g.game_values.playerInput.reset_keys();
                    }
                }
            }
        }
    }

    false
}

pub fn update_screen_shake(g: &mut Game) {
    if g.y_shake > 0 && !g.game_values.spinscreen {
        g.y_shake -= CRUNCHVELOCITY as i16;

        if g.y_shake < 0 {
            g.y_shake = 0;
        }
    }
}

// updates the bullet bills spawned by a powerup
pub fn update_bullet_bill_powerup(g: &mut Game) {
    for iPlayer in 0..4i16 {
        let p = iPlayer as usize;
        if g.game_values.bulletbilltimer[p] > 0 {
            g.game_values.bulletbilltimer[p] -= 1;

            g.game_values.bulletbillspawntimer[p] -= 1;
            if g.game_values.bulletbillspawntimer[p] <= 0 && !net_random::event(g, Ev::BulletBills, &[p as i32]) {
                net_bullet_bill(g, p);
            }
        }
    }
}

pub fn net_bullet_bill(g: &mut Game, p: usize) {
        let mut gp = g.ptr();
    g.game_values.bulletbillspawntimer[p] = (RANDOM_INT(gp.get(), 20) + 25) as i16;
    let speed: f32 = ((RANDOM_INT(gp.get(), 21) + 20) as f32) / 10.0f32;
    let posy = RANDOM_INT(gp.get(), 448) as i16;
    let vel = if RANDOM_INT(gp.get(), 2) != 0 { speed } else { -speed };
    let obj = Ptr::new_box(MO_BulletBill::new(
        g,
        gp.get().rm.spr_bulletbill,
        gp.get().rm.spr_bulletbilldead,
        Vec2s::new(0, posy),
        vel,
        p as i16,
        false,
    ));
    gp.get().objectcontainer[2].add(g, obj);
    if_sound_on_play(g, &mut gp.rm.sfx_bulletbillsound);
}

// scrolling to center at the end of game
pub fn update_scoreboard_animation(g: &mut Game) {
    if g.game_values.flags.showscoreboard {
        if g.game_values.flags.scorepercentmove < 1.0f32 {
            g.game_values.flags.scorepercentmove += 0.01f32;

            if g.game_values.flags.scorepercentmove >= 1.0f32 {
                g.game_values.flags.scorepercentmove = 1.0f32;
            }
        } else {
            g.game_values.flags.scorepercentmove = 1.0f32;
        }

        for i in 0..g.score_cnt as usize {
            let mut s = g.score[i].as_ptr();
            s.x = ((((s.destx as i32 - s.fromx as i32) as f32 * g.game_values.flags.scorepercentmove) as i16) as i32 + s.fromx as i32) as i16;
            s.y = ((((s.desty as i32 - s.fromy as i32) as f32 * g.game_values.flags.scorepercentmove) as i16) as i32 + s.fromy as i32) as i16;
        }
    }
}

pub fn set_game_mode_settings_from_menu(g: &mut Game) {
    //If this is a tour stop and the tour has settings in it, use those.  Otherwise use the menu settings.
    if g.game_values.tourstopcurrent < g.game_values.tourstops.len()
        && g.game_values.tourstops[g.game_values.tourstopcurrent].fUseSettings
        && (g.game_values.matchtype == MatchType::Tour || g.game_values.matchtype == MatchType::World)
    {
        let settings: GameModeSettings = g.game_values.tourstops[g.game_values.tourstopcurrent].gmsSettings.clone();
        g.game_values.gamemodesettings = settings;
    } else {
        let settings: GameModeSettings = g.game_values.gamemodemenusettings.clone();
        g.game_values.gamemodesettings = settings;
    }
}

pub fn play_sfx(g: &mut Game) {
        let mut gp = g.ptr();
    //Play sound for skidding players
    if g.game_values.flags.playskidsound {
        if !g.rm.sfx_skid.is_playing() {
            if_sound_on_play(g, &mut gp.rm.sfx_skid);
        }
    } else if g.rm.sfx_skid.is_playing() {
        gp.rm.sfx_skid.stop(g);
    }

    //Play sound for players using PWings
    if g.game_values.flags.playflyingsound {
        if !g.rm.sfx_flyingsound.is_playing() {
            if_sound_on_play(g, &mut gp.rm.sfx_flyingsound);
        }
    } else if g.rm.sfx_flyingsound.is_playing() {
        gp.rm.sfx_flyingsound.stop(g);
    }
}

pub fn play_music(g: &mut Game) {
        let mut gp = g.ptr();
    //Make sure music and sound effects keep playing
    if g.game_values.flags.slowdownon != -1 {
        if !g.rm.sfx_slowdownmusic.is_playing() {
            if_sound_on_play(g, &mut gp.rm.sfx_slowdownmusic);
        }
    } else if g.rm.sfx_slowdownmusic.is_playing() {
        gp.rm.sfx_slowdownmusic.stop(g);
    }

    if g.game_values.flags.playinvinciblesound && g.game_values.musicvolume > 0 {
        if !g.rm.sfx_invinciblemusic.is_playing() && !g.rm.sfx_timewarning.is_playing() && !gp.rm.backgroundmusic[0].is_playing(g) {
            if_sound_on_play(g, &mut gp.rm.sfx_invinciblemusic);
        }
    } else if g.rm.sfx_invinciblemusic.is_playing() {
        gp.rm.sfx_invinciblemusic.stop(g);
    }

    //If no background music is playing, then play some
    if !gp.rm.backgroundmusic[0].is_playing(g) && !g.rm.sfx_invinciblemusic.is_playing() && !g.rm.sfx_timewarning.is_playing() && !g.mode().gameover {
        if g.game_values.playnextmusic {
            let mapname = g.maplist.current_shortmapname().to_string();
            g.musiclist.set_next_music(music_category(g.g_map.musicCategoryID), &mapname, &g.g_map.szBackgroundFile);
            let path = g.musiclist.current_music().clone();
            g.rm.backgroundmusic[0] = load_music(g, &path);
        }

        gp.rm.backgroundmusic[0].play(g, g.game_values.playnextmusic, false);
    }
}

pub fn play_next_music_track(g: &mut Game) {
    let mut gp = g.ptr();
    if g.mode().gameover || g.game_values.flags.playinvinciblesound || g.rm.sfx_timewarning.is_playing() {
        return;
    }

    gp.rm.backgroundmusic[0].stop(g);
    let mapname = g.maplist.current_shortmapname().to_string();
    g.musiclist.set_next_music(music_category(g.g_map.musicCategoryID), &mapname, &g.g_map.szBackgroundFile);
    let path = g.musiclist.current_music().clone();
    g.rm.backgroundmusic[0] = load_music(g, &path);
    gp.rm.backgroundmusic[0].play(g, g.game_values.playnextmusic, false);
}

impl GameplayState {
    fn handle_input(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let menu_controls = if g.game_values.flags.exitinggame { 1 } else { 0 };
        g.game_values.playerInput.clear_pressed_keys(menu_controls);

        while let Some(input) = crate::smw::input::poll(gp.get()) {
            if input.event == InputEvent::Quit {
                clean_up(g);
                g.game_values.appstate = AppState::Quit;
                return;
            } else if let InputEvent::Key { key, mods, down: true, .. } = input.event {
                let sym = key.0;
                if (mods as u32) & (KMOD_LALT as u32 | KMOD_RALT as u32) != 0 {
                    if sym == SDLK_F4 {
                        clean_up(g);
                        g.game_values.appstate = AppState::Quit;
                        return;
                    } else if sym == SDLK_RETURN {
                        g.game_values.fullscreen = !g.game_values.fullscreen;
                        gfx_changefullscreen(g, g.game_values.fullscreen);
                        g.blitdest = g.screen;

                        // Not fed to the input, or the Return would also pause the game.
                        continue;
                    }
                }
                if sym == SDLK_F1 {
                    g.game_values.showfps = !g.game_values.showfps;
                } else if sym == SDLK_ESCAPE {
                    g.game_values.playerInput.outputControls[0].game_cancel_mut().fPressed = true;
                } else if sym == SDLK_TAB {
                    play_next_music_track(g);
                } else if sym == SDLK_INSERT {
                    gfx_take_screenshot(g);
                }
            }

            //Feed the player control structures with input data
            //Use menu controls when exit game dialog is up
            let menu_controls = if g.game_values.flags.exitinggame { 1 } else { 0 };
            gp.game_values.playerInput.update_input(g, &input, menu_controls);
        }
    }
}

pub fn should_update(g: &mut Game) -> bool {
    if g.netplay.active {
        return true;
    }

    if g.game_values.flags.pausegame || g.game_values.flags.exitinggame {
        return false;
    }

    true
}

impl GameplayState {
    fn read_network(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if !g.netplay.active {
            return;
        }

        // React to network events
        // The host receives remote input
        g.netplay.gamestate_changed = false;
        gp.netplay.client.update(g);

        let percent_new: f32 = 1.0f32.min(g.netplay.frames_since_last_gamestate as f32 / NET_GAMESTATE_FRAMES_TO_SEND as f32);
        let percent_old: f32 = 1.0 - percent_new;
        debug_assert!(0.0 <= percent_new && percent_new <= 1.0);
        debug_assert!(0.0 <= percent_old && percent_old <= 1.0);
        debug_assert!(0.99 < (percent_old + percent_new) as f64 && (percent_old + percent_new) as f64 <= 1.01);

        if !g.netplay.theHostIsMe {
            for p in 0..g.players.len() {
                let mut player = g.players[p];

                // if local player
                if p == g.netplay.remotePlayerNumber as usize {
                    // save the player data that was cause by the input of the previous frame
                    let mut pdata = Net_IndexedPlayerData::new(g.netplay.current_input_counter);
                    pdata.data.x = g.player(player).fx;
                    pdata.data.y = g.player(player).fy;
                    pdata.data.xvel = g.player(player).velx;
                    pdata.data.yvel = g.player(player).vely;
                    g.netplay.local_playerdata_buffer.push_back(pdata);
                    g.netplay.local_playerdata_store_count = g.netplay.local_playerdata_store_count.wrapping_add(1);
                    let (slot, count) = (g.netplay.current_input_counter as usize, g.netplay.local_playerdata_store_count);
                    g.netplay.local_playerdata_store_time[slot] = count;
                    g.netplay.current_input_counter = g.netplay.current_input_counter.wrapping_add(1);

                    if g.netplay.gamestate_changed {
                        // remove old, saved player data
                        let confirmed_index: u8 = (g.netplay.last_confirmed_input as u8).wrapping_add(1);
                        let confirmed_until = g.netplay.local_playerdata_store_time[confirmed_index as usize];
                        let mut last_popped_local = Net_IndexedPlayerData::new(0xFF);
                        while let Some(front) = g.netplay.local_playerdata_buffer.front() {
                            let iid = front.input_id;
                            if confirmed_until <= g.netplay.local_playerdata_store_time[iid as usize] {
                                break;
                            }

                            last_popped_local = *front;
                            g.netplay.local_playerdata_buffer.pop_front();
                        }

                        // check if we can use the local player data,
                        // or the difference is so big we have to fall back
                        // to the remote, confirmed positions
                        let mut use_remote_pdata = false;
                        if g.netplay.local_playerdata_buffer.is_empty() {
                            use_remote_pdata = true;
                        } else {
                            let pd_remote = g.netplay.latest_playerdata.player[p];

                            if (last_popped_local.data.x - pd_remote.x).abs() > 0.45f32
                                || (last_popped_local.data.y - pd_remote.y).abs() > 0.45f32
                            {
                                use_remote_pdata = true;
                            }
                        }

                        if use_remote_pdata {
                            // the buffered data is now invalid
                            g.netplay.local_playerdata_buffer.clear();

                            // set confirmed data
                            g.player(player).fx = g.netplay.latest_playerdata.player[p].x;
                            g.player(player).fy = g.netplay.latest_playerdata.player[p].y;
                            g.player(player).velx = g.netplay.latest_playerdata.player[p].xvel;
                            g.player(player).vely = g.netplay.latest_playerdata.player[p].yvel;
                        }
                    }
                }
                // for remote players, interpolate
                else if g.netplay.gamestate_received {
                    g.player(player).fx = percent_old * g.netplay.previous_playerdata.player[p].x + percent_new * g.netplay.latest_playerdata.player[p].x;
                    g.player(player).fy = percent_old * g.netplay.previous_playerdata.player[p].y + percent_new * g.netplay.latest_playerdata.player[p].y;
                    g.player(player).velx = percent_old * g.netplay.previous_playerdata.player[p].xvel + percent_new * g.netplay.latest_playerdata.player[p].xvel;
                    g.player(player).vely = percent_old * g.netplay.previous_playerdata.player[p].yvel + percent_new * g.netplay.latest_playerdata.player[p].yvel;
                }
            }

            g.netplay.frames_since_last_gamestate += 1;
        }

        if self.previous_playerKeys != g.game_values.playerInput.outputControls[0] {
            self.previous_playerKeys = g.game_values.playerInput.outputControls[0];
        }

        // Consume the next input from the remote input buffer
        g.netplay.netPlayerInput.clear_game_action_keys();
        if g.netplay.theHostIsMe {
            gp.netplay.client.local_gamehost.confirm_current_inputs(g);
        }
        for p in 0..g.players.len() {
            if let Some(front) = g.netplay.remote_input_buffer[p].pop_front() {
                g.netplay.netPlayerInput.outputControls[p] = front.1;
            }
        }
    }
}

pub fn network_send_local_input(g: &mut Game) {
        let mut gp = g.ptr();
    if !g.netplay.active {
        return;
    }

    gp.netplay.client.store_local_input(g);
    gp.netplay.client.send_local_input(g);
}

pub fn network_broadcast_game_state(g: &mut Game) {
        let mut gp = g.ptr();
    // The host sends the game state to clients
    if g.netplay.active && g.netplay.theHostIsMe {
        gp.netplay.client.local_gamehost.send_current_game_state_if_needed(g);
        gp.netplay.client.local_gamehost.update(g);
    }
}

pub fn start_gameplay(g: &mut Game) {
    let mut gp = g.ptr();
    clean_up(g);
    set_game_mode_settings_from_menu(g);
    g.game_values.appstate = AppState::Game;

    if g.game_values.music {
        gp.musiclist.set_random_music(g, music_category(g.g_map.musicCategoryID), "", "");
        let path = g.musiclist.current_music().clone();
        g.rm.backgroundmusic[0] = load_music(g, &path);
        gp.rm.backgroundmusic[0].play(g, g.game_values.playnextmusic, false);
    }
}

pub fn end_gameplay(g: &mut Game) {
        let mut gp = g.ptr();
    clean_up(g);
    g.game_values.appstate = AppState::Menu;
    g.game_values.screenfadespeed = -8;
    GameStateManager::instance(g).change_state_to(gp.get(), crate::smw::game_state::StateId::Menu);
}

impl GameplayState {
    fn update_countdown_timer(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        //Count down start timer before each game
        if self.iCountDownState > 0 {
            self.iCountDownTimer -= 1;
            if self.iCountDownTimer <= 0 {
                // Reaching 0 ends the countdown; index 28 would be out of bounds.
                self.iCountDownState -= 1;
                if self.iCountDownState != 0 {
                    self.iCountDownTimer = iCountDownTimes[(28 - self.iCountDownState) as usize];

                    let countDownAnnounce = iCountDownAnnounce[(28 - self.iCountDownState) as usize];
                    if countDownAnnounce >= 0 {
                        gp.rm.sfx_announcer[countDownAnnounce as usize].play(g);
                    }
                }
            }
        }
    }
}

pub fn update_playerswap(g: &mut Game) {
    if g.game_values.flags.swapplayers {
        for i in 0..g.players.len() {
            let mut player = g.players[i];
            g.player(player).updateswap(g);
        }
    }
}

impl GameplayState {
    fn update_world(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        shake_screen(g);
        self.spin_screen(g);
        update_bullet_bill_powerup(g);

        if g.game_values.matchtype == MatchType::World && g.mode().gameover && g.game_values.flags.forceexittimer <= 0 {
            g.game_values.flags.noexittimer -= 1;
            if g.game_values.flags.noexittimer <= 0 {
                g.game_values.flags.noexit = false;
            }
        }

        //------------- update objects -----------------------

        //Advance the cpu's turn (AI only calculates player's actions 1 out of 4 frames)
        g.game_values.cputurn += 1;
        if g.game_values.cputurn > 3 {
            g.game_values.cputurn = 0;
        }

        if !g.netplay.active || (g.netplay.active && g.netplay.theHostIsMe) {
            handle_p2p_collisions(g);
        }

        //Move platforms
        gp.g_map.update_platforms(g);

        g.game_values.flags.playskidsound = false;
        g.game_values.flags.playinvinciblesound = false;
        g.game_values.flags.playflyingsound = false;

        // Move every player before object collisions, so they test against post-map-collision positions.
        for i in 0..g.players.len() {
            let mut player = g.players[i];
            g.player(player).r#move(g);
        }

        play_sfx(g);

        gp.noncolcontainer.update(g);
        gp.objectcontainer[0].update(g);
        gp.objectcontainer[1].update(g);
        gp.objectcontainer[2].update(g);

        handle_p2obj_collisions(g);
        if g.game_values.flags.swapplayers {
            update_playerswap(g);
            return;
        }

        handle_obj2obj_collisions(g);

        //Commit all player actions at this point (after we have collided with any objects
        //that the player might have picked up)
        for i in 0..g.players.len() {
            let mut player = g.players[i];
            g.player(player).commit_action(g);
        }

        clean_dead_non_player_objects(g);
        self.clean_dead_players(g);

        gp.eyecandy[0].update(g);
        gp.eyecandy[1].update(g);
        gp.eyecandy[2].update(g);

        gp.mode().think(g);

        if g.game_values.flags.slowdownon != -1 {
            g.game_values.flags.slowdowncounter += 1;
            if g.game_values.flags.slowdowncounter > 580 {
                g.game_values.flags.slowdownon = -1;
                g.game_values.flags.slowdowncounter = 0;
            }
        }

        gp.g_map.update(g);

        update_screen_shake(g);

        update_scoreboard_animation(g);
    }
}

impl GameState for GameplayState {
    fn on_enter_state(&mut self, g: &mut Game) {
        crate::services::services(g).video.show_cursor(false);
        self.iCountDownState = 0;
        self.iCountDownTimer = 0;

        if g.game_values.startgamecountdown && g.game_values.singleplayermode == -1 {
            self.iCountDownState = 28;
            self.iCountDownTimer = iCountDownTimes[0];
        }

        self.previous_playerKeys = g.game_values.playerInput.outputControls[0];

        self.init_run_game(g);
    }

    fn update(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if crate::smw::net::waiting_to_start(g) {
            crate::smw::net::draw_match_preview(g);
            return;
        }
        net_random::gameplay_frame(g);
        self.read_network(g);

        if !g.netplay.active {
            check_wind_event(g, &mut self.iWindTimer, &mut self.dNextWind);
        }

        for iTeam in 0..g.score_cnt as usize {
            self.iScoreTextOffset[iTeam] = (34 * g.game_values.teamcounts[iTeam] as i32 + 1) as i16;
        }

        self.handle_input(g);
        network_send_local_input(g);

        if update_exit_pause_dialog(g, self.iCountDownState) {
            if g.netplay.active {
                gp.netplay.client.send_leave_game_message(g);
            }
            GameStateManager::instance(g).change_state_to(gp.get(), crate::smw::game_state::StateId::Menu);
            return;
        }

        if should_update(g) {
            if !g.game_values.flags.swapplayers && g.game_values.screenfade == 0 {
                self.update_countdown_timer(g);

                //Make updates to background stuff (animate map while countdown is counting)
                if self.iCountDownState > COUNTDOWN_START_INDEX {
                    animate_during_countdown(g);
                } else {
                    self.update_world(g);
                }
            }
            update_playerswap(g);
            crate::smw::net_outcomes::sample(g);
            network_broadcast_game_state(g);

            if g.game_values.screenfade == 255 {
                if g.game_values.appstate == AppState::StartGame {
                    start_gameplay(g);
                    return;
                } else if g.game_values.appstate == AppState::EndGame {
                    end_gameplay(g);
                    return;
                }
            }

            //--------------- draw everything ----------------------
            let (state, offsets) = (self.iCountDownState, self.iScoreTextOffset);
            self.draw_everything(g, state, offsets);
        }

        if g.game_values.flags.pausegame || g.game_values.flags.exitinggame {
            draw_exit_pause_dialog(g);
        }

        play_music(g);
    }
}

pub fn coldec_player2player(g: &Game, o1: PlayerId, o2: PlayerId) -> bool {
    let sw = App::screenWidth;
    let (o1ix, o1iy, o1r, o1b) = (g.player(o1).ix as i32, g.player(o1).iy as i32, g.player(o1).right_x() as i32, g.player(o1).bottom_y() as i32);
    let (o2ix, o2iy, o2r, o2b) = (g.player(o2).ix as i32, g.player(o2).iy as i32, g.player(o2).right_x() as i32, g.player(o2).bottom_y() as i32);

    //Special cases to deal with players overlapping the right and left sides of the screen
    if o1r < o2ix {
        o1ix + sw < o2r && o1r + sw >= o2ix && o1iy <= o2b && o1b >= o2iy
    } else if o2r < o1ix {
        o1ix < o2r + sw && o1r >= o2ix + sw && o1iy <= o2b && o1b >= o2iy
    } else {
        //Normal case where no overlap
        o1ix < o2r && o1r >= o2ix && o1iy <= o2b && o1b >= o2iy
    }
}

pub fn coldec_player2obj(g: &Game, o1: PlayerId, o2: Ptr<dyn CObjectTrait>) -> bool {
    let sw = App::screenWidth;
    let (o1ix, o1iy, o1r, o1b) = (g.player(o1).ix as i32, g.player(o1).iy as i32, g.player(o1).right_x() as i32, g.player(o1).bottom_y() as i32);
    let (o2x, o2y, o2w, o2h) = (o2.x(), o2.y(), o2.collision_rect_w() as i32, o2.collision_rect_h() as i32);

    //Special cases to deal with players overlapping the right and left sides of the screen
    if o1r < o2x {
        o1ix + sw < o2x + o2w && o1r + sw >= o2x && o1iy < o2y + o2h && o1b >= o2y
    } else if o2x + o2w < o1ix {
        o1ix < o2x + o2w + sw && o1r >= o2x + sw && o1iy < o2y + o2h && o1b >= o2y
    } else {
        //Normal case where no overlap
        o1ix < o2x + o2w && o1r >= o2x && o1iy < o2y + o2h && o2y <= o1b
    }
}

pub fn coldec_obj2obj(o1: Ptr<dyn CObjectTrait>, o2: Ptr<dyn CObjectTrait>) -> bool {
    let sw = App::screenWidth;
    //Special cases to deal with players overlapping the right and left sides of the screen
    let o1r: i16 = (o1.x() + o1.collision_rect_w() as i32) as i16;
    let o1b: i16 = (o1.y() + o1.collision_rect_h() as i32) as i16;
    let o2r: i16 = (o2.x() + o2.collision_rect_w() as i32) as i16;
    let o2b: i16 = (o2.y() + o2.collision_rect_h() as i32) as i16;
    let (o1r, o1b, o2r, o2b) = (o1r as i32, o1b as i32, o2r as i32, o2b as i32);

    if o1r < o2.x() {
        o1.x() + sw < o2r && o1r + sw >= o2.x() && o1.y() < o2b && o1b >= o2.y()
    } else if o2r < o1.x() {
        o1.x() < o2r + sw && o1r >= o2.x() + sw && o1.y() < o2b && o1b >= o2.y()
    } else {
        o1.x() < o2r && o1r >= o2.x() && o1.y() < o2b && o1b >= o2.y()
    }
}

pub fn swap_players(g: &mut Game, iUsingPlayerID: i16) -> bool {
    let ready = g.players.iter().filter(|p| g.player(**p).isready()).count();
    if ready > 1 && net_random::event(g, Ev::MysterySwap, &[iUsingPlayerID as i32]) {
        return true;
    }
    net_mystery_swap(g, iUsingPlayerID)
}

pub fn net_mystery_swap(g: &mut Game, iUsingPlayerID: i16) -> bool {
        let mut gp = g.ptr();
    //Count available players to switch with
    let mut iNumAvailablePlayers: i16 = 0;
    for i in 0..g.players.len() {
        let mut player = g.players[i];
        if g.player(player).isready() {
            iNumAvailablePlayers += 1;
            g.player(player).fOldSwapX = g.player(player).left_x() as f32;
            g.player(player).fOldSwapY = g.player(player).top_y() as f32;

            let teamcount = g.game_values.teamcounts[g.player(player).teamID as usize];
            g.player(player).iNewPowerupX = (g.player(player).score(g).x as i32 + scorepowerupoffsets[(teamcount - 1) as usize][g.player(player).subTeamID as usize] as i32) as i16;
            g.player(player).iNewPowerupY = (g.player(player).score(g).y as i32 + 25) as i16;
        }
    }

    if iNumAvailablePlayers <= 1 {
        return false;
    }

    if g.game_values.swapstyle != 2 {
        g.game_values.flags.swapplayers = true;
        g.game_values.flags.swapplayersposition = 0.0f32;

        if g.game_values.swapstyle == 1 {
            g.game_values.flags.swapplayersblink = false;
            g.game_values.flags.swapplayersblinkcount = 0;
        } else {
            g.game_values.screenfade = App::menuTransparency as i16;
        }
    }

    let iIncrement = RANDOM_INT(gp.get(), iNumAvailablePlayers as i32 - 1) as i16;

    let mut spots: [MysteryMushroomTempPlayer; 4] =
        [MysteryMushroomTempPlayer::new(), MysteryMushroomTempPlayer::new(), MysteryMushroomTempPlayer::new(), MysteryMushroomTempPlayer::new()];
    for iPlayer in 0..g.players.len() {
        let mut player = g.players[iPlayer];

        if !g.player(player).isready() {
            continue;
        }

        let mut iNewSpot: i16 = (iPlayer as i32 + iIncrement as i32) as i16;

        loop {
            iNewSpot += 1;
            if iNewSpot as usize >= g.players.len() {
                iNewSpot = 0;
            }
            if !(spots[iNewSpot as usize].fUsed || !g.player(g.players[iNewSpot as usize]).isready()) {
                break;
            }
        }

        spots[iNewSpot as usize].fUsed = true;
        let newPlayer = g.players[iNewSpot as usize];
        spots[iPlayer].set_player(g, newPlayer, g.game_values.gamepowerups[g.player(newPlayer).get_global_id() as usize]);

        //Give credit for deaths to the player that used the mystery mushroom
        if iUsingPlayerID == iNewSpot {
            g.player(player).iSuicideCreditPlayerID = g.player(newPlayer).globalID;
            g.player(player).iSuicideCreditTimer = 62;
        }
    }

    for iPlayer in 0..g.players.len() {
        let mut player = g.players[iPlayer];

        if !g.player(player).isready() {
            continue;
        }

        let gid = g.player(player).get_global_id() as usize;
        spots[iPlayer].get_player(g, player, &mut gp.game_values.gamepowerups[gid]);

        if g.game_values.swapstyle != 1 {
            gp.get().eyecandy[2].emplace(EC_SingleAnimation::new(&gp.rm, 
                gp.get().rm.spr_fireballexplosion,
                ((g.player(player).fNewSwapX as i16) as i32 + HALFPW - 16) as i16,
                ((g.player(player).fNewSwapY as i16) as i32 + HALFPH - 16) as i16,
                3,
                8,
            ));
        }

        if g.game_values.swapstyle == 2 {
            let (nx, ny) = (g.player(player).fNewSwapX, g.player(player).fNewSwapY);
            g.player(player).set_xf(nx);
            g.player(player).set_yf(ny);

            if !g.carried_item(player).is_null() {
                g.carried_item(player).move_to_owner(g);
            }
        }
    }

    true
}
