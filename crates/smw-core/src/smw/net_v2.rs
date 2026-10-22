//! Netplay v2 matches: every peer restores the host's match start, then the match runs under a GGRS P2P session over
//! the room's WebRTC connections. Recordings keep each frame as a `#@ v2` line, which a replay plays without the network.
#![deny(clippy::all)]

use crate::common::global_constants::MAX_PLAYERS;
use crate::common::input::{COutputControl, DEVICE_KEYBOARD};
use crate::game::Game;
use crate::smw::harness;
use crate::smw::player::PlayerKeys;
use crate::smw::network::net_record;
use crate::smw::rollback::{self, GgrsConfig, KeySlot, Keys, MatchState, Req};
use ggrs::{GgrsError, GgrsEvent, P2PSession, PlayerType, SessionBuilder, SessionState};
use smw_peer::{PeerId, PeerSocket, PeerState, RELIABLE};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

/// `[MSG_SLOT, HANDSHAKE_VERSION, (slot, pad)...]`: the room slot of each of the sender's local players, in local order,
/// and whether that player plays on a gamepad (1) or the keyboard (0). Reliable-channel messages start with one of these
/// kinds.
const MSG_SLOT: u8 = 1;
/// The handshake's format; a peer announcing another one cannot play this build.
const HANDSHAKE_VERSION: u8 = 2;
/// `[MSG_START_PART, index u16 LE, count u16 LE, bytes]`: one part of the host's match start.
const MSG_START_PART: u8 = 2;
/// A joiner to the host: the match start is restored.
const MSG_READY: u8 = 3;
/// The host to the joiners: every joiner is ready, start the GGRS session.
const MSG_GO: u8 = 4;
/// The sender left the match; the others end it too.
const MSG_LEAVE: u8 = 5;
/// Match start bytes per `MSG_START_PART`.
const PART_BYTES: usize = 16 * 1024;
/// The largest match start a peer sends or takes; a real one is about 20 KiB.
const MAX_START_BYTES: usize = 1024 * 1024;
/// How long the start handshake may take before the match is abandoned.
const START_DEADLINE: Duration = Duration::from_secs(20);
/// Frames between repeats of the handshake messages.
const RESEND_TICKS: u64 = 30;
/// Ticks of local input delay.
const INPUT_DELAY: usize = 2;
/// How many ticks GGRS runs ahead of the confirmed inputs before it waits.
const MAX_PREDICTION: usize = 8;
/// GGRS frames between checksum comparisons.
const DESYNC_INTERVAL: u32 = 30;
/// A peer is dropped for good after this much silence; GGRS's 2 s default is shorter than a Wi-Fi mesh's node hop.
const DISCONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Silence after which GGRS reports a peer as interrupted.
const INTERRUPTED_AFTER: Duration = Duration::from_secs(1);
/// How long a notice stays over the match.
const NOTICE_FRAMES: u32 = 4 * crate::common::global_constants::TICKS_PER_SECOND;

/// Where a [`V2Match`] is: the start handshake, or the running GGRS session.
enum Phase {
    Starting,
    Running(Box<P2PSession<GgrsConfig<PeerId>>>),
}

/// This client's live v2 match, held in `NetClient::v2` from `match_starting` until it ends.
pub struct V2Match {
    socket: PeerSocket,
    host: bool,
    /// The room slots of this client's players, by local index (`localSlots`).
    my_slots: Vec<u8>,
    /// Each slot's player is on a gamepad, as its client announced; `playerDevice` changes how a player drops through
    /// platforms, so every peer sets it from these, not from its own controls.
    pads: [Option<bool>; 4],
    /// How many other peers the match waits for.
    others: usize,
    /// Each other peer's room slots, from its `MSG_SLOT`.
    slots: BTreeMap<PeerId, Vec<u8>>,
    /// The host's room slot (`hostPlayerNumber`).
    host_slot: u8,
    /// The peer whose `MSG_SLOT` claimed the host's slot; a joiner takes the match start, GO and LEAVE only from it.
    host_peer: Option<PeerId>,
    /// Peers whose slot claim was refused, so each is logged once.
    refused: Vec<PeerId>,
    /// SMW_V2_ROGUE, for tests: a joiner that also claims the host's slot and sends its own start parts and GO.
    rogue: bool,
    /// The host's match start (`checkpoint::capture_shared`); empty on a joiner.
    start: Vec<u8>,
    /// The match start parts a joiner has received so far.
    parts: StartParts,
    /// The joiners that sent `MSG_READY`; host only.
    ready: Vec<PeerId>,
    /// The match start is in place: from the outset on the host, after restoring it on a joiner.
    restored: bool,
    /// A joiner received the host's `MSG_GO`.
    go: bool,
    /// A joiner's own options.bin and controls bytes: the host's match start replaces them for the match.
    own_config: Option<(Vec<u8>, Vec<u8>)>,
    phase: Phase,
    /// Where each GGRS player handle's keys go.
    targets: Vec<KeySlot>,
    /// The local players' device keys by local index, carried from frame to frame because rollback ticks overwrite
    /// their input slots.
    local_devices: Vec<COutputControl>,
    opened: Instant,
    /// Handshake frames while starting, then frames of the running session.
    ticks: u64,
    /// Ticks re-run after rollbacks.
    rollback_ticks: u64,
    desyncs: u32,
    /// Remote players GGRS dropped for good.
    gone: usize,
    /// The room's player names by slot when the match started; the lobby drops a player who disconnects.
    names: [String; 4],
    /// Peers already named in a desync notice.
    desynced: Vec<PeerId>,
    notice: Notice,
    perf: V2Perf,
    /// This frame's `#@ v2` tokens, while recording.
    rec: Option<Vec<String>>,
}

/// What a 600-tick window of a match cost, logged with `[net] v2:`.
#[derive(Default)]
struct V2Perf {
    frames: u32,
    waits: u32,
    total_us: u64,
    worst_us: u64,
}

/// Called where a match starts (`MenuState::enter_gameplay`, after the checkpoint point). On a v2 client in a room the
/// lobby handed to WebRTC, it takes the room's socket for the match; the host also captures the match start to send.
/// True when the caller must not start the match: a joiner starts it from the host's match start instead.
pub fn match_starting(g: &mut Game) -> bool {
    let mut gp = g.ptr();
    if !g.netplay.active {
        return false;
    }
    g.netplay.match_notice = None;
    if net_record::replaying(g) {
        if !net_record::has_v2(g) {
            return false;
        }
        let host = g.netplay.theHostIsMe;
        g.netplay.v2_match = true;
        start_replay(g);
        return !host;
    }
    let Some((mut socket, others)) = gp.netplay.client.take_peer_socket() else {
        println!("[net] v2: the lobby sent no WebRTC room");
        crate::smw::game_state::GameStateManager::instance(g).change_state_to(gp.get(), crate::smw::game_state::StateId::Menu);
        return true;
    };
    let host = g.netplay.theHostIsMe;
    let my_slots = g.netplay.localSlots.clone();
    let mut pads = [None; 4];
    for (local, &slot) in my_slots.iter().enumerate() {
        pads[slot as usize] = Some(g.input_control(g.game_values.playerInput.inputControls[local]).iDevice != DEVICE_KEYBOARD);
    }
    let start = if host {
        let start = crate::smw::checkpoint::capture_shared(g);
        let too_large = start.as_ref().is_ok_and(|s| s.len() > MAX_START_BYTES);
        match start {
            Ok(start) if !too_large => start,
            failed => {
                match failed {
                    Ok(start) => println!("[net] v2: cannot start the match: its start is {} bytes", start.len()),
                    Err(e) => println!("[net] v2: cannot start the match: {e}"),
                }
                let notice = if too_large { "This match is too large to send" } else { "Cannot read the map for this match" };
                g.netplay.match_notice = Some(notice.to_string());
                for peer in socket.connected_peers().to_vec() {
                    socket.send(RELIABLE, peer, &slot_message(&my_slots, &pads));
                    socket.send(RELIABLE, peer, &[MSG_LEAVE]);
                }
                gp.netplay.client.linger(socket);
                to_menu(g, false);
                return true;
            }
        }
    } else {
        Vec::new()
    };
    g.netplay.v2_match = true;
    let mut m = V2Match {
        socket,
        host,
        local_devices: devices_of(g, my_slots.len()),
        my_slots,
        pads,
        others,
        slots: BTreeMap::new(),
        host_slot: g.netplay.hostPlayerNumber,
        host_peer: None,
        refused: Vec::new(),
        rogue: std::env::var_os("SMW_V2_ROGUE").is_some(),
        start,
        parts: StartParts::default(),
        ready: Vec::new(),
        restored: host,
        go: false,
        own_config: None,
        phase: Phase::Starting,
        targets: Vec::new(),
        opened: Instant::now(),
        ticks: 0,
        rollback_ticks: 0,
        desyncs: 0,
        gone: 0,
        names: g.netplay.currentRoom.playerNames.clone(),
        desynced: Vec::new(),
        notice: Notice::default(),
        perf: V2Perf::default(),
        rec: None,
    };
    m.announce();
    println!("[net] v2: starting the match with {} peers ({})", others, if host { "host" } else { "joiner" });
    gp.netplay.client.v2 = Some(Box::new(m));
    !host
}

/// True while a v2 match or its replay runs the frame instead of the current game state.
pub fn owns_tick(g: &mut Game) -> bool {
    g.netplay.client.v2.is_some() || g.netplay.client.v2_replay.is_some()
}

/// One frame of a v2 match: the start handshake, then input, GGRS and the ticks it asks for.
pub fn tick(g: &mut Game) {
    let (mut gp, mut mp) = (g.ptr(), g.ptr());
    let frame = harness::frame(g);
    if g.netplay.client.v2_replay.is_some() {
        return replay_tick(g, frame);
    }
    gp.netplay.client.update(g);
    let Some(m) = mp.netplay.client.v2.as_mut() else { return };
    m.rec = harness::recording(g).then(Vec::new);
    let keep = match m.phase {
        Phase::Starting => m.start_step(g) || m.abandon(g),
        Phase::Running(_) => m.run_step(g),
    };
    if let Some(mut tokens) = m.rec.take() {
        if !keep && !tokens.iter().any(|t| t.starts_with('e')) {
            tokens.push("eF".to_string());
        }
        let tokens = if tokens.is_empty() { "w".to_string() } else { tokens.join(" ") };
        harness::record_line(g, &format!("#@ v2 frame={frame} {tokens}"));
    }
    if !keep {
        finish(g);
    }
}

/// Restores the host's match start on a joiner and returns the joiner's own settings, which the match replaces.
fn restore_start(g: &mut Game, my_slots: &[u8], bytes: Vec<u8>) -> Result<(Vec<u8>, Vec<u8>), String> {
    let mut gp = g.ptr();
    let own_config = gp.game_values.config_bytes(g);
    let own_skins: Vec<(usize, i16)> = my_slots.iter().map(|&slot| (slot as usize, g.game_values.skinids[slot as usize])).collect();
    let gv = &g.game_values;
    let own_presentation = (gv.framelimiter, gv.sound, gv.music, gv.musicvolume, gv.soundvolume, gv.fullscreen);
    crate::smw::checkpoint::restore_shared(g, bytes)?;
    // The host's start carries its settings for the match rules; frame rate, sound and fullscreen stay this player's.
    let gv = &mut g.game_values;
    (gv.framelimiter, gv.sound, gv.music, gv.musicvolume, gv.soundvolume, gv.fullscreen) = own_presentation;
    crate::common::sfx::sfx_setmusicvolume(g, own_presentation.3 as i32);
    crate::common::sfx::sfx_setsoundvolume(g, own_presentation.4 as i32);
    // The host's start holds the host's skin choices; this client's players' own skins are their choice here.
    for (slot, own_skin) in own_skins {
        g.game_values.skinids[slot] = own_skin;
        let strip = gp.rm.load_full_skin(g, own_skin, g.game_values.colorids[slot]);
        gp.rm.set_strip(gp.rm.spr_player[slot], strip);
    }
    // The host's start also holds its key bindings and devices; this player keeps reading its own.
    gp.game_values.read_controls_bytes(g, own_config.1.clone());
    // The host captured mid-tick; this finishes that tick as its frame_end did there.
    harness::end_rollback_tick(g);
    println!("[net] v2: restored the host's match start");
    Ok(own_config)
}

/// The device keys of the first `count` input slots, which a match's local players read.
fn devices_of(g: &Game, count: usize) -> Vec<COutputControl> {
    g.game_values.playerInput.outputControls[..count].to_vec()
}

/// The local players' input for a running match's frame, and the exit and pause dialog. True when the player left.
fn local_input(g: &mut Game, local_devices: &mut [COutputControl]) -> bool {
    let mut gp = g.ptr();
    harness::frame_start_input(g);
    g.game_values.playerInput.outputControls[..local_devices.len()].copy_from_slice(local_devices);
    let gameplay = gp.gps.as_mut().expect("a v2 match outside gameplay");
    gameplay.gather_input(g);
    local_devices.copy_from_slice(&g.game_values.playerInput.outputControls[..local_devices.len()]);
    crate::smw::gs_gameplay::update_exit_pause_dialog(g, gameplay.countdown_state())
}

/// A line shown over the match for a few seconds: a desync, or a player who dropped out or left.
#[derive(Default)]
struct Notice {
    text: String,
    frames: u32,
}

/// Logs `text`, shows it over the match for `NOTICE_FRAMES` and records it as an `n` token.
fn notify(rec: &mut Option<Vec<String>>, notice: &mut Notice, text: String) {
    println!("[net] v2: {text}");
    show_notice(rec, notice, text, NOTICE_FRAMES);
}

/// Shows `text` for `frames`, or hides the notice when `text` is empty, and records it as `n<frames>.<text>`.
fn show_notice(rec: &mut Option<Vec<String>>, notice: &mut Notice, text: String, frames: u32) {
    note(rec, format!("n{frames}.{}", harness::base64_encode(text.as_bytes())));
    *notice = Notice { text, frames };
}

/// A recorded [`show_notice`] token's frames and text.
fn parse_notice(data: &str) -> Option<(u32, String)> {
    let (frames, text) = data.split_once('.')?;
    Some((frames.parse().ok()?, String::from_utf8(harness::base64_decode(text)?).ok()?))
}

/// The room's name for the player in `slot`.
fn player_name(names: &[String], slot: u8) -> String {
    match names.get(slot as usize) {
        Some(name) if !name.is_empty() && name != "(empty)" => name.clone(),
        _ => format!("Player {}", slot as u32 + 1),
    }
}

/// The room's name for the players on `peer`, joined with "and" when a couch plays there.
fn peer_name(names: &[String], slots: &BTreeMap<PeerId, Vec<u8>>, peer: PeerId) -> String {
    match slots.get(&peer) {
        Some(held) if !held.is_empty() => held.iter().map(|&slot| player_name(names, slot)).collect::<Vec<_>>().join(" and "),
        _ => "A player".to_string(),
    }
}

/// The room's occupied slots, other than `my_slots`, that are not in `heard`.
fn missing_slots(names: &[String], my_slots: &[u8], heard: &[u8]) -> Vec<u8> {
    (0..names.len() as u8).filter(|slot| !my_slots.contains(slot) && names[*slot as usize] != "(empty)" && !heard.contains(slot)).collect()
}

/// The notice while `peer` is silent, which `NetworkResumed` takes down again.
fn lag_text(names: &[String], slots: &BTreeMap<PeerId, Vec<u8>>, peer: PeerId) -> String {
    format!("{} is lagging", peer_name(names, slots, peer))
}

/// A match that ended early leaves its last notice for the room menu.
fn keep_notice(g: &mut Game, notice: &Notice) {
    if !notice.text.is_empty() {
        g.netplay.match_notice = Some(notice.text.clone());
    }
}

/// The end of a running match's frame: the pause or exit dialog, the notice, and the music.
fn draw_dialogs(g: &mut Game, notice: &mut Notice) {
    if g.game_values.flags.pausegame || g.game_values.flags.exitinggame {
        crate::smw::gs_gameplay::draw_exit_pause_dialog(g);
    }
    if notice.frames > 0 && !notice.text.is_empty() {
        notice.frames -= 1;
        crate::smw::gs_menu::draw_message_bar(g, &notice.text);
    }
    if g.main_loop.stats_page {
        crate::smw::stats::draw_page(g);
    }
    crate::smw::gs_gameplay::play_music(g);
}

/// Back to the menu, telling the lobby; a match that did not start (`abandon`) also cleans up its gameplay.
fn to_menu(g: &mut Game, abandoned: bool) {
    let mut gp = g.ptr();
    if abandoned {
        crate::smw::gs_gameplay::clean_up(g);
        g.game_values.appstate = crate::common::game_values::AppState::Menu;
    }
    gp.netplay.client.send_leave_game_message(g);
    crate::smw::game_state::GameStateManager::instance(g).change_state_to(gp.get(), crate::smw::game_state::StateId::Menu);
}

/// A v2 match in a replay: each frame does what the recording's `#@ v2` line says the live frame did.
pub struct V2Replay {
    own_config: Option<(Vec<u8>, Vec<u8>)>,
    targets: Vec<KeySlot>,
    local_devices: Vec<COutputControl>,
    saves: BTreeMap<i32, MatchState>,
    notice: Notice,
}

fn start_replay(g: &mut Game) {
    let mut gp = g.ptr();
    g.netplay.v2_match = true;
    gp.netplay.client.v2_replay = Some(Box::new(V2Replay {
        own_config: None,
        targets: Vec::new(),
        local_devices: devices_of(g, g.netplay.localSlots.len()),
        saves: BTreeMap::new(),
        notice: Notice::default(),
    }));
}

/// A segment replay of a net match (`--segment`, the web page's Clip) restored the match's checkpoint; the
/// recording's `#@ v2` lines play the rest, a joiner's starting with the host's match start.
pub fn segment_started(g: &mut Game) {
    if g.game_values.matchtype == crate::common::match_types::MatchType::NetGame && net_record::has_v2(g) {
        start_replay(g);
    }
}

fn replay_tick(g: &mut Game, frame: u32) {
    let (mut gp, mut mp) = (g.ptr(), g.ptr());
    gp.netplay.client.update(g);
    let Some(r) = mp.netplay.client.v2_replay.as_mut() else { return };
    let Some((at, tokens)) = net_record::next_v2(g) else { harness::fail(format!("the replay has no v2 match frame {frame}")) };
    if at != frame {
        harness::fail(format!("the replay's next v2 match frame is {at}, not {frame}"));
    }
    let mut requests = Vec::new();
    let mut ended = false;
    for token in tokens.split(' ') {
        if let Some(request) = Req::parse(token) {
            requests.push(request);
            continue;
        }
        if !requests.is_empty() {
            rollback::replay_requests(g, &std::mem::take(&mut requests), &r.targets, &mut r.saves, DESYNC_INTERVAL as i32);
        }
        match token.split_at(1) {
            ("w", _) => {}
            ("r", data) => {
                let start = data.split_once('.').and_then(|(slots, data)| {
                    let bytes = harness::base64_decode(data).and_then(|z| crate::smw::network::file_compressor::inflate(&z))?;
                    Some((parse_slots(slots)?, bytes))
                });
                let Some((slots, bytes)) = start else {
                    harness::fail(format!("the replay's v2 match start at frame {frame} is damaged"));
                };
                if !g.netplay.active {
                    // A segment replay set up the joiner's own match, which the live joiner skipped for the host's.
                    crate::smw::gs_gameplay::clean_up(g);
                }
                match restore_start(g, &slots, bytes) {
                    Ok(own) => r.own_config = Some(own),
                    Err(e) => harness::fail(format!("the replay's v2 match start: {e}")),
                }
                r.local_devices = devices_of(g, slots.len());
            }
            ("t", data) => {
                let Some((my_slots, slots)) = data.split_once(':').and_then(|(m, s)| Some((parse_slots(m)?, s))) else {
                    harness::fail(format!("the replay's v2 session at frame {frame} is damaged"));
                };
                // Recordings from before the devices were announced have no pads part.
                let (slots, pads) = match slots.split_once(':') {
                    Some((slots, pads)) => match parse_pads(pads) {
                        Some(pads) => (slots, Some(pads)),
                        None => harness::fail(format!("the replay's v2 session at frame {frame} is damaged")),
                    },
                    None => (slots, None),
                };
                r.targets = slots.split(',').filter_map(parse_slot).collect();
                if r.local_devices.len() != my_slots.len() {
                    r.local_devices = devices_of(g, my_slots.len());
                }
                if !g.netplay.active {
                    // A segment replay made its players as a local match's; they read their keys as the net game's did.
                    for i in 0..g.players.len() {
                        let player = g.player(g.players[i]);
                        player.playerKeys = match my_slots.iter().position(|&slot| slot as i16 == player.globalID) {
                            Some(local) => PlayerKeys::Local(local as u8),
                            None => PlayerKeys::Net(player.globalID as u8),
                        };
                    }
                }
                if let Some(pads) = pads {
                    apply_pads(g, &pads);
                }
            }
            ("i", _) => {
                local_input(g, &mut r.local_devices);
            }
            ("d", _) => draw_dialogs(g, &mut r.notice),
            ("n", data) => {
                let Some((frames, text)) = parse_notice(data) else {
                    harness::fail(format!("the replay's v2 notice at frame {frame} is damaged"));
                };
                r.notice = Notice { text, frames };
            }
            ("e", kind) => {
                match kind {
                    "A" => {
                        keep_notice(g, &r.notice);
                        to_menu(g, true)
                    }
                    "L" => to_menu(g, false),
                    _ => {}
                }
                ended = true;
            }
            _ => harness::fail(format!("the replay's v2 match frame {frame} has an unknown token {token}")),
        }
    }
    if !requests.is_empty() {
        rollback::replay_requests(g, &requests, &r.targets, &mut r.saves, DESYNC_INTERVAL as i32);
    }
    if ended {
        g.netplay.v2_match = false;
        if let Some(r) = g.netplay.client.v2_replay.take() {
            if let Some((options, controls)) = r.own_config {
                gp.game_values.read_config_bytes(g, options, controls);
            }
        }
        println!("[net] v2: replayed match over");
    }
}

/// Adds a token to this frame's `#@ v2` line, while recording.
fn note(rec: &mut Option<Vec<String>>, token: String) {
    if let Some(tokens) = rec {
        tokens.push(token);
    }
}

/// `L<input slot>` or `N<net slot>`, as `#@ v2` lines write the session's players.
fn slot_token(slot: &KeySlot) -> String {
    match slot {
        KeySlot::Local(i) => format!("L{i}"),
        KeySlot::Net(i) => format!("N{i}"),
    }
}

/// `0+2`: room slots by local index, as `#@ v2` lines write this client's players.
fn slots_token(slots: &[u8]) -> String {
    slots.iter().map(u8::to_string).collect::<Vec<_>>().join("+")
}

/// Reads a [`slots_token`].
fn parse_slots(token: &str) -> Option<Vec<u8>> {
    token.split('+').map(|slot| slot.parse().ok()).collect()
}

/// Reads a [`slot_token`].
fn parse_slot(token: &str) -> Option<KeySlot> {
    let (kind, n) = token.split_at_checked(1)?;
    let n = n.parse().ok()?;
    match kind {
        "L" => Some(KeySlot::Local(n)),
        "N" => Some(KeySlot::Net(n)),
        _ => None,
    }
}

/// The parts of the host's match start a joiner has received.
#[derive(Default)]
struct StartParts(Vec<Option<Vec<u8>>>);

impl StartParts {
    /// Takes part `index` of `count`; the whole match start once every part is in. A count of 0, an index past it, a
    /// count that changes mid-transfer, or a start over `MAX_START_BYTES` is an error.
    fn add(&mut self, index: usize, count: usize, bytes: &[u8]) -> Result<Option<Vec<u8>>, String> {
        if index >= count {
            return Err(format!("the host sent match start part {index} of {count}"));
        }
        if count > MAX_START_BYTES.div_ceil(PART_BYTES) || bytes.len() > PART_BYTES {
            return Err("The host's match start is too large".to_string());
        }
        if self.0.is_empty() {
            self.0 = vec![None; count];
        } else if self.0.len() != count {
            return Err(format!("the host's match start changed from {} to {count} parts", self.0.len()));
        }
        self.0[index] = Some(bytes.to_vec());
        if !self.0.iter().all(Option::is_some) {
            return Ok(None);
        }
        Ok(Some(self.0.iter().flatten().flatten().copied().collect()))
    }
}

/// This client's `MSG_SLOT` for `slots`, with each one's entry in `pads` (keyboard when unknown).
fn slot_message(slots: &[u8], pads: &[Option<bool>; 4]) -> Vec<u8> {
    let mut msg = vec![MSG_SLOT, HANDSHAKE_VERSION];
    for &slot in slots {
        msg.extend([slot, pads.get(slot as usize).copied().flatten().unwrap_or(false) as u8]);
    }
    msg
}

/// Sets every player's `playerDevice` from the slots' announced devices, alike on every peer.
fn apply_pads(g: &mut Game, pads: &[Option<bool>; 4]) {
    for i in 0..g.players.len() {
        let player = g.player(g.players[i]);
        if let Some(Some(pad)) = pads.get(player.globalID as usize) {
            player.playerDevice = if *pad { 0 } else { DEVICE_KEYBOARD };
        }
    }
}

/// `kp--`: each room slot's player on the keyboard (`k`) or a gamepad (`p`), `-` for none, as `#@ v2` lines write them.
fn pads_token(pads: &[Option<bool>; 4]) -> String {
    pads.iter().map(|pad| match pad {
        Some(false) => 'k',
        Some(true) => 'p',
        None => '-',
    }).collect()
}

/// Reads a [`pads_token`].
fn parse_pads(token: &str) -> Option<[Option<bool>; 4]> {
    let mut pads = [None; 4];
    if token.len() != 4 {
        return None;
    }
    for (pad, c) in pads.iter_mut().zip(token.chars()) {
        *pad = match c {
            'k' => Some(false),
            'p' => Some(true),
            '-' => None,
            _ => return None,
        };
    }
    Some(pads)
}

/// Records `peer`'s claim on room `claimed`. Refuses a slot out of range or repeated, one of this client's, one another
/// peer holds, and a peer that changes its slots, so two GGRS handles never play one player.
fn claim_slots(slots: &mut BTreeMap<PeerId, Vec<u8>>, my_slots: &[u8], peer: PeerId, claimed: &[u8]) -> Result<(), String> {
    for (i, &slot) in claimed.iter().enumerate() {
        if slot as i32 >= MAX_PLAYERS || my_slots.contains(&slot) || claimed[..i].contains(&slot) {
            return Err(format!("slot {slot} is not another player's"));
        }
        if slots.iter().any(|(&p, held)| p != peer && held.contains(&slot)) {
            return Err(format!("slot {slot} is already taken"));
        }
    }
    match slots.get(&peer) {
        Some(old) if old != claimed => Err(format!("slots {claimed:?} after claiming slots {old:?}")),
        _ => {
            slots.insert(peer, claimed.to_vec());
            Ok(())
        }
    }
}

/// The slots in a peer's `MSG_SLOT`, each with whether its player is on a gamepad, or why this build cannot play with
/// that peer.
fn read_slot_message(msg: &[u8]) -> Result<Vec<(u8, bool)>, String> {
    match msg {
        [MSG_SLOT, HANDSHAKE_VERSION, pairs @ ..] if pairs.len() % 2 == 0 && (1..=MAX_PLAYERS as usize).contains(&(pairs.len() / 2)) => {
            pairs.chunks(2).map(|p| if p[1] <= 1 { Ok((p[0], p[1] == 1)) } else { Err(format!("a handshake with device {}", p[1])) }).collect()
        }
        [MSG_SLOT, HANDSHAKE_VERSION, ..] => Err(format!("a handshake with {} bytes of slots", msg.len() - 2)),
        [MSG_SLOT, version, _, ..] => Err(format!("handshake version {version}, this build speaks {HANDSHAKE_VERSION}")),
        _ => Err(format!("a handshake without a version, this build speaks {HANDSHAKE_VERSION}")),
    }
}

/// Ends the match on this client and gives the joiner's own settings back.
fn finish(g: &mut Game) {
    let mut gp = g.ptr();
    g.netplay.v2_match = false;
    if let Some(m) = g.netplay.client.v2.take() {
        if let Some((options, controls)) = m.own_config.clone() {
            gp.game_values.read_config_bytes(g, options, controls);
        }
        println!(
            "[net] v2: match over after {} ticks, {} re-run ticks, {} desyncs",
            m.ticks, m.rollback_ticks, m.desyncs
        );
        // Dropping the socket now could lose the LEAVE this frame sent; it closes a little later.
        gp.netplay.client.linger(m.socket);
    }
}

impl V2Match {
    /// Sends `data` reliably to every connected peer.
    fn send_all(&mut self, data: &[u8]) {
        for peer in self.socket.connected_peers().to_vec() {
            self.socket.send(RELIABLE, peer, data);
        }
    }

    /// Sends this player's slot and, on the host, the match start in parts.
    fn announce(&mut self) {
        self.send_all(&slot_message(&self.my_slots.clone(), &self.pads));
        if self.host {
            let parts: Vec<Vec<u8>> = self.start.chunks(PART_BYTES).map(|c| c.to_vec()).collect();
            let count = parts.len() as u16;
            for (i, part) in parts.into_iter().enumerate() {
                let mut msg = vec![MSG_START_PART];
                msg.extend_from_slice(&(i as u16).to_le_bytes());
                msg.extend_from_slice(&count.to_le_bytes());
                msg.extend_from_slice(&part);
                self.send_all(&msg);
            }
        } else if self.rogue {
            self.send_all(&slot_message(&[self.host_slot], &self.pads));
            self.send_all(&[MSG_START_PART, 0, 0, 0, 0]);
            self.send_all(&[MSG_START_PART, 0, 0, 1, 0, b'x']);
            self.send_all(&[MSG_GO]);
        }
    }

    /// Handles the handshake messages; a joiner restores the match start once all its parts arrived.
    fn receive_reliable(&mut self, g: &mut Game) -> Result<(), String> {
        for (peer, msg) in self.socket.receive(RELIABLE) {
            let from_host = self.host_peer == Some(peer);
            match msg.first() {
                Some(&MSG_SLOT) => {
                    let announced = read_slot_message(&msg).map_err(|e| {
                        println!("[net] v2: peer {:08x}: {e}", peer.0 >> 96);
                        let name = msg.get(2).map_or_else(|| "A player".to_string(), |&slot| player_name(&self.names, slot));
                        format!("{name} runs another game version")
                    })?;
                    let claimed: Vec<u8> = announced.iter().map(|&(slot, _)| slot).collect();
                    let claim = claim_slots(&mut self.slots, &self.my_slots, peer, &claimed);
                    if claim.is_ok() {
                        for &(slot, pad) in &announced {
                            self.pads[slot as usize] = Some(pad);
                        }
                    }
                    match claim {
                        Ok(()) if !self.host && claimed.contains(&self.host_slot) => self.host_peer = Some(peer),
                        Ok(()) => {}
                        Err(e) if !self.refused.contains(&peer) => {
                            self.refused.push(peer);
                            println!("[net] v2: ignoring peer {:08x}: {e}", peer.0 >> 96);
                        }
                        Err(_) => {}
                    }
                }
                Some(&MSG_START_PART) if msg.len() >= 5 && from_host && !self.restored => {
                    let index = u16::from_le_bytes([msg[1], msg[2]]) as usize;
                    let count = u16::from_le_bytes([msg[3], msg[4]]) as usize;
                    if let Some(bytes) = self.parts.add(index, count, &msg[5..])? {
                        if self.rec.is_some() {
                            let data = harness::base64_encode(&crate::smw::network::file_compressor::deflate(&bytes));
                            note(&mut self.rec, format!("r{}.{data}", slots_token(&self.my_slots)));
                        }
                        self.own_config = Some(restore_start(g, &self.my_slots, bytes)?);
                        self.restored = true;
                        self.local_devices = devices_of(g, self.my_slots.len());
                        let ready = [MSG_READY];
                        self.send_all(&ready);
                    }
                }
                Some(&MSG_READY) if self.host && self.slots.contains_key(&peer) => {
                    if !self.ready.contains(&peer) {
                        self.ready.push(peer);
                    }
                }
                Some(&MSG_GO) if from_host => self.go = true,
                Some(&MSG_LEAVE) if self.slots.contains_key(&peer) => {
                    return Err(format!("{} left before the match started", peer_name(&self.names, &self.slots, peer)));
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// The handshake: slots from everyone, the match start to the joiners, READY back to the host, then the host's GO.
    /// Every message repeats until GO, since a peer may still be connecting or not yet reading. False ends it.
    fn start_step(&mut self, g: &mut Game) -> bool {
        let dropped = self.socket.update_peers().into_iter().find(|&(_, state)| state == PeerState::Disconnected);
        if let Some((peer, _)) = dropped {
            println!("[net] v2: peer {:08x} disconnected during the start", peer.0 >> 96);
            let missing = self.missing_players();
            let name = match self.slots.get(&peer) {
                Some(_) => peer_name(&self.names, &self.slots, peer),
                None if !missing.is_empty() => missing.join(", "),
                None => "A player".to_string(),
            };
            notify(&mut self.rec, &mut self.notice, format!("{name} could not connect"));
            return false;
        }
        if let Err(e) = self.receive_reliable(g) {
            notify(&mut self.rec, &mut self.notice, e);
            return false;
        }
        if self.opened.elapsed() > START_DEADLINE {
            println!("[net] v2: the match did not start within {} s", START_DEADLINE.as_secs());
            let missing = self.missing_players();
            let text = if missing.is_empty() { "The match did not start in time".to_string() } else { format!("{} did not connect", missing.join(", ")) };
            notify(&mut self.rec, &mut self.notice, text);
            return false;
        }
        self.ticks += 1;
        if self.ticks.is_multiple_of(RESEND_TICKS) {
            self.announce();
            if self.restored && !self.host {
                self.send_all(&[MSG_READY]);
            }
        }
        let all_slots = self.slots.len() >= self.others;
        if !(self.restored && all_slots) {
            return true;
        }
        if self.host {
            if self.ready.len() < self.others {
                return true;
            }
            self.send_all(&[MSG_GO]);
        } else if !self.go {
            return true;
        }
        match self.start_session() {
            Ok(session) => {
                println!("[net] v2: GGRS session started in {} ms", self.opened.elapsed().as_millis());
                apply_pads(g, &self.pads);
                let slots = self.targets.iter().map(slot_token).collect::<Vec<_>>().join(",");
                note(&mut self.rec, format!("t{}:{slots}:{}", slots_token(&self.my_slots), pads_token(&self.pads)));
                self.ticks = 0;
                self.phase = Phase::Running(Box::new(session));
                true
            }
            Err(e) => {
                println!("[net] v2: cannot start the GGRS session: {e}");
                notify(&mut self.rec, &mut self.notice, "Cannot start the match".to_string());
                false
            }
        }
    }

    /// The room's other players this peer has not heard from: no slot, or on the host, no READY.
    fn missing_players(&self) -> Vec<String> {
        let heard: Vec<u8> = match self.host {
            true => self.ready.iter().filter_map(|peer| self.slots.get(peer)).flatten().copied().collect(),
            false => self.slots.values().flatten().copied().collect(),
        };
        missing_slots(&self.names, &self.my_slots, &heard).into_iter().map(|slot| player_name(&self.names, slot)).collect()
    }

    /// Builds the GGRS session; handles follow room slot order, so every peer numbers the players alike.
    fn start_session(&mut self) -> Result<P2PSession<GgrsConfig<PeerId>>, GgrsError> {
        let mut players: Vec<(u8, Option<PeerId>)> = self.slots.iter().flat_map(|(&peer, held)| held.iter().map(move |&slot| (slot, Some(peer)))).collect();
        players.extend(self.my_slots.iter().map(|&slot| (slot, None)));
        players.sort();
        let mut builder = SessionBuilder::<GgrsConfig<PeerId>>::new()
            .with_num_players(players.len())?
            .with_input_delay(INPUT_DELAY)
            .with_max_prediction_window(MAX_PREDICTION)
            .with_fps(crate::common::global_constants::TICKS_PER_SECOND as usize)?
            .with_desync_detection_mode(ggrs::DesyncDetection::On { interval: DESYNC_INTERVAL })
            .with_disconnect_timeout(DISCONNECT_TIMEOUT)
            .with_disconnect_notify_delay(INTERRUPTED_AFTER);
        self.targets.clear();
        for (handle, (slot, peer)) in players.iter().enumerate() {
            let player = match peer {
                None => PlayerType::Local,
                Some(peer) => PlayerType::Remote(*peer),
            };
            builder = builder.add_player(player, handle)?;
            self.targets.push(match self.my_slots.iter().position(|s| s == slot) {
                Some(local) if peer.is_none() => KeySlot::Local(local),
                _ => KeySlot::Net(*slot as usize),
            });
        }
        builder.start_p2p_session(self.socket.take_ggrs_socket())
    }

    /// Leaves the match for the menu and tells the other peers, who leave too, as everyone does when a player quits a
    /// local match.
    fn leave(&mut self, g: &mut Game) {
        self.send_all(&[MSG_LEAVE]);
        note(&mut self.rec, "eL".to_string());
        to_menu(g, false);
    }

    /// A match that could not start, or that every other player left, ends for this player, who goes back to the room.
    /// False, for `tick`.
    fn abandon(&mut self, g: &mut Game) -> bool {
        self.send_all(&[MSG_LEAVE]);
        keep_notice(g, &self.notice);
        note(&mut self.rec, "eA".to_string());
        to_menu(g, true);
        false
    }

    /// One frame of the running match. False when it ended.
    fn run_step(&mut self, g: &mut Game) -> bool {
        let mut gp = g.ptr();
        let Phase::Running(session) = &mut self.phase else { return false };
        session.poll_remote_clients();
        for event in session.events() {
            match event {
                GgrsEvent::DesyncDetected { frame, local_checksum, remote_checksum, addr } => {
                    self.desyncs += 1;
                    println!("[net] v2: desync at frame {frame} with {:08x}: {local_checksum:x} vs {remote_checksum:x}", addr.0 >> 96);
                    if !self.desynced.contains(&addr) {
                        self.desynced.push(addr);
                        notify(&mut self.rec, &mut self.notice, format!("Out of sync with {}", peer_name(&self.names, &self.slots, addr)));
                    }
                }
                GgrsEvent::Disconnected { addr } => {
                    self.gone += 1;
                    println!("[net] v2: peer {:08x} disconnected", addr.0 >> 96);
                    notify(&mut self.rec, &mut self.notice, format!("{} dropped out", peer_name(&self.names, &self.slots, addr)));
                }
                GgrsEvent::NetworkInterrupted { addr, disconnect_timeout } => {
                    println!("[net] v2: peer {:08x} silent; dropped in {} ms unless it returns", addr.0 >> 96, disconnect_timeout);
                    let frames = (disconnect_timeout as u64 * crate::common::global_constants::TICKS_PER_SECOND as u64 / 1000) as u32;
                    show_notice(&mut self.rec, &mut self.notice, lag_text(&self.names, &self.slots, addr), frames);
                }
                GgrsEvent::NetworkResumed { addr } => {
                    println!("[net] v2: peer {:08x} back", addr.0 >> 96);
                    if self.notice.text == lag_text(&self.names, &self.slots, addr) {
                        show_notice(&mut self.rec, &mut self.notice, String::new(), 0);
                    }
                }
                _ => {}
            }
        }

        let mut leaver = None;
        for (peer, msg) in self.socket.receive(RELIABLE) {
            match msg.first() {
                Some(&MSG_LEAVE) => leaver = Some(peer),
                // A joiner repeats READY until it hears GO.
                Some(&MSG_READY) if self.host && self.slots.contains_key(&peer) => self.socket.send(RELIABLE, peer, &[MSG_GO]),
                _ => {}
            }
        }
        if let Some(peer) = leaver {
            notify(&mut self.rec, &mut self.notice, format!("{} left the match", peer_name(&self.names, &self.slots, peer)));
            return self.abandon(g);
        }
        if self.gone >= self.others {
            println!("[net] v2: every other player is gone");
            return self.abandon(g);
        }

        note(&mut self.rec, "i".to_string());
        if local_input(g, &mut self.local_devices) {
            self.leave(g);
            return false;
        }

        if session.current_state() != SessionState::Running {
            return true;
        }
        // GGRS's time sync: a peer ahead of the others skips a frame now and then.
        if session.frames_ahead() > 0 && self.ticks.is_multiple_of(4) {
            self.ticks += 1;
            return true;
        }
        for handle in session.local_player_handles() {
            let KeySlot::Local(local) = self.targets[handle] else { continue };
            if session.add_local_input(handle, Keys::of(&self.local_devices[local])).is_err() {
                return true;
            }
        }
        let started = Instant::now();
        let requests = match session.advance_frame() {
            Ok(requests) => requests,
            Err(e) => {
                println!("[net] v2: {e}");
                return false;
            }
        };
        if self.rec.is_some() {
            for request in &requests {
                note(&mut self.rec, Req::of(request).token());
            }
        }
        let advances = requests.iter().filter(|r| matches!(r, ggrs::GgrsRequest::AdvanceFrame { .. })).count() as u64;
        // At the prediction window GGRS returns no AdvanceFrame rather than `PredictionThreshold`.
        if advances == 0 {
            self.perf.waits += 1;
        }
        self.rollback_ticks += advances.saturating_sub(1);
        self.ticks += 1;
        if self.ticks.is_multiple_of(600) {
            let p = std::mem::take(&mut self.perf);
            println!(
                "[net] v2: {} ticks, {} re-run ticks, {} desyncs; last 600: GGRS frames avg {:.2} ms, worst {:.2} ms, {} waits for input",
                self.ticks,
                self.rollback_ticks,
                self.desyncs,
                p.total_us as f64 / p.frames.max(1) as f64 / 1000.0,
                p.worst_us as f64 / 1000.0,
                p.waits
            );
        }
        let ended = rollback::handle_requests(g, requests, &self.targets, true, DESYNC_INTERVAL as i32);
        let spent = started.elapsed().as_micros() as u64;
        self.perf.frames += 1;
        self.perf.total_us += spent;
        self.perf.worst_us = self.perf.worst_us.max(spent);
        if ended || g.gsm.currentState != crate::smw::game_state::StateId::Gameplay {
            return false;
        }
        note(&mut self.rec, "d".to_string());
        draw_dialogs(g, &mut self.notice);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_messages_carry_the_handshake_version() {
        let pads = [Some(false), Some(true), None, Some(true)];
        assert_eq!(read_slot_message(&slot_message(&[2], &pads)), Ok(vec![(2, false)]));
        assert_eq!(read_slot_message(&slot_message(&[3, 0, 1], &pads)), Ok(vec![(3, true), (0, false), (1, true)]));
        assert!(read_slot_message(&[MSG_SLOT, HANDSHAKE_VERSION + 1, 2, 0]).is_err());
        assert!(read_slot_message(&[MSG_SLOT, 2]).is_err());
        assert!(read_slot_message(&[MSG_SLOT, HANDSHAKE_VERSION]).is_err(), "no slots");
        assert!(read_slot_message(&[MSG_SLOT, HANDSHAKE_VERSION, 1]).is_err(), "a slot without its device");
        assert!(read_slot_message(&[MSG_SLOT, HANDSHAKE_VERSION, 1, 2]).is_err(), "an unknown device");
        assert!(read_slot_message(&slot_message(&[0, 1, 2, 3, 0], &pads)).is_err(), "more slots than players");
    }

    #[test]
    fn pads_tokens_round_trip() {
        let pads = [Some(false), Some(true), None, Some(true)];
        assert_eq!(pads_token(&pads), "kp-p");
        assert_eq!(parse_pads("kp-p"), Some(pads));
        assert_eq!(parse_pads("kp"), None);
        assert_eq!(parse_pads("kpxp"), None);
    }

    #[test]
    fn slots_tokens_round_trip() {
        assert_eq!(slots_token(&[0, 2]), "0+2");
        assert_eq!(parse_slots("0+2"), Some(vec![0, 2]));
        assert_eq!(parse_slots("1"), Some(vec![1]), "a recording from before couch play");
        assert_eq!(parse_slots("1+x"), None);
    }

    #[test]
    fn peer_names_join_a_couch() {
        let names = ["Ann", "Bo", "Bo 2", "(empty)"].map(String::from);
        let slots = BTreeMap::from([(PeerId(1), vec![1, 2])]);
        assert_eq!(peer_name(&names, &slots, PeerId(1)), "Bo and Bo 2");
        assert_eq!(peer_name(&names, &slots, PeerId(2)), "A player");
    }

    #[test]
    fn start_parts_assemble_in_any_order() {
        let mut parts = StartParts::default();
        assert_eq!(parts.add(1, 2, b"cd"), Ok(None));
        assert_eq!(parts.add(1, 2, b"cd"), Ok(None), "a resent part");
        assert_eq!(parts.add(0, 2, b"ab"), Ok(Some(b"abcd".to_vec())));
    }

    #[test]
    fn start_parts_refuse_bad_counts() {
        assert!(StartParts::default().add(0, 0, b"").is_err(), "no parts");
        assert!(StartParts::default().add(2, 2, b"x").is_err(), "an index past the count");
        let mut parts = StartParts::default();
        assert_eq!(parts.add(0, 3, b"a"), Ok(None));
        assert!(parts.add(1, 2, b"b").is_err(), "a changed count");
    }

    #[test]
    fn start_parts_refuse_oversized_starts() {
        let most = MAX_START_BYTES / PART_BYTES;
        assert_eq!(StartParts::default().add(0, most, &[0; PART_BYTES]), Ok(None));
        assert!(StartParts::default().add(0, most + 1, b"x").is_err(), "too many parts");
        assert!(StartParts::default().add(0, 1, &[0; PART_BYTES + 1]).is_err(), "an oversized part");
    }

    #[test]
    fn missing_slots_skip_empty_and_own() {
        let names = ["Ann", "(empty)", "Bo", "Cy"].map(String::from);
        assert_eq!(missing_slots(&names, &[0], &[]), vec![2, 3]);
        assert_eq!(missing_slots(&names, &[0], &[3]), vec![2]);
        assert!(missing_slots(&names, &[2], &[0, 3]).is_empty());
        assert_eq!(missing_slots(&names, &[0, 3], &[]), vec![2]);
    }

    #[test]
    fn notices_are_recorded_as_tokens() {
        let (mut rec, mut notice) = (Some(Vec::new()), Notice::default());
        notify(&mut rec, &mut notice, "Bo dropped out".to_string());
        show_notice(&mut rec, &mut notice, String::new(), 0);
        let tokens = rec.unwrap();
        assert_eq!(parse_notice(&tokens[0][1..]), Some((NOTICE_FRAMES, "Bo dropped out".to_string())));
        assert_eq!(parse_notice(&tokens[1][1..]), Some((0, String::new())));
        assert!(notice.text.is_empty());
    }

    #[test]
    fn slot_claims_are_unique() {
        let (a, b) = (PeerId(1), PeerId(2));
        let mut slots = BTreeMap::new();
        assert!(claim_slots(&mut slots, &[0], a, &[1, 3]).is_ok());
        assert!(claim_slots(&mut slots, &[0], a, &[1, 3]).is_ok(), "a repeated claim");
        assert!(claim_slots(&mut slots, &[0], b, &[2, 3]).is_err(), "another peer's slot");
        assert!(claim_slots(&mut slots, &[0], b, &[0]).is_err(), "this client's slot");
        assert!(claim_slots(&mut slots, &[0], b, &[2, 2]).is_err(), "a slot twice");
        assert!(claim_slots(&mut slots, &[0], b, &[MAX_PLAYERS as u8]).is_err(), "out of range");
        assert!(claim_slots(&mut slots, &[0], a, &[1]).is_err(), "changed slots");
        assert!(claim_slots(&mut slots, &[0], b, &[2]).is_ok());
        assert_eq!(slots, BTreeMap::from([(a, vec![1, 3]), (b, vec![2])]));
    }
}
