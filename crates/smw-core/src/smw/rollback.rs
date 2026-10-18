//! Rollback for netplay v2 (docs/design/NETPLAY_V2.md): a snapshot of everything a match tick reads and writes, the
//! tick itself with given keys, and a state checksum. GGRS decides when to save, load and re-run.

use crate::common::eyecandy::{CEyecandyContainer, SpotlightManager};
use crate::common::game_mode::CGameModeTrait;
use crate::common::game_values::CGameValues;
use crate::common::input::COutputControl;
use crate::common::map::CMap;
use crate::common::random_number_generator::RngState;
use crate::common::score::CScore;
use crate::common::sfx::SfxState;
use crate::game::Game;
use crate::smw::gamemodes::{bonus_house::CGM_Bonus, mini_boss::CGM_Boss_MiniGame, mini_boxes::CGM_Boxes_MiniGame, mini_pipe::CGM_Pipe_MiniGame};
use crate::smw::gs_gameplay::GameplayState;
use crate::smw::harness;
use crate::smw::network::protocol_game_packages::RawInput;
use crate::smw::object_container::CObjectContainer;
use crate::smw::players::Players;
use serde::{Deserialize, Serialize};

/// One player's keys for one tick: `down` at bit 2k and `pressed` at bit 2k+1 for key k, as in `RawInput`.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Serialize, Deserialize)]
pub struct Keys(pub u16);

impl Keys {
    pub fn of(control: &COutputControl) -> Keys {
        let mut raw = RawInput::default();
        for (k, key) in control.keys.iter().enumerate() {
            raw.set_player_key(k as u8, key.fDown, key.fPressed);
        }
        Keys(raw.flags)
    }

    pub fn apply(self, control: &mut COutputControl) {
        let raw = RawInput { flags: self.0 };
        for (k, key) in control.keys.iter_mut().enumerate() {
            raw.get_player_key(k as u8, &mut key.fDown, &mut key.fPressed);
        }
    }
}

/// Where a player's keys go: a local input slot, or a remote player's slot (net games' `PlayerKeys`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeySlot {
    Local(usize),
    Net(usize),
}

/// Everything a match tick changes. Loading writes into the live allocations, which other code points into.
#[derive(Clone)]
pub struct MatchState {
    game_values: CGameValues,
    g_map: CMap,
    noncolcontainer: CObjectContainer,
    objectcontainer: [CObjectContainer; 3],
    eyecandy: [CEyecandyContainer; 3],
    players: Players,
    score: [CScore; 4],
    score_cnt: i16,
    gamemodes: Vec<Option<Box<dyn CGameModeTrait>>>,
    bonushousemode: Option<CGM_Bonus>,
    pipegamemode: Option<CGM_Pipe_MiniGame>,
    bossgamemode: Option<CGM_Boss_MiniGame>,
    boxesgamemode: Option<CGM_Boxes_MiniGame>,
    currentgamemode: i16,
    rng: RngState,
    sfx: SfxState,
    x_shake: i16,
    y_shake: i16,
    shakeleft: bool,
    swirl_spawns: [[[i16; 25]; 2]; 4],
    spotlights: SpotlightManager,
    winning_player: i16,
    draw_index: i16,
    network_id_context: u32,
    network_id_count: u32,
    gps: Option<GameplayState>,
    frame: u32,
}

impl Clone for Box<dyn CGameModeTrait> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub fn save(g: &mut Game) -> MatchState {
    MatchState {
        game_values: (*g.game_values).clone(),
        g_map: (*g.g_map).clone(),
        noncolcontainer: g.noncolcontainer.clone(),
        objectcontainer: g.objectcontainer.clone(),
        eyecandy: g.eyecandy.clone(),
        players: g.players.clone(),
        score: std::array::from_fn(|i| (*g.score[i]).clone()),
        score_cnt: g.score_cnt,
        gamemodes: g.gamemodes.iter().map(|m| m.is_set().then(|| m.clone_box())).collect(),
        bonushousemode: g.bonushousemode.is_set().then(|| (*g.bonushousemode).clone()),
        pipegamemode: g.pipegamemode.is_set().then(|| (*g.pipegamemode).clone()),
        bossgamemode: g.bossgamemode.is_set().then(|| (*g.bossgamemode).clone()),
        boxesgamemode: g.boxesgamemode.is_set().then(|| (*g.boxesgamemode).clone()),
        currentgamemode: g.currentgamemode,
        rng: g.rng.clone(),
        sfx: g.sfx.clone(),
        x_shake: g.x_shake,
        y_shake: g.y_shake,
        shakeleft: g.shakeleft,
        swirl_spawns: g.g_iSwirlSpawnLocations,
        spotlights: (*g.spotlightManager).clone(),
        winning_player: g.g_iWinningPlayer,
        draw_index: g.g_iCurrentDrawIndex,
        network_id_context: g.g_networkIDContext,
        network_id_count: g.g_networkIDCount,
        gps: g.gps.clone(),
        frame: harness::frame(g),
    }
}

/// Puts `s` back. Call it between ticks, never from inside the gameplay state's own update.
pub fn load(g: &mut Game, s: &MatchState) {
    *g.game_values = s.game_values.clone();
    *g.g_map = s.g_map.clone();
    g.noncolcontainer = s.noncolcontainer.clone();
    g.objectcontainer = s.objectcontainer.clone();
    g.eyecandy = s.eyecandy.clone();
    g.players = s.players.clone();
    for (live, saved) in g.score.iter_mut().zip(&s.score) {
        **live = saved.clone();
    }
    g.score_cnt = s.score_cnt;
    for (live, saved) in g.gamemodes.iter_mut().zip(&s.gamemodes) {
        if let Some(mode) = saved {
            live.set(mode.clone_box());
        }
    }
    if let Some(mode) = &s.bonushousemode {
        *g.bonushousemode = mode.clone();
    }
    if let Some(mode) = &s.pipegamemode {
        *g.pipegamemode = mode.clone();
    }
    if let Some(mode) = &s.bossgamemode {
        *g.bossgamemode = mode.clone();
    }
    if let Some(mode) = &s.boxesgamemode {
        *g.boxesgamemode = mode.clone();
    }
    g.currentgamemode = s.currentgamemode;
    g.rng = s.rng.clone();
    let audible = g.sfx.sfx_audible;
    g.sfx = s.sfx.clone();
    g.sfx.sfx_audible = audible;
    g.x_shake = s.x_shake;
    g.y_shake = s.y_shake;
    g.shakeleft = s.shakeleft;
    g.g_iSwirlSpawnLocations = s.swirl_spawns;
    *g.spotlightManager = s.spotlights.clone();
    g.g_iWinningPlayer = s.winning_player;
    g.g_iCurrentDrawIndex = s.draw_index;
    g.g_networkIDContext = s.network_id_context;
    g.g_networkIDCount = s.network_id_count;
    g.gps = s.gps.clone();
    harness::set_frame(g, s.frame);
}

/// One match tick with each player's keys in place; slots not listed keep theirs (a CPU player's).
/// `silent` mutes the tick, for re-runs after a rollback. True when the match ended or restarted.
pub fn advance(g: &mut Game, keys: &[(KeySlot, Keys)], silent: bool) -> bool {
    let mut gp = g.ptr();
    crate::common::sfx::sfx_virtual_advance(g);
    for &(slot, k) in keys {
        match slot {
            KeySlot::Local(i) => k.apply(&mut g.game_values.playerInput.outputControls[i]),
            KeySlot::Net(i) => k.apply(&mut g.netplay.netPlayerInput.outputControls[i]),
        }
    }
    let audible = g.sfx.sfx_audible;
    if silent {
        g.sfx.sfx_audible = false;
    }
    let gameplay = gp.gps.as_mut().expect("rollback::advance outside a match");
    gameplay.wind(g);
    let changed = gameplay.simulate(g);
    g.sfx.sfx_audible = audible;
    harness::end_rollback_tick(g);
    changed
}

/// FNV-1a over the match's harness dump (game mode, players, objects): what a desync check compares.
pub fn checksum(g: &mut Game) -> u128 {
    let mut gp = g.ptr();
    let mut dump = Vec::new();
    if let Some(gameplay) = gp.gps.as_mut() {
        let _ = gameplay.harness_dump(g, &mut dump);
    }
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in dump.iter().chain(&harness::frame(g).to_le_bytes()) {
        hash = (hash ^ *byte as u64).wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash as u128
}

/// The GGRS session parameters of a match: a player's keys per tick, addressed by peer.
pub struct GgrsConfig<A>(std::marker::PhantomData<A>);

impl<A: Clone + PartialEq + Eq + std::hash::Hash + std::fmt::Debug + 'static> ggrs::Config for GgrsConfig<A> {
    type Input = Keys;
    type InputPredictor = ggrs::PredictRepeatLast;
    type State = MatchState;
    type Address = A;
}

/// Carries out a session's requests. `slots[p]` is where GGRS player `p`'s keys go.
pub fn handle_requests<A>(g: &mut Game, requests: Vec<ggrs::GgrsRequest<GgrsConfig<A>>>, slots: &[KeySlot], silent_reruns: bool) -> bool
where
    A: Clone + PartialEq + Eq + std::hash::Hash + std::fmt::Debug + 'static,
{
    let mut rerun = false;
    let mut changed = false;
    for request in requests {
        match request {
            ggrs::GgrsRequest::SaveGameState { cell, frame } => {
                let sum = checksum(g);
                if let Ok(path) = std::env::var("SMW_V2_STATE_LOG") {
                    let mut gp = g.ptr();
                    let mut dump = Vec::new();
                    if let Some(gameplay) = gp.gps.as_mut() {
                        let _ = gameplay.harness_dump(g, &mut dump);
                    }
                    use std::io::Write;
                    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
                        let _ = writeln!(f, "#F {} hframe={} sum={:x}", frame, harness::frame(g), sum);
                        let _ = f.write_all(&dump);
                    }
                }
                cell.save(frame, Some(save(g)), Some(sum));
            }
            ggrs::GgrsRequest::LoadGameState { cell, .. } => {
                load(g, &cell.load().expect("GGRS loads only saved frames"));
                rerun = true;
            }
            ggrs::GgrsRequest::AdvanceFrame { inputs } => {
                let keys: Vec<(KeySlot, Keys)> = inputs.iter().enumerate().map(|(player, (k, _status))| (slots[player], *k)).collect();
                changed |= advance(g, &keys, rerun && silent_reruns);
            }
        }
    }
    changed
}
