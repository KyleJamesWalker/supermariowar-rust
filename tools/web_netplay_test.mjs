#!/usr/bin/env node
// Browser netplay end to end: start smw_relay, open the web build (dist/web) in two headless Chromes,
// and play each tools/ref/<scenario>'s scripted net game through the real menus (connect, create room,
// join, start, play) under GGRS over WebRTC. Writes the pages' per-frame states, dumps, recordings,
// screenshots and logs to <out_dir>/<scenario>; every frame both players logged must be identical.
//
// Usage: node tools/web_netplay_test.mjs [out_dir]
//   CHROME     Chrome binary (default: the macOS Google Chrome app)
//   RELAY_BIN  smw_relay binary (default: target/release/smw_relay)
//   WEB_DIR    web build (default: dist/web)
//   NET_GAMES  scenarios (default: every tools/ref/net_game*)
//   MIXED=1    the joiner is the native build (SMW_BIN, default target/release/smw), which reaches the same
//              relay lobby through a ws:// server entry; MIXED=host makes the host native instead
import { spawn, spawnSync } from 'node:child_process';
import { createServer } from 'node:http';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createServer as createNetServer } from 'node:net';
import { tmpdir } from 'node:os';
import { dirname, extname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const web = resolve(process.env.WEB_DIR ?? join(repo, 'dist', 'web'));
const relayBin = resolve(process.env.RELAY_BIN ?? join(repo, 'target', 'release', 'smw_relay'));
const chrome = process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const out = resolve(process.argv[2] ?? join(tmpdir(), 'smw-web-netplay'));
mkdirSync(out, { recursive: true });

const MIXED = process.env.MIXED === '1' || process.env.MIXED === 'host';
const nativeRole = process.env.MIXED === 'host' ? 'host' : 'join';
const nativeBin = resolve(process.env.SMW_BIN ?? join(repo, 'target', 'release', 'smw'));
const games = (process.env.NET_GAMES ?? 'net_game net_game_blocks net_game_frenzy net_game_stomp net_game_coins net_game_classic net_game_stompkill net_game_stompkill_powerup').split(/\s+/).filter(Boolean);

// The native scripts assume both clients start together. Pages load at different speeds, so the
// host's start press and both players' gameplay inputs move SHIFT frames later.
const SHIFT = 600;
const scenario = (game) => {
    const dir = join(repo, 'tools', 'ref', game);
    const script = (role) => readFileSync(join(dir, `${role}.txt`), 'utf8');
    const param = (key) => script('host').match(new RegExp(`^#@ ${key}=(.*)$`, 'm'))?.[1];
    const frames = Number(param('frames')) + SHIFT;
    // An `await gameplay` before `from` already absorbs the delay for the lines after it.
    const shifted = (role, from) => {
        let awaited = false;
        return script(role)
            .split('\n')
            .map((line) => {
                if (line.startsWith('#@ frames=')) return `#@ frames=${frames}`;
                const m = line.match(/^(\d+)( .*)$/);
                if (m && Number(m[1]) < from && m[2] === ' await gameplay') awaited = true;
                return m && !awaited && Number(m[1]) >= from ? `${Number(m[1]) + SHIFT}${m[2]}` : line;
            })
            .join('\n');
    };
    const map = param('map');
    const netSeed = param('net_seed');
    const mapFile = map && { name: `${map}.map`, base64: readFileSync(join(repo, 'tools', 'ref', 'net_maps', `${map}.map`)).toString('base64') };
    return {
        frames,
        roles: [
            { role: 'host', name: 'Host', replay: shifted('host', 420), mapFile, map, netSeed },
            { role: 'join', name: 'Join', replay: shifted('join', 620), mapFile },
        ],
    };
};

const children = [];
const profiles = [];
let httpServer;
const cleanup = () => {
    for (const child of children) {
        try {
            process.kill(-child.pid, 'SIGKILL');
        } catch {
            child.kill('SIGKILL');
        }
    }
    httpServer?.close();
    for (const p of profiles) rmSync(p, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
};
process.on('exit', cleanup);
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => process.exit(1));
const SCENARIO_MS = 200000;
const DEADLINE_MS = (2 * SCENARIO_MS + 40000) * games.length;
setTimeout(() => {
    console.error(`no result after ${DEADLINE_MS / 1000} s; giving up`);
    process.exit(1);
}, DEADLINE_MS).unref();

const within = (promise, ms) =>
    Promise.race([promise, new Promise((_, fail) => setTimeout(() => fail(new Error(`no answer in ${ms / 1000} s`)), ms))]);

const freePort = () =>
    new Promise((ok) => {
        const s = createNetServer();
        s.listen(0, '127.0.0.1', () => {
            const { port } = s.address();
            s.close(() => ok(port));
        });
    });

const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.data': 'application/octet-stream' };
httpServer = createServer((req, res) => {
    const path = decodeURIComponent(req.url.split('?')[0]);
    const file = join(web, path === '/' ? 'index.html' : path);
    try {
        const body = readFileSync(file);
        res.writeHead(200, { 'Content-Type': types[extname(file)] ?? 'application/octet-stream' });
        res.end(body);
    } catch {
        res.writeHead(404);
        res.end();
    }
});
await new Promise((ok) => httpServer.listen(0, '127.0.0.1', ok));
const origin = `http://127.0.0.1:${httpServer.address().port}`;

const relayPort = await freePort();
const relayDir = join(out, 'relay');
mkdirSync(relayDir, { recursive: true });
let relayLog = '';
const relay = spawn(relayBin, ['--port', String(relayPort)], {
    cwd: relayDir,
    env: { ...process.env, SMW_RELAY_ALLOWED_ORIGINS: origin },
    stdio: ['ignore', 'pipe', 'pipe'],
});
children.push(relay);
relay.on('error', (e) => (relayLog += `${e.message}\n`));
relay.stdout.on('data', (d) => (relayLog += d));
relay.stderr.on('data', (d) => (relayLog += d));
for (let i = 0; i < 50 && !relayLog.includes('Ready!'); i++) await new Promise((ok) => setTimeout(ok, 100));
if (!relayLog.includes('Ready!')) {
    console.error(`smw_relay did not start:\n${relayLog}`);
    process.exit(1);
}
const health = await fetch(`http://127.0.0.1:${relayPort}/healthz`);
console.log(`relay on port ${relayPort}, /healthz ${health.status}`);

// Runs before smw.js (see web_replay.mjs): harness variables, the replay, and the player's name.
const initScript = ({ name, replay, frames, mapFile, map, env: extra }) => `(() => {
    const env = ${JSON.stringify({
        SMW_SEED: '1', SMW_FRAMES: String(frames), SMW_REPLAY: '/replay.txt', SMW_DUMP: '/dump.txt', ...(map && { SMW_MAP: map }), ...extra,
    })};
    const replay = ${JSON.stringify(replay)};
    const mapFile = ${JSON.stringify(mapFile ?? null)};
    const config = ${JSON.stringify(`player_name = "${name}"\nservers = []\n`)};
    let module;
    Object.defineProperty(window, 'Module', {
        configurable: true,
        get() { return module; },
        set(m) {
            if (m === module) return;
            module = m;
            m.noInitialRun = false;
            m.preRun = [...(m.preRun ?? []), () => {
                Object.assign(m.ENV, env);
                m.FS.writeFile('/replay.txt', replay);
                if (mapFile) {
                    m.FS.mkdirTree('/data/maps');
                    m.FS.writeFile('/data/maps/' + mapFile.name, Uint8Array.from(atob(mapFile.base64), (c) => c.charCodeAt(0)));
                }
                const home = (m.ENV.HOME ?? '/home/web_user') + '/Library/Preferences/.smw';
                m.FS.mkdirTree(home);
                m.FS.writeFile(home + '/servers.toml', config);
            }];
        },
    });
    window.alert = (msg) => console.error('[alert] ' + msg);
})();`;

const openPage = async (page) => {
    const { role } = page;
    const profile = mkdtempSync(join(tmpdir(), 'smw-netplay-chrome-'));
    profiles.push(profile);
    const browser = spawn(chrome, [
        '--headless=new', '--remote-debugging-port=0', `--user-data-dir=${profile}`, '--disable-extensions',
        '--autoplay-policy=no-user-gesture-required', '--mute-audio', '--window-size=800,700', 'about:blank',
    ], { stdio: ['ignore', 'ignore', 'pipe'], detached: true });
    children.push(browser);
    const close = () => {
        try {
            process.kill(-browser.pid, 'SIGKILL');
        } catch {}
    };
    const wsUrl = await new Promise((ok, fail) => {
        let buf = '';
        browser.stderr.on('data', (d) => {
            buf += d;
            const m = buf.match(/DevTools listening on (ws:\/\/\S+)/);
            if (m) ok(m[1]);
        });
        browser.on('exit', () => fail(new Error('chrome exited')));
    });
    const ws = new WebSocket(wsUrl);
    await new Promise((ok) => ws.addEventListener('open', ok));
    let nextId = 1;
    const pending = new Map();
    const consoleLines = [];
    ws.addEventListener('message', (ev) => {
        const msg = JSON.parse(ev.data);
        if (msg.id && pending.has(msg.id)) {
            const { ok, fail } = pending.get(msg.id);
            pending.delete(msg.id);
            msg.error ? fail(new Error(msg.error.message)) : ok(msg.result);
        } else if (msg.method === 'Runtime.consoleAPICalled') {
            consoleLines.push(msg.params.args.map((a) => a.value ?? a.description ?? '').join(' '));
        }
    });
    const send = (method, params = {}, sessionId) =>
        new Promise((ok, fail) => {
            const id = nextId++;
            pending.set(id, { ok, fail });
            ws.send(JSON.stringify({ id, method, params, sessionId }));
        });
    const { targetId } = await send('Target.createTarget', { url: 'about:blank' });
    const { sessionId } = await send('Target.attachToTarget', { targetId, flatten: true });
    await send('Runtime.enable', {}, sessionId);
    await send('Page.enable', {}, sessionId);
    await send('Page.addScriptToEvaluateOnNewDocument', { source: initScript(page) }, sessionId);
    const relayUrl = page.offline ? '' : `ws://127.0.0.1:${relayPort}`;
    await send('Page.navigate', { url: `${origin}/index.html?relay=${encodeURIComponent(relayUrl)}` }, sessionId);
    const evaluate = async (expression) =>
        (await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true }, sessionId)).result.value;
    const screenshot = async (file) => {
        const { data } = await send('Page.captureScreenshot', { format: 'png' }, sessionId);
        writeFileSync(file, Buffer.from(data, 'base64'));
    };
    return { role, evaluate, screenshot, consoleLines, close };
};

const progress = (page) =>
    page.evaluate(`(() => {
        try {
            const dump = Module.FS.readFile('/dump.txt', { encoding: 'utf8' });
            let done = false;
            try { done = /^#@ frames=/m.test(Module.FS.readFile('/rec.txt', { encoding: 'utf8' })); } catch {}
            return { frames: (dump.match(/^F /gm) || []).length, gameplay: (dump.match(/^F \\d+ gameplay/gm) || []).length, done };
        } catch { return { frames: 0, gameplay: 0, done: false }; }
    })()`);

const play = async (game) => {
    const { frames, roles } = scenario(game);
    const dir = join(out, game);
    mkdirSync(dir, { recursive: true });
    const logStart = relayLog.length;
    const pages = [];
    const v2 = { SMW_V2_STATE_LOG: '/v2state.log' };
    let native = null;
    for (const r of roles) {
        if (MIXED && r.role === nativeRole) {
            native = startNative(r, frames, dir);
            continue;
        }
        pages.push(await openPage({ ...r, frames, env: { SMW_RECORD_TO: '/rec.txt', ...(r.netSeed && { SMW_NET_SEED: r.netSeed }), ...v2 } }));
    }

    let shot = false;
    let timedOut = false;
    const started = Date.now();
    for (;;) {
        await new Promise((ok) => setTimeout(ok, 1000));
        const states = await Promise.all(pages.map((p) => within(progress(p), 10000).catch(() => ({ frames: -1, gameplay: 0 }))));
        if (!shot && states.every((s) => s.gameplay >= 300)) {
            await Promise.all(pages.map((p) => p.screenshot(join(dir, `${p.role}.png`))));
            shot = true;
        }
        // A script's await lines run a session past `frames`; it is over once its recording has `#@ frames=`.
        if (states.every((s) => s.done) && (!native || native.finished)) break;
        if (Date.now() - started > SCENARIO_MS) {
            console.error(`${game}: no result after ${SCENARIO_MS / 1000} s: ` + pages.map((p, i) => `${p.role} ${states[i].frames}/${frames} frames`).join(', '));
            timedOut = true;
            break;
        }
    }

    console.log(`== ${game}`);
    let ok = !timedOut;
    for (const page of pages) {
        const state = await within(page.evaluate(`(() => { try { return Module.FS.readFile('/v2state.log', { encoding: 'utf8' }); } catch { return ''; } })()`), 10000).catch(() => '');
        writeFileSync(join(dir, `${page.role}.v2state.log`), state);
    }
    for (const page of pages) {
        try {
            writeFileSync(join(dir, `${page.role}.dump`), await within(page.evaluate(`Module.FS.readFile('/dump.txt', { encoding: 'utf8' })`), 10000));
            writeFileSync(join(dir, `${page.role}.rec.txt`), await within(page.evaluate(`Module.FS.readFile('/rec.txt', { encoding: 'utf8' })`), 10000));
        } catch (e) {
            console.error(`${page.role}: could not read the dump or the recording: ${e.message}`);
            ok = false;
        }
        writeFileSync(join(dir, `${page.role}.log`), page.consoleLines.join('\n') + '\n');
    }
    writeFileSync(join(dir, 'relay.log'), relayLog.slice(logStart));
    if (!relayLog.slice(logStart).includes('starting.')) {
        console.error('the lobby never started a room');
        ok = false;
    }
    if (native) {
        if (!native.finished) native.child.kill('SIGKILL');
        writeFileSync(join(dir, `${native.role}.log`), native.consoleLines.join('\n') + '\n');
    }
    ok &&= compareV2(dir, native ? [...pages, native] : pages);
    for (const page of pages) page.close();
    console.log(`${game}: ${ok ? 'PASS' : 'FAIL'} (${dir})`);
    return ok;
};

// The joiner as a native process for MIXED: the same script, its server list holding just the relay, as the page's does.
const startNative = (r, frames, dir) => {
    const home = join(dir, 'home-native');
    const settings = join(home, 'Library', 'Preferences', '.smw');
    mkdirSync(settings, { recursive: true });
    writeFileSync(join(settings, 'servers.toml'), `player_name = "${r.name}"\nservers = ["ws://127.0.0.1:${relayPort}"]\n`);
    writeFileSync(join(dir, `${r.role}.native.txt`), r.replay);
    const data = nativeData();
    const child = spawn(nativeBin, ['--datadir', data], {
        cwd: dirname(data),
        env: {
            ...process.env, HOME: home, SDL_VIDEODRIVER: 'dummy', SDL_AUDIODRIVER: 'dummy', SMW_SEED: '1', SMW_FRAMES: String(frames),
            SMW_REPLAY: join(dir, `${r.role}.native.txt`), SMW_NO_RECORD: '1', SMW_V2_STATE_LOG: join(dir, `${r.role}.v2state.log`),
            ...(r.map && { SMW_MAP: r.map }), ...(r.netSeed && { SMW_NET_SEED: r.netSeed }),
        },
        stdio: ['ignore', 'pipe', 'pipe'],
        detached: true,
    });
    children.push(child);
    const native = { role: r.role, consoleLines: [], finished: false, child };
    const collect = (d) => native.consoleLines.push(...String(d).split('\n').filter(Boolean));
    child.stdout.on('data', collect);
    child.stderr.on('data', collect);
    child.on('exit', () => (native.finished = true));
    return native;
};

// Netplay v2: each page logs the state it checksums at every saved frame; both must match wherever both logged, both
// must have run a GGRS session, and neither may report a desync.
const compareV2 = (dir, pages) => {
    const frames = (text) => {
        const out = new Map();
        let cur = null;
        for (const line of text.split('\n')) {
            if (line.startsWith('#F ')) {
                const f = line.split(' ');
                cur = [f[2]];
                out.set(Number(f[1]), cur);
            } else if (cur) cur.push(line);
        }
        return out;
    };
    let ok = true;
    for (const page of pages) {
        if (!page.consoleLines.some((l) => l.includes('[net] v2: GGRS session started'))) {
            console.error(`${page.role}: no GGRS session`);
            ok = false;
        }
        const desync = page.consoleLines.find((l) => l.includes('[net] v2: desync'));
        if (desync) {
            console.error(`${page.role}: ${desync}`);
            ok = false;
        }
    }
    const [h, j] = pages.map((p) => frames(readFileSync(join(dir, `${p.role}.v2state.log`), 'utf8')));
    // A page's last frames may be predictions the session never confirmed before the match ended.
    const last = Math.min(Math.max(...h.keys()), Math.max(...j.keys())) - 8;
    const common = [...h.keys()].filter((f) => j.has(f) && f <= last).sort((a, b) => a - b);
    const differ = common.find((f) => h.get(f).join('\n') !== j.get(f).join('\n'));
    console.log(`  v2: ${common.length} frames compared${differ === undefined ? ', identical' : `, states differ from frame ${differ}`}`);
    return ok && differ === undefined && common.length >= 300;
};

// The data tree with tools/ref/net_maps added, for the native player.
let nativeDataDir;
const nativeData = () => {
    if (!nativeDataDir) {
        nativeDataDir = join(out, 'data');
        rmSync(nativeDataDir, { recursive: true, force: true });
        spawnSync('cp', ['-R', join(repo, 'data'), nativeDataDir]);
        spawnSync('sh', ['-c', 'cp "$0"/*.map "$1"/maps/', join(repo, 'tools', 'ref', 'net_maps'), nativeDataDir]);
    }
    return nativeDataDir;
};

let ok = true;
for (const game of games) if (!(await play(game))) ok = false;
writeFileSync(join(out, 'relay.log'), relayLog);
console.log(`${ok ? 'PASS' : 'FAIL'}: results in ${out}`);
process.exit(ok ? 0 : 1);
