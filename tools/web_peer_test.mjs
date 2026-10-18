#!/usr/bin/env node
// Browser and native netplay v2 peers in one room: starts the smw-peer `signal` example, one native `echo_peer`,
// and two headless Chrome pages that load web/peer_socket.js on its own (no wasm). Every pair must connect, and
// every page must get the other page's greetings, the native peer's greetings and its own echoes on both channels.
//
// Usage: node tools/web_peer_test.mjs [out_dir]
//   CHROME        Chrome binary (default: the macOS Google Chrome app)
//   EXAMPLES_DIR  where `cargo build -p smw-peer --examples --release` put signal and echo_peer
//                 (default: target/release/examples)
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const chrome = process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const examples = resolve(process.env.EXAMPLES_DIR ?? join(repo, 'target', 'release', 'examples'));
const out = resolve(process.argv[2] ?? join(tmpdir(), 'smw-web-peer'));
mkdirSync(out, { recursive: true });

const children = [];
const profiles = [];
const cleanup = () => {
    for (const child of children) {
        try {
            process.kill(-child.pid, 'SIGKILL');
        } catch {}
    }
    for (const profile of profiles) rmSync(profile, { recursive: true, force: true });
};
const fail = (message) => {
    console.error(`FAIL: ${message}`);
    cleanup();
    process.exit(1);
};
setTimeout(() => fail('timed out after 90 s'), 90_000).unref();

const start = (bin, args, log) => {
    const child = spawn(bin, args, { stdio: ['ignore', 'pipe', 'pipe'], detached: true });
    children.push(child);
    let text = '';
    const append = (d) => {
        text += d;
        writeFileSync(join(out, log), text);
    };
    child.stdout.on('data', append);
    child.stderr.on('data', append);
    return { child, output: () => text };
};

const signal = start(join(examples, 'signal'), [], 'signal.log');
const signalPort = await new Promise((ok) => {
    const poll = setInterval(() => {
        const m = signal.output().match(/^listening (\d+)$/m);
        if (m) {
            clearInterval(poll);
            ok(Number(m[1]));
        }
    }, 20);
});
const room = `ws://127.0.0.1:${signalPort}/web-peer-test`;

const page = `<!doctype html><meta charset="utf-8"><title>peer test</title>
<script>window.addToLibrary = (lib) => { window.SMWPeer = lib.$SMWPeer; };</script>
<script src="peer_socket.js"></script>
<script>
SMWPeer.iceServers = [];
window.received = [new Set(), new Set()];
window.connected = new Set();
window.run = (url, name) => {
    SMWPeer.open(url);
    setInterval(() => {
        for (let c; (c = SMWPeer.changes.shift()); ) c[1] ? connected.add(c[0]) : connected.delete(c[0]);
        for (let ch = 0; ch < 2; ch++)
            for (let m; (m = SMWPeer.inbox[ch].shift()); ) received[ch].add(new TextDecoder().decode(m[1]));
    }, 16);
    setInterval(() => {
        for (const id of connected)
            for (let ch = 0; ch < 2; ch++) SMWPeer.send(ch, id, new TextEncoder().encode(name + '-hello/' + ch));
    }, 500);
};
window.state = () => ({ id: SMWPeer.id, connected: [...connected], received: received.map((s) => [...s].sort()) });
</script>`;
const server = createServer((req, res) => {
    if (req.url === '/peer_socket.js') {
        res.writeHead(200, { 'Content-Type': 'text/javascript' });
        res.end(readFileSync(join(repo, 'web', 'peer_socket.js')));
    } else {
        res.writeHead(200, { 'Content-Type': 'text/html' });
        res.end(page);
    }
});
await new Promise((ok) => server.listen(0, '127.0.0.1', ok));
server.unref();
const origin = `http://127.0.0.1:${server.address().port}`;

const profile = mkdtempSync(join(tmpdir(), 'smw-peer-chrome-'));
profiles.push(profile);
const browser = spawn(chrome, [
    '--headless=new', '--remote-debugging-port=0', `--user-data-dir=${profile}`, '--disable-extensions', '--mute-audio',
    'about:blank',
], { stdio: ['ignore', 'ignore', 'pipe'], detached: true });
children.push(browser);
const wsUrl = await new Promise((ok, no) => {
    let buf = '';
    browser.stderr.on('data', (d) => {
        buf += d;
        const m = buf.match(/DevTools listening on (ws:\/\/\S+)/);
        if (m) ok(m[1]);
    });
    browser.on('exit', () => no(new Error('chrome exited')));
});
const ws = new WebSocket(wsUrl);
await new Promise((ok) => ws.addEventListener('open', ok));
let nextId = 1;
const pending = new Map();
const consoleLines = [];
ws.addEventListener('message', (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.id && pending.has(msg.id)) {
        const { ok, no } = pending.get(msg.id);
        pending.delete(msg.id);
        msg.error ? no(new Error(msg.error.message)) : ok(msg.result);
    } else if (msg.method === 'Runtime.consoleAPICalled') {
        consoleLines.push(msg.params.args.map((a) => a.value ?? a.description ?? '').join(' '));
    }
});
const send = (method, params = {}, sessionId) =>
    new Promise((ok, no) => {
        const id = nextId++;
        pending.set(id, { ok, no });
        ws.send(JSON.stringify({ id, method, params, sessionId }));
    });
const openPage = async (name) => {
    const { targetId } = await send('Target.createTarget', { url: 'about:blank' });
    const { sessionId } = await send('Target.attachToTarget', { targetId, flatten: true });
    await send('Runtime.enable', {}, sessionId);
    await send('Page.navigate', { url: `${origin}/` }, sessionId);
    const evaluate = async (expression) =>
        (await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true }, sessionId)).result.value;
    while (!(await evaluate('typeof window.run === "function"'))) await new Promise((ok) => setTimeout(ok, 50));
    await evaluate(`run(${JSON.stringify(room)}, ${JSON.stringify(name)})`);
    return { name, evaluate };
};

const pages = [await openPage('a'), await openPage('b')];
const native = start(join(examples, 'echo_peer'), [room, '80'], 'echo_peer.log');

const expected = (name, other) =>
    [0, 1].map((ch) => [`${other}-hello/${ch}`, `echo:${name}-hello/${ch}`, `native-hello/${ch}`].sort());
const deadline = Date.now() + 60_000;
let states;
for (;;) {
    states = await Promise.all(pages.map((p) => p.evaluate('state()')));
    const done = states.every((s, i) => {
        const want = expected(pages[i].name, pages[1 - i].name);
        return s.connected.length === 2 && want.every((w, ch) => w.every((m) => s.received[ch].includes(m)));
    });
    if (done) break;
    if (Date.now() > deadline) {
        writeFileSync(join(out, 'pages.json'), JSON.stringify(states, null, 2));
        writeFileSync(join(out, 'console.log'), consoleLines.join('\n'));
        fail(`peers incomplete: ${JSON.stringify(states)}`);
    }
    await new Promise((ok) => setTimeout(ok, 200));
}
const nativeIds = [...native.output().matchAll(/^([0-9a-f]{32}) connected$/gm)].map((m) => m[1]);
if (nativeIds.length !== 2) fail(`native peer saw ${nativeIds.length} connections:\n${native.output()}`);
writeFileSync(join(out, 'pages.json'), JSON.stringify(states, null, 2));
console.log('PASS: 2 browser peers and 1 native peer connected in full mesh; both channels delivered both ways');
cleanup();
process.exit(0);
