#!/usr/bin/env node
// Browser netplay end to end: start smw_relay, open the web build (dist/web) in two headless Chromes,
// and play each tools/ref/<scenario>'s scripted net game through the real menus (connect, create room,
// join, start, play). Writes both harness dumps, page screenshots and logs to <out_dir>/<scenario>, then
// compares the players' tracks and spawned powerups with net_game_compare.py, as net_game_interop.sh
// does natively (including the scenario's `#@ map=` and `#@ compare=` parameters).
//
// Usage: node tools/web_netplay_test.mjs [out_dir]
//   CHROME     Chrome binary (default: the macOS Google Chrome app)
//   RELAY_BIN  smw_relay binary (default: relay/target/release/smw_relay)
//   WEB_DIR    web build (default: dist/web)
//   NET_GAMES  scenarios (default: every tools/ref/net_game*)
import { spawn, spawnSync } from 'node:child_process';
import { createServer } from 'node:http';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createServer as createNetServer } from 'node:net';
import { tmpdir } from 'node:os';
import { dirname, extname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const web = resolve(process.env.WEB_DIR ?? join(repo, 'dist', 'web'));
const relayBin = resolve(process.env.RELAY_BIN ?? join(repo, 'relay', 'target', 'release', 'smw_relay'));
const chrome = process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const out = resolve(process.argv[2] ?? join(tmpdir(), 'smw-web-netplay'));
mkdirSync(out, { recursive: true });

const games = (process.env.NET_GAMES ?? 'net_game net_game_blocks net_game_frenzy net_game_stomp net_game_coins net_game_classic').split(/\s+/).filter(Boolean);

// The native scripts assume both clients start together. Pages load at different speeds, so the
// host's start press and both players' gameplay inputs move SHIFT frames later.
const SHIFT = 600;
const scenario = (game) => {
    const dir = join(repo, 'tools', 'ref', game);
    const script = (role) => readFileSync(join(dir, `${role}.txt`), 'utf8');
    const param = (key) => script('host').match(new RegExp(`^#@ ${key}=(.*)$`, 'm'))?.[1];
    const frames = Number(param('frames')) + SHIFT;
    const shifted = (role, from) =>
        script(role)
            .split('\n')
            .map((line) => {
                if (line.startsWith('#@ frames=')) return `#@ frames=${frames}`;
                const m = line.match(/^(\d+)( .*)$/);
                return m && Number(m[1]) >= from ? `${Number(m[1]) + SHIFT}${m[2]}` : line;
            })
            .join('\n');
    const map = param('map');
    const mapFile = map && { name: `${map}.map`, base64: readFileSync(join(repo, 'tools', 'ref', 'net_maps', `${map}.map`)).toString('base64') };
    return {
        frames,
        compareArgs: (param('compare') ?? '').split(/\s+/).filter(Boolean),
        roles: [
            { role: 'host', name: 'Host', replay: shifted('host', 420), mapFile, map },
            { role: 'join', name: 'Join', replay: shifted('join', 620), mapFile },
        ],
    };
};

const children = [];
const profiles = [];
let httpServer;
const cleanup = () => {
    for (const child of children) child.kill('SIGKILL');
    httpServer?.close();
    for (const p of profiles) rmSync(p, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
};
process.on('exit', cleanup);
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => process.exit(1));
const DEADLINE_MS = 240000 * games.length;
setTimeout(() => {
    console.error(`no result after ${DEADLINE_MS / 1000} s; giving up`);
    process.exit(1);
}, DEADLINE_MS).unref();

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
const initScript = ({ name, replay, frames, mapFile, map }) => `(() => {
    const env = ${JSON.stringify({
        SMW_SEED: '1', SMW_FRAMES: String(frames), SMW_REPLAY: '/replay.txt', SMW_DUMP: '/dump.txt', ...(map && { SMW_MAP: map }),
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
    ], { stdio: ['ignore', 'ignore', 'pipe'] });
    children.push(browser);
    const close = () => browser.kill('SIGKILL');
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
    await send('Page.navigate', { url: `${origin}/index.html?relay=${encodeURIComponent(`ws://127.0.0.1:${relayPort}`)}` }, sessionId);
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
            return { frames: (dump.match(/^F /gm) || []).length, gameplay: (dump.match(/^F \\d+ gameplay/gm) || []).length };
        } catch { return { frames: 0, gameplay: 0 }; }
    })()`);

const play = async (game) => {
    const { frames, compareArgs, roles } = scenario(game);
    const dir = join(out, game);
    mkdirSync(dir, { recursive: true });
    const logStart = relayLog.length;
    const pages = [];
    for (const r of roles) pages.push(await openPage({ ...r, frames }));

    let shot = false;
    for (;;) {
        await new Promise((ok) => setTimeout(ok, 1000));
        const states = await Promise.all(pages.map(progress));
        if (!shot && states.every((s) => s.gameplay >= 300)) {
            await Promise.all(pages.map((p) => p.screenshot(join(dir, `${p.role}.png`))));
            shot = true;
        }
        if (states.every((s) => s.frames >= frames)) break;
    }

    let ok = true;
    for (const page of pages) {
        writeFileSync(join(dir, `${page.role}.dump`), await page.evaluate(`Module.FS.readFile('/dump.txt', { encoding: 'utf8' })`));
        writeFileSync(join(dir, `${page.role}.log`), page.consoleLines.join('\n') + '\n');
        page.close();
    }
    console.log(`== ${game}`);
    if (!shot) {
        console.error('a page never reached 300 gameplay frames');
        ok = false;
    }
    if (!relayLog.slice(logStart).includes('starting.')) {
        console.error('the lobby never started a room');
        ok = false;
    }
    const compare = spawnSync(
        'python3',
        [join(repo, 'tools', 'ref', 'net_game_compare.py'), '--spawns', ...compareArgs, join(dir, 'host.dump'), join(dir, 'join.dump')],
        { encoding: 'utf8' },
    );
    process.stdout.write(compare.stdout + compare.stderr);
    return ok && compare.status === 0;
};

let ok = true;
for (const game of games) if (!(await play(game))) ok = false;
writeFileSync(join(out, 'relay.log'), relayLog);
console.log(`${ok ? 'PASS' : 'FAIL'}: results in ${out}`);
process.exit(ok ? 0 : 1);
