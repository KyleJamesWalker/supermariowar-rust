//! Native peers: a matchbox-protocol signaling client and webrtc-rs connections, run on one tokio thread.

use crate::{IceConfig, PeerId, PeerState, RELIABLE, UNRELIABLE};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::tungstenite::Message;
use webrtc::api::{APIBuilder, API};
use webrtc::data_channel::data_channel_init::RTCDataChannelInit;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::policy::ice_transport_policy::RTCIceTransportPolicy;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;

const KEEP_ALIVE: Duration = Duration::from_secs(10);

/// Test conditions for the unreliable channel: `SMW_PEER_LAG_MS` delays each packet, `SMW_PEER_LOSS` drops that
/// percentage of them.
struct Impairment {
    lag: Duration,
    loss_percent: u32,
    seed: std::cell::Cell<u64>,
}

impl Impairment {
    fn from_env() -> Impairment {
        let var = |name: &str| std::env::var(name).ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        Impairment { lag: Duration::from_millis(var("SMW_PEER_LAG_MS")), loss_percent: var("SMW_PEER_LOSS") as u32, seed: std::cell::Cell::new(0x9e37_79b9_7f4a_7c15) }
    }

    fn drop_next(&self) -> bool {
        if self.loss_percent == 0 {
            return false;
        }
        let mut x = self.seed.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.seed.set(x);
        (x % 100) < self.loss_percent as u64
    }
}

#[derive(Default)]
struct Shared {
    id: Option<PeerId>,
    changes: Vec<(PeerId, PeerState)>,
    inbox: [Vec<(PeerId, Vec<u8>)>; 2],
    signaling_closed: bool,
}

type SharedRef = Arc<Mutex<Shared>>;

enum Command {
    Send(usize, PeerId, Vec<u8>),
    Close,
}

pub struct Socket {
    shared: SharedRef,
    commands: UnboundedSender<Command>,
    unreliable_taken: bool,
}

impl Socket {
    pub fn connect(room_url: &str, ice: &IceConfig) -> Socket {
        let shared = SharedRef::default();
        let (commands, rx) = unbounded_channel();
        let url = room_url.to_string();
        let ice = ice.clone();
        let task_shared = shared.clone();
        std::thread::Builder::new()
            .name("smw-peer".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("tokio runtime");
                runtime.block_on(run(url, ice, task_shared.clone(), rx));
                task_shared.lock().unwrap().signaling_closed = true;
            })
            .expect("spawn the peer socket thread");
        Socket {
            shared,
            commands,
            unreliable_taken: false,
        }
    }

    pub fn id(&mut self) -> Option<PeerId> {
        self.shared.lock().unwrap().id
    }

    pub fn update_peers(&mut self) -> Vec<(PeerId, PeerState)> {
        std::mem::take(&mut self.shared.lock().unwrap().changes)
    }

    pub fn signaling_closed(&self) -> bool {
        self.shared.lock().unwrap().signaling_closed
    }

    pub fn send(&mut self, channel: usize, peer: PeerId, data: &[u8]) {
        let _ = self
            .commands
            .send(Command::Send(channel, peer, data.to_vec()));
    }

    pub fn receive(&mut self, channel: usize) -> Vec<(PeerId, Vec<u8>)> {
        if channel > UNRELIABLE || (channel == UNRELIABLE && self.unreliable_taken) {
            return Vec::new();
        }
        std::mem::take(&mut self.shared.lock().unwrap().inbox[channel])
    }

    pub fn take_unreliable(&mut self) -> Channel {
        self.unreliable_taken = true;
        Channel {
            shared: self.shared.clone(),
            commands: self.commands.clone(),
        }
    }

    pub fn close(&mut self) {
        let _ = self.commands.send(Command::Close);
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        self.close();
    }
}

pub struct Channel {
    shared: SharedRef,
    commands: UnboundedSender<Command>,
}

impl Channel {
    pub fn send(&mut self, peer: PeerId, packet: &[u8]) {
        let _ = self
            .commands
            .send(Command::Send(UNRELIABLE, peer, packet.to_vec()));
    }

    pub fn receive(&mut self) -> Vec<(PeerId, Vec<u8>)> {
        std::mem::take(&mut self.shared.lock().unwrap().inbox[UNRELIABLE])
    }
}

fn uuid_string(peer: PeerId) -> String {
    uuid::Uuid::from_u128(peer.0).hyphenated().to_string()
}

fn parse_uuid(value: &Value) -> Option<PeerId> {
    uuid::Uuid::parse_str(value.as_str()?)
        .ok()
        .map(|u| PeerId(u.as_u128()))
}

enum Signal {
    Offer(String),
    Answer(String),
    IceCandidate(String),
}

struct Peer {
    connection: Arc<RTCPeerConnection>,
    channels: Vec<Arc<RTCDataChannel>>,
    signals: UnboundedSender<Signal>,
    connected: Arc<AtomicBool>,
}

/// What a peer's callbacks need: where packets and state changes go, and how to reach the signaling server.
#[derive(Clone)]
struct Context {
    ice: RTCConfiguration,
    shared: SharedRef,
    outgoing: UnboundedSender<String>,
    gone: UnboundedSender<PeerId>,
}

impl Context {
    fn signal(&self, receiver: PeerId, data: Value) {
        let _ = self.outgoing.send(
            json!({ "Signal": { "receiver": uuid_string(receiver), "data": data } }).to_string(),
        );
    }
}

fn rtc_configuration(ice: &IceConfig) -> RTCConfiguration {
    let ice_servers = ice
        .servers
        .iter()
        .map(|s| RTCIceServer {
            urls: s.urls.clone(),
            username: s.username.clone(),
            credential: s.credential.clone(),
        })
        .collect();
    let ice_transport_policy = if ice.relay_only {
        RTCIceTransportPolicy::Relay
    } else {
        RTCIceTransportPolicy::All
    };
    RTCConfiguration {
        ice_servers,
        ice_transport_policy,
        ..Default::default()
    }
}

async fn run(
    url: String,
    ice: IceConfig,
    shared: SharedRef,
    mut commands: UnboundedReceiver<Command>,
) {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let stream = match tokio_tungstenite::connect_async(url.as_str()).await {
        Ok((stream, _)) => stream,
        Err(e) => {
            eprintln!("[net] signaling {url}: {e}");
            return;
        }
    };
    let (mut ws_tx, mut ws_rx) = stream.split();
    let (outgoing, mut outgoing_rx) = unbounded_channel::<String>();
    let (gone, mut gone_rx) = unbounded_channel::<PeerId>();
    let context = Context {
        ice: rtc_configuration(&ice),
        shared: shared.clone(),
        outgoing,
        gone,
    };
    let api = Arc::new(APIBuilder::new().build());
    let mut peers: HashMap<PeerId, Peer> = HashMap::new();
    let mut keep_alive = tokio::time::interval(KEEP_ALIVE);
    let impairment = Impairment::from_env();
    let mut signaling_open = true;

    loop {
        tokio::select! {
            message = ws_rx.next(), if signaling_open => match message {
                Some(Ok(Message::Text(text))) => {
                    if let Ok(event) = serde_json::from_str::<Value>(&text) {
                        on_event(&api, &context, &mut peers, &event).await;
                    }
                }
                Some(Ok(_)) => {}
                _ => {
                    signaling_open = false;
                    shared.lock().unwrap().signaling_closed = true;
                }
            },
            Some(text) = outgoing_rx.recv(), if signaling_open => {
                if ws_tx.send(Message::Text(text.into())).await.is_err() {
                    signaling_open = false;
                    shared.lock().unwrap().signaling_closed = true;
                }
            }
            _ = keep_alive.tick(), if signaling_open => {
                let _ = ws_tx.send(Message::Text("\"KeepAlive\"".into())).await;
            }
            Some(peer) = gone_rx.recv() => drop_peer(&shared, &mut peers, peer).await,
            command = commands.recv() => match command {
                Some(Command::Send(channel, peer, data)) => {
                    let open = peers.get(&peer).filter(|p| p.connected.load(Ordering::Acquire));
                    let index = channel;
                    if let Some(channel) = open.and_then(|p| p.channels.get(channel)) {
                        if index != UNRELIABLE {
                            let _ = channel.send(&Bytes::from(data)).await;
                        } else if !impairment.drop_next() {
                            if impairment.lag.is_zero() {
                                let _ = channel.send(&Bytes::from(data)).await;
                            } else {
                                let (channel, lag) = (channel.clone(), impairment.lag);
                                tokio::spawn(async move {
                                    tokio::time::sleep(lag).await;
                                    let _ = channel.send(&Bytes::from(data)).await;
                                });
                            }
                        }
                    }
                }
                Some(Command::Close) | None => {
                    for peer in peers.keys().copied().collect::<Vec<_>>() {
                        drop_peer(&shared, &mut peers, peer).await;
                    }
                    let _ = ws_tx.close().await;
                    return;
                }
            },
        }
    }
}

async fn on_event(
    api: &Arc<API>,
    context: &Context,
    peers: &mut HashMap<PeerId, Peer>,
    event: &Value,
) {
    if let Some(id) = event.get("IdAssigned").and_then(parse_uuid) {
        context.shared.lock().unwrap().id = Some(id);
    } else if let Some(id) = event.get("NewPeer").and_then(parse_uuid) {
        if let Some(peer) = new_peer(api, context, id, true).await {
            peers.insert(id, peer);
        }
    } else if let Some(id) = event.get("PeerLeft").and_then(parse_uuid) {
        let _ = context.gone.send(id);
    } else if let Some(signal) = event.get("Signal") {
        let (Some(sender), Some(data)) = (
            signal.get("sender").and_then(parse_uuid),
            signal.get("data"),
        ) else {
            return;
        };
        let text = |key: &str| data.get(key).and_then(Value::as_str).map(str::to_string);
        let signal = if let Some(sdp) = text("Offer") {
            Signal::Offer(sdp)
        } else if let Some(sdp) = text("Answer") {
            Signal::Answer(sdp)
        } else if let Some(candidate) = text("IceCandidate") {
            Signal::IceCandidate(candidate)
        } else {
            return;
        };
        if matches!(signal, Signal::Offer(_)) && !peers.contains_key(&sender) {
            let Some(peer) = new_peer(api, context, sender, false).await else {
                return;
            };
            peers.insert(sender, peer);
        }
        if let Some(peer) = peers.get(&sender) {
            let _ = peer.signals.send(signal);
        }
    }
}

async fn new_peer(api: &API, context: &Context, id: PeerId, offerer: bool) -> Option<Peer> {
    let connection = Arc::new(api.new_peer_connection(context.ice.clone()).await.ok()?);
    let connected = Arc::new(AtomicBool::new(false));
    let opened = Arc::new(AtomicUsize::new(0));
    let mut channels = Vec::new();
    for (index, ordered, max_retransmits) in [(RELIABLE, true, None), (UNRELIABLE, false, Some(0))]
    {
        let init = RTCDataChannelInit {
            ordered: Some(ordered),
            max_retransmits,
            negotiated: Some(index as u16),
            ..Default::default()
        };
        let channel = connection
            .create_data_channel(&format!("matchbox_socket_{index}"), Some(init))
            .await
            .ok()?;
        let (opened, connected, shared) =
            (opened.clone(), connected.clone(), context.shared.clone());
        channel.on_open(Box::new(move || {
            if opened.fetch_add(1, Ordering::AcqRel) + 1 == 2 {
                connected.store(true, Ordering::Release);
                shared
                    .lock()
                    .unwrap()
                    .changes
                    .push((id, PeerState::Connected));
            }
            Box::pin(async {})
        }));
        let shared = context.shared.clone();
        channel.on_message(Box::new(move |message| {
            shared.lock().unwrap().inbox[index].push((id, message.data.to_vec()));
            Box::pin(async {})
        }));
        let gone = context.gone.clone();
        channel.on_close(Box::new(move || {
            let _ = gone.send(id);
            Box::pin(async {})
        }));
        channels.push(channel);
    }
    let gone = context.gone.clone();
    connection.on_peer_connection_state_change(Box::new(move |state| {
        if matches!(
            state,
            RTCPeerConnectionState::Failed | RTCPeerConnectionState::Closed
        ) {
            let _ = gone.send(id);
        }
        Box::pin(async {})
    }));

    let remote_set = Arc::new(AtomicBool::new(false));
    let pending = Arc::new(Mutex::new(Vec::<String>::new()));
    {
        let (context, remote_set, pending) = (context.clone(), remote_set.clone(), pending.clone());
        connection.on_ice_candidate(Box::new(move |candidate| {
            let json = candidate
                .and_then(|c| c.to_json().ok())
                .and_then(|c| serde_json::to_string(&c).ok());
            if let Some(json) = json {
                if remote_set.load(Ordering::Acquire) {
                    context.signal(id, json!({ "IceCandidate": json }));
                } else {
                    pending.lock().unwrap().push(json);
                }
            }
            Box::pin(async {})
        }));
    }

    let (signals, signals_rx) = unbounded_channel();
    let handshake = Handshake {
        connection: connection.clone(),
        context: context.clone(),
        id,
        offerer,
        remote_set,
        pending,
    };
    tokio::spawn(handshake.run(signals_rx));
    Some(Peer {
        connection,
        channels,
        signals,
        connected,
    })
}

struct Handshake {
    connection: Arc<RTCPeerConnection>,
    context: Context,
    id: PeerId,
    offerer: bool,
    remote_set: Arc<AtomicBool>,
    pending: Arc<Mutex<Vec<String>>>,
}

impl Handshake {
    async fn run(self, mut signals: UnboundedReceiver<Signal>) {
        if let Err(e) = self.exchange(&mut signals).await {
            eprintln!("[net] handshake with {:032x}: {e}", self.id.0);
            let _ = self.context.gone.send(self.id);
        }
    }

    async fn exchange(&self, signals: &mut UnboundedReceiver<Signal>) -> Result<(), webrtc::Error> {
        let (connection, context, id) = (&self.connection, &self.context, self.id);
        let mut early_candidates = Vec::new();
        if self.offerer {
            let offer = connection.create_offer(None).await?;
            let sdp = offer.sdp.clone();
            connection.set_local_description(offer).await?;
            context.signal(id, json!({ "Offer": sdp }));
        }
        while let Some(signal) = signals.recv().await {
            let remote = match signal {
                Signal::Offer(sdp) if !self.offerer => RTCSessionDescription::offer(sdp)?,
                Signal::Answer(sdp) if self.offerer => RTCSessionDescription::answer(sdp)?,
                Signal::IceCandidate(json) => {
                    match serde_json::from_str::<RTCIceCandidateInit>(&json) {
                        Ok(candidate) if self.remote_set.load(Ordering::Acquire) => {
                            connection.add_ice_candidate(candidate).await?
                        }
                        Ok(candidate) => early_candidates.push(candidate),
                        Err(_) => {}
                    }
                    continue;
                }
                _ => continue,
            };
            connection.set_remote_description(remote).await?;
            self.remote_set.store(true, Ordering::Release);
            for json in std::mem::take(&mut *self.pending.lock().unwrap()) {
                context.signal(id, json!({ "IceCandidate": json }));
            }
            for candidate in early_candidates.drain(..) {
                connection.add_ice_candidate(candidate).await?;
            }
            if !self.offerer {
                let answer = connection.create_answer(None).await?;
                let sdp = answer.sdp.clone();
                connection.set_local_description(answer).await?;
                context.signal(id, json!({ "Answer": sdp }));
            }
        }
        Ok(())
    }
}

async fn drop_peer(shared: &SharedRef, peers: &mut HashMap<PeerId, Peer>, id: PeerId) {
    let Some(peer) = peers.remove(&id) else {
        return;
    };
    if peer.connected.load(Ordering::Acquire) {
        shared
            .lock()
            .unwrap()
            .changes
            .push((id, PeerState::Disconnected));
    }
    let _ = peer.connection.close().await;
}
