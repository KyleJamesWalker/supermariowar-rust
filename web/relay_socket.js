// The browser build's one WebSocket to smw_relay (src/smw/platform/network/websocket). Linked with
// --js-library; the game polls it every frame, so nothing here calls back into wasm.
addToLibrary({
    $SMWRelay: { socket: null, inbox: [], failed: false },

    smw_ws_open__deps: ['$SMWRelay', '$UTF8ToString'],
    smw_ws_open: (url) => {
        if (SMWRelay.socket) SMWRelay.socket.close();
        SMWRelay.socket = null;
        SMWRelay.inbox = [];
        SMWRelay.failed = false;
        let socket;
        try {
            socket = new WebSocket(UTF8ToString(url));
        } catch (e) {
            console.error('[net] relay: ' + e.message);
            SMWRelay.failed = true;
            return;
        }
        socket.binaryType = 'arraybuffer';
        socket.onmessage = (e) => {
            if (SMWRelay.socket === socket && e.data instanceof ArrayBuffer) SMWRelay.inbox.push(new Uint8Array(e.data));
        };
        SMWRelay.socket = socket;
    },

    // -1 no socket, 0 connecting, 1 open, 2 closed or failed.
    smw_ws_state__deps: ['$SMWRelay'],
    smw_ws_state: () => {
        const socket = SMWRelay.socket;
        if (!socket) return SMWRelay.failed ? 2 : -1;
        return socket.readyState >= 2 ? 2 : socket.readyState;
    },

    smw_ws_send__deps: ['$SMWRelay'],
    smw_ws_send: (data, len) => {
        const socket = SMWRelay.socket;
        if (!socket || socket.readyState !== 1) return 0;
        socket.send(HEAPU8.slice(data, data + len));
        return 1;
    },

    smw_ws_recv__deps: ['$SMWRelay'],
    smw_ws_recv: (buf, cap) => {
        const message = SMWRelay.inbox.shift();
        if (!message) return -1;
        if (message.length > cap) return -2;
        HEAPU8.set(message, buf);
        return message.length;
    },

    smw_ws_close__deps: ['$SMWRelay'],
    smw_ws_close: () => {
        if (SMWRelay.socket) SMWRelay.socket.close();
        SMWRelay.socket = null;
        SMWRelay.inbox = [];
        SMWRelay.failed = false;
    },

    smw_relay_url_param__deps: ['$stringToUTF8'],
    smw_relay_url_param: (buf, cap) => {
        const url = new URLSearchParams(location.search).get('relay');
        return url ? stringToUTF8(url, buf, cap) : 0;
    },
});
