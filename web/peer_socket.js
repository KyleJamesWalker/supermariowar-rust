// The browser build's netplay v2 peers (crates/smw-peer/src/web.rs): matchbox_socket's signaling protocol and
// data channel layout, so browser and native players share rooms. Linked with --js-library and polled every frame.
addToLibrary({
    $SMWPeer: {
        // Channel 0 reliable, channel 1 unordered with no retransmits (crates/smw-peer RELIABLE, UNRELIABLE).
        channels: [{ ordered: true }, { ordered: false, maxRetransmits: 0 }],
        iceServers: [],
        relayOnly: false,
        gatherTimeoutMs: 5000,
        socket: null,
        failed: false,
        id: null,
        peers: new Map(),
        changes: [],
        inbox: [[], []],
        keepAlive: null,

        uuidToBytes: (uuid) => {
            const hex = uuid.replace(/-/g, '');
            const bytes = new Uint8Array(16);
            for (let i = 0; i < 16; i++) bytes[i] = parseInt(hex.substr(i * 2, 2), 16);
            return bytes;
        },

        bytesToUuid: (bytes) => {
            const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
            return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
        },

        signal: (receiver, data) => {
            const socket = SMWPeer.socket;
            if (socket && socket.readyState === 1) socket.send(JSON.stringify({ Signal: { receiver, data } }));
        },

        open: (url, iceServers, relayOnly) => {
            SMWPeer.close();
            SMWPeer.iceServers = iceServers;
            SMWPeer.relayOnly = relayOnly;
            let socket;
            try {
                socket = new WebSocket(url);
            } catch (e) {
                console.error('[net] signaling: ' + e.message);
                SMWPeer.failed = true;
                return;
            }
            socket.onmessage = (e) => {
                if (SMWPeer.socket === socket && typeof e.data === 'string') SMWPeer.onEvent(JSON.parse(e.data));
            };
            socket.onclose = () => {
                if (SMWPeer.socket === socket) SMWPeer.failed = true;
            };
            SMWPeer.socket = socket;
            SMWPeer.keepAlive = setInterval(() => {
                if (socket.readyState === 1) socket.send('"KeepAlive"');
            }, 10000);
        },

        onEvent: (event) => {
            if (event.IdAssigned) SMWPeer.id = event.IdAssigned;
            else if (event.NewPeer) SMWPeer.offer(event.NewPeer);
            else if (event.PeerLeft) SMWPeer.drop(event.PeerLeft);
            else if (event.Signal) SMWPeer.onSignal(event.Signal.sender, event.Signal.data);
        },

        // A matchbox peer: the RTCPeerConnection and its two pre-negotiated channels.
        peer: (id) => {
            const pc = new RTCPeerConnection({
                iceServers: SMWPeer.iceServers,
                iceTransportPolicy: SMWPeer.relayOnly ? 'relay' : 'all',
            });
            const peer = { pc, channels: [], open: 0, connected: false, gone: false, remoteSet: false, candidates: [], pending: [] };
            // Firefox drops the connection without a handler here (matchbox issue 36).
            pc.oniceconnectionstatechange = () => {};
            pc.onicecandidate = (e) => {
                const candidate = JSON.stringify(e.candidate ? e.candidate.toJSON() : null);
                if (peer.remoteSet) SMWPeer.signal(id, { IceCandidate: candidate });
                else peer.pending.push(candidate);
            };
            SMWPeer.channels.forEach((config, index) => {
                const channel = pc.createDataChannel(`matchbox_socket_${index}`, { ...config, negotiated: true, id: index });
                channel.binaryType = 'arraybuffer';
                channel.onopen = () => {
                    if (++peer.open === SMWPeer.channels.length && !peer.gone) {
                        peer.connected = true;
                        SMWPeer.changes.push([id, 1]);
                    }
                };
                channel.onmessage = (e) => {
                    if (!peer.gone && e.data instanceof ArrayBuffer) SMWPeer.inbox[index].push([id, new Uint8Array(e.data)]);
                };
                channel.onclose = () => SMWPeer.drop(id);
                peer.channels.push(channel);
            });
            SMWPeer.peers.set(id, peer);
            return peer;
        },

        // Like matchbox, the description goes out after ICE gathering, but never later than gatherTimeoutMs.
        gathered: (pc) =>
            new Promise((resolve) => {
                if (pc.iceGatheringState === 'complete') return resolve();
                const timer = setTimeout(resolve, SMWPeer.gatherTimeoutMs);
                pc.addEventListener('icegatheringstatechange', () => {
                    if (pc.iceGatheringState === 'complete') {
                        clearTimeout(timer);
                        resolve();
                    }
                });
            }),

        remoteSet: async (id, peer) => {
            peer.remoteSet = true;
            for (const candidate of peer.pending.splice(0)) SMWPeer.signal(id, { IceCandidate: candidate });
            for (const candidate of peer.candidates.splice(0)) await SMWPeer.addCandidate(peer, candidate);
        },

        addCandidate: async (peer, json) => {
            try {
                const candidate = JSON.parse(json);
                if (candidate) await peer.pc.addIceCandidate(candidate);
            } catch (e) {
                console.warn('[net] ice candidate: ' + e.message);
            }
        },

        offer: async (id) => {
            const peer = SMWPeer.peer(id);
            try {
                await peer.pc.setLocalDescription(await peer.pc.createOffer());
                await SMWPeer.gathered(peer.pc);
                SMWPeer.signal(id, { Offer: peer.pc.localDescription.sdp });
            } catch (e) {
                console.error('[net] offer: ' + e.message);
                SMWPeer.drop(id);
            }
        },

        onSignal: async (id, data) => {
            let peer = SMWPeer.peers.get(id);
            try {
                if (data.Offer !== undefined) {
                    peer = peer ?? SMWPeer.peer(id);
                    await peer.pc.setRemoteDescription({ type: 'offer', sdp: data.Offer });
                    await SMWPeer.remoteSet(id, peer);
                    await peer.pc.setLocalDescription(await peer.pc.createAnswer());
                    await SMWPeer.gathered(peer.pc);
                    SMWPeer.signal(id, { Answer: peer.pc.localDescription.sdp });
                } else if (data.Answer !== undefined && peer) {
                    await peer.pc.setRemoteDescription({ type: 'answer', sdp: data.Answer });
                    await SMWPeer.remoteSet(id, peer);
                } else if (data.IceCandidate !== undefined && peer) {
                    if (peer.remoteSet) await SMWPeer.addCandidate(peer, data.IceCandidate);
                    else peer.candidates.push(data.IceCandidate);
                }
            } catch (e) {
                console.error('[net] handshake: ' + e.message);
                SMWPeer.drop(id);
            }
        },

        drop: (id) => {
            const peer = SMWPeer.peers.get(id);
            if (!peer || peer.gone) return;
            peer.gone = true;
            SMWPeer.changes.push([id, 0]);
            peer.channels.forEach((channel) => channel.close());
            peer.pc.close();
            SMWPeer.peers.delete(id);
        },

        send: (channel, id, bytes) => {
            const peer = SMWPeer.peers.get(id);
            const dc = peer && peer.connected && peer.channels[channel];
            if (!dc || dc.readyState !== 'open') return 0;
            dc.send(bytes);
            return 1;
        },

        close: () => {
            if (SMWPeer.keepAlive) clearInterval(SMWPeer.keepAlive);
            for (const id of [...SMWPeer.peers.keys()]) SMWPeer.drop(id);
            if (SMWPeer.socket) SMWPeer.socket.close();
            SMWPeer.socket = null;
            SMWPeer.keepAlive = null;
            SMWPeer.failed = false;
            SMWPeer.id = null;
            SMWPeer.changes = [];
            SMWPeer.inbox = [[], []];
        },
    },

    smw_peer_open__deps: ['$SMWPeer', '$UTF8ToString'],
    smw_peer_open: (url, iceServers, relayOnly) =>
        SMWPeer.open(UTF8ToString(url), JSON.parse(UTF8ToString(iceServers)), relayOnly !== 0),

    smw_peer_signaling_closed__deps: ['$SMWPeer'],
    smw_peer_signaling_closed: () => (SMWPeer.failed ? 1 : 0),

    smw_peer_id__deps: ['$SMWPeer'],
    smw_peer_id: (out) => {
        if (!SMWPeer.id) return 0;
        HEAPU8.set(SMWPeer.uuidToBytes(SMWPeer.id), out);
        return 1;
    },

    // 1 connected, 0 disconnected, -1 no change left.
    smw_peer_change__deps: ['$SMWPeer'],
    smw_peer_change: (out) => {
        const change = SMWPeer.changes.shift();
        if (!change) return -1;
        HEAPU8.set(SMWPeer.uuidToBytes(change[0]), out);
        return change[1];
    },

    smw_peer_send__deps: ['$SMWPeer'],
    smw_peer_send: (channel, id, data, len) =>
        SMWPeer.send(channel, SMWPeer.bytesToUuid(HEAPU8.subarray(id, id + 16)), HEAPU8.slice(data, data + len)),

    smw_peer_recv__deps: ['$SMWPeer'],
    smw_peer_recv: (channel, id, buf, cap) => {
        const message = SMWPeer.inbox[channel] && SMWPeer.inbox[channel].shift();
        if (!message) return -1;
        if (message[1].length > cap) return -2;
        HEAPU8.set(SMWPeer.uuidToBytes(message[0]), id);
        HEAPU8.set(message[1], buf);
        return message[1].length;
    },

    smw_peer_close__deps: ['$SMWPeer'],
    smw_peer_close: () => SMWPeer.close(),
});
