// The browser build's netplay v2 peers (crates/smw-peer/src/web.rs): matchbox_socket's signaling protocol and
// data channel layout, so browser and native players share rooms. Linked with --js-library and polled every frame.
addToLibrary({
    $SMWPeer: {
        // Channel 0 reliable, channel 1 unordered with no retransmits (crates/smw-peer RELIABLE, UNRELIABLE).
        channels: [{ ordered: true }, { ordered: false, maxRetransmits: 0 }],
        iceServers: [],
        relayOnly: false,
        gatherTimeoutMs: 5000,
        // As in crates/smw-peer/src/native.rs: keep-alive, silence before a resumable connection counts as lost, how
        // long it tries to resume, and how often the offering side checks a connection and repeats an ICE restart.
        keepAliveMs: 2000,
        silenceMs: 6000,
        resumeWindowMs: 25000,
        restartCheckMs: 1000,
        restartEveryMs: 4000,
        url: null,
        socket: null,
        // From the relay; null when the server cannot resume a dropped connection.
        token: null,
        lostAt: null,
        retryMs: 250,
        heardAt: 0,
        // Signals queued while the connection resumes.
        outbox: [],
        restartTimer: null,
        // Counts opens, so dropping a replaced Rust socket does not close the connection that replaced it.
        generation: 0,
        failed: false,
        id: null,
        peers: new Map(),
        changes: [],
        inbox: [[], []],
        inboxBytes: [0, 0],
        // Per channel, as in crates/smw-peer/src/native.rs: unreliable drops its oldest, reliable drops the peer.
        inboxPackets: 8192,
        inboxMaxBytes: 8 << 20,
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
            const text = JSON.stringify({ Signal: { receiver, data } });
            const socket = SMWPeer.socket;
            if (socket && socket.readyState === 1) socket.send(text);
            else if (SMWPeer.lostAt !== null) SMWPeer.outbox.push(text);
        },

        open: (url, iceServers, relayOnly) => {
            SMWPeer.close();
            const generation = ++SMWPeer.generation;
            SMWPeer.iceServers = iceServers;
            SMWPeer.relayOnly = relayOnly;
            SMWPeer.url = url;
            SMWPeer.connect(generation, url, false);
            SMWPeer.restartTimer = setInterval(SMWPeer.checkPeers, SMWPeer.restartCheckMs);
            return generation;
        },

        // One signaling connection. Lost without a close (an error, or silence once the relay handed out a token), it
        // resumes the same peer ID; a resume the relay refuses, or one that outlasts resumeWindowMs, fails the socket.
        connect: (generation, target, resuming) => {
            let socket;
            try {
                socket = new WebSocket(target);
            } catch (e) {
                console.error('[net] signaling: ' + e.message);
                SMWPeer.lost(generation, resuming, false);
                return;
            }
            let heardAny = false;
            const current = () => SMWPeer.generation === generation && SMWPeer.socket === socket;
            socket.onopen = () => {
                if (!current()) return;
                SMWPeer.heardAt = performance.now();
                for (const text of SMWPeer.outbox.splice(0)) socket.send(text);
            };
            socket.onmessage = (e) => {
                if (!current() || typeof e.data !== 'string') return;
                SMWPeer.heardAt = performance.now();
                let event;
                try {
                    event = JSON.parse(e.data);
                } catch (err) {
                    console.warn('[net] signaling: ' + err.message);
                    return;
                }
                if (!heardAny && resuming) console.log('[net] signaling resumed');
                heardAny = true;
                SMWPeer.lostAt = null;
                SMWPeer.retryMs = 250;
                if (event && typeof event === 'object') {
                    if (typeof event.ResumeToken === 'string') SMWPeer.token = event.ResumeToken;
                    SMWPeer.onEvent(event);
                }
            };
            socket.onclose = () => {
                if (current()) SMWPeer.lost(generation, resuming, heardAny);
            };
            SMWPeer.socket = socket;
            if (SMWPeer.keepAlive) clearInterval(SMWPeer.keepAlive);
            SMWPeer.heardAt = performance.now();
            SMWPeer.keepAlive = setInterval(() => {
                if (!current()) return;
                if (SMWPeer.token && performance.now() - SMWPeer.heardAt >= SMWPeer.silenceMs) {
                    console.warn('[net] signaling went silent');
                    // Abandoned, not closed: a close frame would make the relay give the place up.
                    socket.onclose = null;
                    socket.onmessage = null;
                    SMWPeer.lost(generation, resuming, heardAny);
                    return;
                }
                if (socket.readyState === 1) socket.send('"KeepAlive"');
            }, SMWPeer.keepAliveMs);
        },

        lost: (generation, resuming, heardAny) => {
            SMWPeer.socket = null;
            if (!SMWPeer.token || !SMWPeer.id || (resuming && !heardAny)) {
                SMWPeer.failed = true;
                return;
            }
            const now = performance.now();
            if (SMWPeer.lostAt === null) SMWPeer.lostAt = now;
            if (now - SMWPeer.lostAt >= SMWPeer.resumeWindowMs) {
                console.warn('[net] signaling did not come back');
                SMWPeer.failed = true;
                return;
            }
            const join = SMWPeer.url.includes('?') ? '&' : '?';
            const target = `${SMWPeer.url}${join}resume=${encodeURIComponent(SMWPeer.id)}&token=${encodeURIComponent(SMWPeer.token)}`;
            setTimeout(() => {
                if (SMWPeer.generation === generation) SMWPeer.connect(generation, target, true);
            }, SMWPeer.retryMs);
            SMWPeer.retryMs = Math.min(SMWPeer.retryMs * 2, 2000);
        },

        // The offering side restarts ICE on a connected peer whose connection is down; a restart whose answer was lost
        // waits in 'checking', so anything short of connected retries.
        checkPeers: () => {
            const now = performance.now();
            for (const [id, peer] of SMWPeer.peers) {
                if (!peer.offerer || !peer.connected || peer.gone) continue;
                const state = peer.pc.iceConnectionState;
                if (state === 'connected' || state === 'completed') continue;
                if (peer.restartedAt !== null && now - peer.restartedAt < SMWPeer.restartEveryMs) continue;
                peer.restartedAt = now;
                console.log('[net] restarting ICE with ' + id);
                SMWPeer.restart(id, peer);
            }
        },

        restart: async (id, peer) => {
            try {
                await peer.pc.setLocalDescription(await peer.pc.createOffer({ iceRestart: true }));
                SMWPeer.signal(id, { Offer: peer.pc.localDescription.sdp });
            } catch (e) {
                console.warn('[net] ICE restart: ' + e.message);
            }
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
            const peer = { pc, channels: [], open: 0, connected: false, gone: false, remoteSet: false, candidates: [], pending: [], offerer: false, restartedAt: null };
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
                    if (!peer.gone && e.data instanceof ArrayBuffer) SMWPeer.deliver(index, id, new Uint8Array(e.data));
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
            peer.offerer = true;
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
                    // A renegotiation answers at once; its candidates trickle.
                    if (!peer.connected) await SMWPeer.gathered(peer.pc);
                    SMWPeer.signal(id, { Answer: peer.pc.localDescription.sdp });
                } else if (data.Answer !== undefined && peer) {
                    await peer.pc.setRemoteDescription({ type: 'answer', sdp: data.Answer });
                    await SMWPeer.remoteSet(id, peer);
                } else if (data.IceCandidate !== undefined && peer) {
                    if (peer.remoteSet) await SMWPeer.addCandidate(peer, data.IceCandidate);
                    else peer.candidates.push(data.IceCandidate);
                }
            } catch (e) {
                if (peer && peer.connected) {
                    console.warn('[net] renegotiation: ' + e.message);
                    return;
                }
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

        deliver: (channel, id, bytes) => {
            const inbox = SMWPeer.inbox[channel];
            const full = () =>
                inbox.length >= SMWPeer.inboxPackets || SMWPeer.inboxBytes[channel] + bytes.length > SMWPeer.inboxMaxBytes;
            if (channel === 0 && full()) {
                console.warn('[net] peer ' + id + ' overflowed the reliable inbox; dropping it');
                SMWPeer.drop(id);
                return;
            }
            while (inbox.length && full()) SMWPeer.inboxBytes[channel] -= inbox.shift()[1].length;
            SMWPeer.inboxBytes[channel] += bytes.length;
            inbox.push([id, bytes]);
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
            if (SMWPeer.restartTimer) clearInterval(SMWPeer.restartTimer);
            for (const id of [...SMWPeer.peers.keys()]) SMWPeer.drop(id);
            if (SMWPeer.socket) SMWPeer.socket.close();
            SMWPeer.socket = null;
            SMWPeer.keepAlive = null;
            SMWPeer.restartTimer = null;
            SMWPeer.token = null;
            SMWPeer.lostAt = null;
            SMWPeer.retryMs = 250;
            SMWPeer.outbox = [];
            SMWPeer.failed = false;
            SMWPeer.id = null;
            SMWPeer.changes = [];
            SMWPeer.inbox = [[], []];
            SMWPeer.inboxBytes = [0, 0];
        },

        closeOpened: (generation) => {
            if (generation === SMWPeer.generation) SMWPeer.close();
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

    // The size of the next packet on `channel`, or -1 when there is none.
    smw_peer_next_len__deps: ['$SMWPeer'],
    smw_peer_next_len: (channel) => {
        const message = SMWPeer.inbox[channel] && SMWPeer.inbox[channel][0];
        return message ? message[1].length : -1;
    },

    smw_peer_recv__deps: ['$SMWPeer'],
    smw_peer_recv: (channel, id, buf, cap) => {
        const message = SMWPeer.inbox[channel] && SMWPeer.inbox[channel].shift();
        if (!message) return -1;
        SMWPeer.inboxBytes[channel] -= message[1].length;
        if (message[1].length > cap) return -2;
        HEAPU8.set(SMWPeer.uuidToBytes(message[0]), id);
        HEAPU8.set(message[1], buf);
        return message[1].length;
    },

    smw_peer_close__deps: ['$SMWPeer'],
    smw_peer_close: (generation) => SMWPeer.closeOpened(generation),
});
