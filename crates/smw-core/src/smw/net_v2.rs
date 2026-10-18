//! Netplay v2 matches (docs/design/NETPLAY_V2.md, rollout step 4): every peer restores the host's match start, then
//! the match runs under a GGRS P2P session over the room's WebRTC connections. Opt in with `SMW_NETPLAY_V2=1` on every
//! client; without it the match runs over v1 as before.

use crate::common::input::COutputControl;
use crate::game::Game;
use crate::smw::rollback::{self, GgrsConfig, KeySlot, Keys};
use ggrs::{GgrsError, GgrsEvent, P2PSession, PlayerType, SessionBuilder, SessionState};
use smw_peer::{PeerId, PeerSocket, RELIABLE};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

const MSG_SLOT: u8 = 1;
const MSG_START_PART: u8 = 2;
const MSG_READY: u8 = 3;
const MSG_GO: u8 = 4;
const MSG_LEAVE: u8 = 5;
const PART_BYTES: usize = 16 * 1024;
const START_DEADLINE: Duration = Duration::from_secs(20);
const RESEND_TICKS: u64 = 30;
const INPUT_DELAY: usize = 2;
const MAX_PREDICTION: usize = 8;
const DESYNC_INTERVAL: u32 = 30;
/// A peer is dropped for good after this much silence; GGRS's 2 s default is shorter than a Wi-Fi mesh's node hop.
const DISCONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const INTERRUPTED_AFTER: Duration = Duration::from_secs(1);

pub fn enabled() -> bool {
    std::env::var("SMW_NETPLAY_V2").is_ok_and(|v| v == "1")
}

enum Phase {
    Starting,
    Running(Box<P2PSession<GgrsConfig<PeerId>>>),
}

pub struct V2Match {
    socket: PeerSocket,
    host: bool,
    my_slot: u8,
    others: usize,
    slots: BTreeMap<PeerId, u8>,
    start: Vec<u8>,
    parts: Vec<Option<Vec<u8>>>,
    ready: Vec<PeerId>,
    restored: bool,
    go: bool,
    /// A joiner's own options.bin and controls bytes: the host's match start replaces them for the match.
    own_config: Option<(Vec<u8>, Vec<u8>)>,
    phase: Phase,
    targets: Vec<KeySlot>,
    local_device: COutputControl,
    opened: Instant,
    ticks: u64,
    rollback_ticks: u64,
    desyncs: u32,
    perf: V2Perf,
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
    if !enabled() || !g.netplay.active || crate::smw::network::net_record::replaying(g) {
        return false;
    }
    let Some((socket, others)) = gp.netplay.client.take_peer_socket() else {
        println!("[net] v2: the lobby sent no WebRTC room; this match runs over v1");
        return false;
    };
    let host = g.netplay.theHostIsMe;
    g.netplay.v2_match = true;
    let start = if host { crate::smw::checkpoint::capture_shared(g) } else { Vec::new() };
    let mut m = V2Match {
        socket,
        host,
        my_slot: g.netplay.remotePlayerNumber as u8,
        others,
        slots: BTreeMap::new(),
        start,
        parts: Vec::new(),
        ready: Vec::new(),
        restored: host,
        go: false,
        own_config: None,
        phase: Phase::Starting,
        targets: Vec::new(),
        local_device: g.game_values.playerInput.outputControls[0],
        opened: Instant::now(),
        ticks: 0,
        rollback_ticks: 0,
        desyncs: 0,
        perf: V2Perf::default(),
    };
    m.announce();
    println!("[net] v2: starting the match with {} peers ({})", others, if host { "host" } else { "joiner" });
    gp.netplay.client.v2 = Some(Box::new(m));
    !host
}

pub fn owns_tick(g: &mut Game) -> bool {
    g.netplay.client.v2.is_some()
}

/// One frame of a v2 match: the start handshake, then input, GGRS and the ticks it asks for.
pub fn tick(g: &mut Game) {
    let (mut gp, mut mp) = (g.ptr(), g.ptr());
    gp.netplay.client.update(g);
    let Some(m) = mp.netplay.client.v2.as_mut() else { return };
    let keep = match m.phase {
        Phase::Starting => m.start_step(g),
        Phase::Running(_) => m.run_step(g),
    };
    if !keep {
        finish(g);
    }
}

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
    }
}

impl V2Match {
    fn send_all(&mut self, data: &[u8]) {
        for peer in self.socket.connected_peers().to_vec() {
            self.socket.send(RELIABLE, peer, data);
        }
    }

    fn announce(&mut self) {
        let slot = [MSG_SLOT, self.my_slot];
        self.send_all(&slot);
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
        }
    }

    fn receive_reliable(&mut self, g: &mut Game) -> Result<(), String> {
        for (peer, msg) in self.socket.receive(RELIABLE) {
            match msg.first() {
                Some(&MSG_SLOT) if msg.len() == 2 => {
                    self.slots.insert(peer, msg[1]);
                }
                Some(&MSG_START_PART) if msg.len() >= 5 && !self.host => {
                    let index = u16::from_le_bytes([msg[1], msg[2]]) as usize;
                    let count = u16::from_le_bytes([msg[3], msg[4]]) as usize;
                    if self.parts.len() != count {
                        self.parts = vec![None; count];
                    }
                    if index < count {
                        self.parts[index] = Some(msg[5..].to_vec());
                    }
                    if !self.restored && self.parts.iter().all(Option::is_some) {
                        let bytes: Vec<u8> = self.parts.iter().flatten().flatten().copied().collect();
                        let mut gp = g.ptr();
                        self.own_config = Some(gp.game_values.config_bytes(g));
                        let slot = self.my_slot as usize;
                        let own_skin = g.game_values.skinids[slot];
                        crate::smw::checkpoint::restore_shared(g, bytes)?;
                        // The host's start holds the host's skin choices; this player's own skin is its choice here.
                        g.game_values.skinids[slot] = own_skin;
                        let strip = gp.rm.load_full_skin(g, own_skin, g.game_values.colorids[slot]);
                        gp.rm.set_strip(gp.rm.spr_player[slot], strip);
                        // The host captured mid-tick; this finishes that tick as its frame_end did there.
                        crate::smw::harness::end_rollback_tick(g);
                        self.restored = true;
                        self.local_device = g.game_values.playerInput.outputControls[0];
                        let ready = [MSG_READY];
                        self.send_all(&ready);
                        println!("[net] v2: restored the host's match start");
                    }
                }
                Some(&MSG_READY) if self.host => {
                    if !self.ready.contains(&peer) {
                        self.ready.push(peer);
                    }
                }
                Some(&MSG_GO) if !self.host => self.go = true,
                _ => {}
            }
        }
        Ok(())
    }

    /// The handshake: slots from everyone, the match start to the joiners, READY back to the host, then the host's GO.
    /// Every message repeats until GO, since a peer may still be connecting or not yet reading. False ends it.
    fn start_step(&mut self, g: &mut Game) -> bool {
        self.socket.update_peers();
        if let Err(e) = self.receive_reliable(g) {
            println!("[net] v2: {e}");
            return false;
        }
        if self.opened.elapsed() > START_DEADLINE {
            println!("[net] v2: the match did not start within {} s", START_DEADLINE.as_secs());
            return false;
        }
        self.ticks += 1;
        if self.ticks % RESEND_TICKS == 0 {
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
                self.ticks = 0;
                self.phase = Phase::Running(Box::new(session));
                true
            }
            Err(e) => {
                println!("[net] v2: cannot start the GGRS session: {e}");
                false
            }
        }
    }

    fn start_session(&mut self) -> Result<P2PSession<GgrsConfig<PeerId>>, GgrsError> {
        let mut players: Vec<(u8, Option<PeerId>)> = self.slots.iter().map(|(&peer, &slot)| (slot, Some(peer))).collect();
        players.push((self.my_slot, None));
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
            self.targets.push(if peer.is_none() { KeySlot::Local(0) } else { KeySlot::Net(*slot as usize) });
        }
        builder.start_p2p_session(self.socket.take_ggrs_socket())
    }

    /// Leaves the match for the menu and tells the other peers, who follow once their match is over too.
    fn leave(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.send_all(&[MSG_LEAVE]);
        gp.netplay.client.send_leave_game_message(g);
        crate::smw::game_state::GameStateManager::instance(g).change_state_to(gp.get(), crate::smw::game_state::StateId::Menu);
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
                }
                GgrsEvent::Disconnected { addr } => println!("[net] v2: peer {:08x} disconnected", addr.0 >> 96),
                GgrsEvent::NetworkInterrupted { addr, disconnect_timeout } => {
                    println!("[net] v2: peer {:08x} silent; dropped in {} ms unless it returns", addr.0 >> 96, disconnect_timeout)
                }
                GgrsEvent::NetworkResumed { addr } => println!("[net] v2: peer {:08x} back", addr.0 >> 96),
                _ => {}
            }
        }

        let others_left = self.socket.receive(RELIABLE).iter().any(|(_, msg)| msg.first() == Some(&MSG_LEAVE));
        if others_left && g.mode().gameover {
            println!("[net] v2: another player ended the match");
            crate::smw::gs_gameplay::clean_up(g);
            g.game_values.appstate = crate::common::game_values::AppState::Menu;
            self.leave(g);
            return false;
        }

        crate::smw::harness::frame_start_input(g);
        g.game_values.playerInput.outputControls[0] = self.local_device;
        let gameplay = gp.gps.as_mut().expect("a v2 match outside gameplay");
        gameplay.gather_input(g);
        self.local_device = g.game_values.playerInput.outputControls[0];
        if crate::smw::gs_gameplay::update_exit_pause_dialog(g, gameplay.countdown_state()) {
            self.leave(g);
            return false;
        }

        if session.current_state() != SessionState::Running {
            return true;
        }
        // GGRS's time sync: a peer ahead of the others skips a frame now and then.
        if session.frames_ahead() > 0 && self.ticks % 4 == 0 {
            self.ticks += 1;
            return true;
        }
        for handle in session.local_player_handles() {
            if session.add_local_input(handle, Keys::of(&self.local_device)).is_err() {
                return true;
            }
        }
        let started = Instant::now();
        let requests = match session.advance_frame() {
            Ok(requests) => requests,
            Err(GgrsError::PredictionThreshold) => {
                self.perf.waits += 1;
                return true;
            }
            Err(e) => {
                println!("[net] v2: {e}");
                return false;
            }
        };
        let advances = requests.iter().filter(|r| matches!(r, ggrs::GgrsRequest::AdvanceFrame { .. })).count() as u64;
        self.rollback_ticks += advances.saturating_sub(1);
        self.ticks += 1;
        if self.ticks % 600 == 0 {
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
        if g.game_values.flags.pausegame || g.game_values.flags.exitinggame {
            crate::smw::gs_gameplay::draw_exit_pause_dialog(g);
        }
        crate::smw::gs_gameplay::play_music(g);
        true
    }
}
