//! Native peers in one room on an in-process signaling server: both channels, then a GGRS session.

use ggrs::{Config, GgrsRequest, InputStatus, PredictRepeatLast, SessionBuilder, SessionState};
use matchbox_signaling::SignalingServer;
use serde::{Deserialize, Serialize};
use smw_peer::{IceConfig, PeerId, PeerSocket, RELIABLE, UNRELIABLE};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

fn signaling_server() -> u16 {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async move {
            let mut server = SignalingServer::full_mesh_builder(([127, 0, 0, 1], 0)).build();
            tx.send(server.bind().unwrap().port()).unwrap();
            server.serve().await.unwrap();
        });
    });
    rx.recv().unwrap()
}

/// One room at a time: several meshes gathering ICE at once outrun the deadline on small CI runners.
static ONE_ROOM: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn connect_all(n: usize, room: &str) -> Vec<PeerSocket> {
    let port = signaling_server();
    let local = IceConfig {
        servers: Vec::new(),
        relay_only: false,
    };
    let mut sockets: Vec<PeerSocket> = (0..n)
        .map(|_| PeerSocket::connect(&format!("ws://127.0.0.1:{port}/{room}"), &local))
        .collect();
    let deadline = Instant::now() + Duration::from_secs(60);
    while sockets.iter().any(|s| s.connected_peers().len() < n - 1) {
        assert!(Instant::now() < deadline, "peers did not connect");
        for socket in &mut sockets {
            socket.update_peers();
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    sockets
}

#[test]
fn channels_reach_every_peer() {
    let _room = ONE_ROOM.lock().unwrap_or_else(|e| e.into_inner());
    let mut sockets = connect_all(3, "channels");
    let ids: Vec<PeerId> = sockets.iter_mut().map(|s| s.id().unwrap()).collect();
    for (i, socket) in sockets.iter_mut().enumerate() {
        for &peer in &ids {
            if peer != ids[i] {
                socket.send(RELIABLE, peer, &[i as u8; 3]);
                socket.send(UNRELIABLE, peer, &[i as u8 + 10]);
            }
        }
    }
    let mut got: Vec<Vec<(usize, PeerId, Vec<u8>)>> = vec![Vec::new(); 3];
    let deadline = Instant::now() + Duration::from_secs(10);
    while got.iter().any(|g| g.len() < 4) {
        assert!(Instant::now() < deadline, "messages missing: {got:?}");
        for (i, socket) in sockets.iter_mut().enumerate() {
            for channel in [RELIABLE, UNRELIABLE] {
                got[i].extend(
                    socket
                        .receive(channel)
                        .into_iter()
                        .map(|(p, d)| (channel, p, d)),
                );
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    for (i, messages) in got.iter().enumerate() {
        for (channel, from, data) in messages {
            let sender = ids.iter().position(|id| id == from).unwrap();
            assert_ne!(sender, i);
            let expected = if *channel == RELIABLE {
                vec![sender as u8; 3]
            } else {
                vec![sender as u8 + 10]
            };
            assert_eq!(data, &expected);
        }
    }

    sockets.pop().unwrap().close();
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let left: Vec<bool> = sockets
            .iter_mut()
            .map(|s| {
                s.update_peers();
                s.connected_peers().len() == 1
            })
            .collect();
        if left.iter().all(|&l| l) {
            break;
        }
        assert!(Instant::now() < deadline, "closed peer still connected");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[derive(Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
struct Input(u8);

struct Stub;

impl Config for Stub {
    type Input = Input;
    type InputPredictor = PredictRepeatLast;
    type State = u64;
    type Address = PeerId;
}

fn step(state: u64, inputs: &[(Input, InputStatus)]) -> u64 {
    inputs
        .iter()
        .fold(state.wrapping_mul(31).wrapping_add(1), |s, (input, _)| {
            s.wrapping_mul(131).wrapping_add(input.0 as u64)
        })
}

#[test]
fn ggrs_sessions_agree() {
    const PLAYERS: usize = 3;
    const CHECK: i32 = 120;
    let _room = ONE_ROOM.lock().unwrap_or_else(|e| e.into_inner());
    let mut sockets = connect_all(PLAYERS, "ggrs");
    let mut sessions: Vec<_> = sockets
        .iter_mut()
        .map(|socket| {
            let mut builder = SessionBuilder::<Stub>::new()
                .with_num_players(PLAYERS)
                .unwrap();
            for (handle, player) in socket.players().into_iter().enumerate() {
                builder = builder.add_player(player, handle).unwrap();
            }
            builder
                .start_p2p_session(socket.take_ggrs_socket())
                .unwrap()
        })
        .collect();
    let mut states = [0u64; PLAYERS];
    let mut frames = [0i32; PLAYERS];
    let mut history: Vec<BTreeMap<i32, u64>> = vec![BTreeMap::new(); PLAYERS];
    let deadline = Instant::now() + Duration::from_secs(30);
    while sessions.iter().any(|s| s.confirmed_frame() < CHECK) {
        let confirmed: Vec<i32> = sessions.iter().map(|s| s.confirmed_frame()).collect();
        assert!(
            Instant::now() < deadline,
            "sessions stalled at {confirmed:?}"
        );
        for (i, session) in sessions.iter_mut().enumerate() {
            session.poll_remote_clients();
            if session.current_state() != SessionState::Running {
                continue;
            }
            let frame = session.current_frame();
            for handle in session.local_player_handles() {
                session
                    .add_local_input(handle, Input((frame as usize * 7 + handle * 13) as u8 % 16))
                    .unwrap();
            }
            let Ok(requests) = session.advance_frame() else {
                continue;
            };
            for request in requests {
                match request {
                    GgrsRequest::SaveGameState { cell, frame } => {
                        cell.save(frame, Some(states[i]), None)
                    }
                    GgrsRequest::LoadGameState { cell, frame } => {
                        states[i] = cell.load().unwrap();
                        frames[i] = frame;
                    }
                    GgrsRequest::AdvanceFrame { inputs } => {
                        states[i] = step(states[i], &inputs);
                        frames[i] += 1;
                        history[i].insert(frames[i], states[i]);
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(16));
    }
    for i in 1..PLAYERS {
        assert_eq!(
            history[i].get(&CHECK),
            history[0].get(&CHECK),
            "peer {i} disagrees at frame {CHECK}"
        );
    }
}
