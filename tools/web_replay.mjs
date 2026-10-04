#!/usr/bin/env node
// Run the web build (dist/web, see tools/package_web.sh) against a replay script in headless
// Chrome, like run_ref.sh runs a native binary: same #@ directives and SMW_* variables, and the
// same outputs (<out_dir>/dump.txt, frame_<n>.bmp), so diffreplay.py can compare them.
//
// Usage: node tools/web_replay.mjs <replay.txt> [out_dir]
//   CHROME   Chrome binary (default: the macOS Google Chrome app)
//   WEB_DIR / WEB_PAGE  another web build and its page (default: dist/web, index.html). It must be
//            linked with -sEXPORTED_RUNTIME_METHODS=ENV,FS.
//   SMW_SEED / SMW_FRAMES / SMW_MAP / SMW_SHOT_FRAMES override the replay's directives.
//   SMW_RLE and SMW_SEGMENT are passed through to the game.
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, extname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const web = resolve(process.env.WEB_DIR ?? join(repo, 'dist', 'web'));
const page = process.env.WEB_PAGE ?? 'index.html';
const chrome = process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';

const [replayArg, outArg] = process.argv.slice(2);
if (!replayArg) {
    console.error('usage: web_replay.mjs <replay.txt> [out_dir]');
    process.exit(2);
}
const replay = readFileSync(replayArg, 'utf8');
const out = resolve(outArg ?? join(tmpdir(), 'smw-web-replay-out', basename(replayArg, '.txt')));
mkdirSync(out, { recursive: true });

const directive = (key) => {
    const lines = replay.split('\n').filter((l) => l.startsWith('#@')).map((l) => l.slice(2).trim());
    const hit = lines.reverse().find((l) => l.startsWith(`${key}=`));
    return hit ? hit.slice(key.length + 1) : '';
};
const env = {
    SMW_SEED: process.env.SMW_SEED || directive('seed') || '1',
    SMW_NOLIMIT: '1',
    SMW_FRAMES: process.env.SMW_FRAMES || directive('frames') || '600',
    SMW_MAP: process.env.SMW_MAP ?? directive('map'),
    SMW_REPLAY: '/replay.txt',
    SMW_DUMP: '/dump.txt',
    SMW_SHOT_FRAMES: process.env.SMW_SHOT_FRAMES ?? directive('shots'),
    SMW_SHOT_DIR: '/shots',
    SMW_RLE: process.env.SMW_RLE ?? '',
    SMW_SEGMENT: process.env.SMW_SEGMENT ?? '',
};
if (directive('options')) {
    console.error('replays with an options= file are not supported in the browser yet');
    process.exit(2);
}
// Session recordings and clips embed their settings files; the game reads them from $HOME, kept apart
// from the page's IndexedDB-backed settings directory.
const settings = {};
for (const [key, file] of [['options_b64', 'options.bin'], ['controls_b64', 'controls.sdl2.bin']]) {
    if (directive(key)) settings[file] = directive(key);
}
if (Object.keys(settings).length) env.HOME = '/replay-home';
const frames = Number(env.SMW_FRAMES);

// Runs before smw.js: the shell page assigns `var Module = {...}`, so trap the assignment and add a
// preRun that sets the harness variables and writes the replay into the in-memory file system.
const initScript = `(() => {
    const env = ${JSON.stringify(env)};
    const replay = ${JSON.stringify(replay)};
    const settings = ${JSON.stringify(settings)};
    let module;
    Object.defineProperty(window, 'Module', {
        configurable: true,
        get() { return module; },
        set(m) {
            // smw.js assigns Module again (to itself); hook it only once.
            if (m === module) return;
            module = m;
            // The page waits for a Play click before main(); replays start straight away.
            m.noInitialRun = false;
            // The game quits after SMW_FRAMES, or at the end of a segment.
            const quit = m.onGameQuit;
            m.onGameQuit = () => { window.smwQuit = true; quit && quit(); };
            m.onAbort = () => { window.smwAbort = true; };
            m.preRun = [...(m.preRun ?? []), () => {
                Object.assign(m.ENV, env);
                m.FS.writeFile('/replay.txt', replay);
                m.FS.mkdir('/shots');
                for (const [file, b64] of Object.entries(settings)) {
                    m.FS.mkdirTree('/replay-home/Library/Preferences/.smw');
                    m.FS.writeFile('/replay-home/Library/Preferences/.smw/' + file, Uint8Array.from(atob(b64), (c) => c.charCodeAt(0)));
                }
            }];
        },
    });
    window.alert = (msg) => console.error('[alert] ' + msg);
    // Physical pads would join the game's joysticks and change who controls each player.
    navigator.getGamepads = () => [];
})();`;

const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.data': 'application/octet-stream' };
const server = createServer((req, res) => {
    const file = join(web, req.url === '/' ? page : decodeURIComponent(req.url.split('?')[0]));
    try {
        const body = readFileSync(file);
        res.writeHead(200, { 'Content-Type': types[extname(file)] ?? 'application/octet-stream' });
        res.end(body);
    } catch {
        res.writeHead(404);
        res.end();
    }
});
await new Promise((ok) => server.listen(0, '127.0.0.1', ok));
const url = `http://127.0.0.1:${server.address().port}/${page}`;

const profile = mkdtempSync(join(tmpdir(), 'smw-web-chrome-'));
const browser = spawn(chrome, [
    '--headless=new', '--remote-debugging-port=0', `--user-data-dir=${profile}`, '--disable-extensions',
    '--autoplay-policy=no-user-gesture-required', '--mute-audio', '--window-size=800,700', 'about:blank',
], { stdio: ['ignore', 'ignore', 'pipe'] });
const cleanup = () => {
    browser.kill();
    server.close();
    rmSync(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
};
process.on('exit', cleanup);
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => process.exit(1));
// A hang anywhere (Chrome, the page, a CDP call) must not leave the browser running.
const deadlineMs = Math.max(120000, frames * 100) + 60000;
setTimeout(() => {
    console.error(`no result after ${deadlineMs / 1000} s; giving up`);
    process.exit(1);
}, deadlineMs);

const wsUrl = await new Promise((ok, fail) => {
    let buf = '';
    browser.stderr.on('data', (d) => {
        buf += d;
        const m = buf.match(/DevTools listening on (ws:\/\/\S+)/);
        if (m) ok(m[1]);
    });
    browser.on('exit', () => fail(new Error('chrome exited')));
});

// Minimal Chrome DevTools Protocol client over the browser websocket.
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
await send('Page.addScriptToEvaluateOnNewDocument', { source: initScript }, sessionId);
await send('Page.navigate', { url }, sessionId);

const evaluate = async (expression) =>
    (await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true }, sessionId)).result.value;

// The harness closes the dump after SMW_FRAMES frames; the browser main loop keeps running.
const started = Date.now();
let done = false;
while (!done) {
    await new Promise((ok) => setTimeout(ok, 1000));
    done = await evaluate(`(() => {
        try {
            if (window.smwQuit) return true;
            if (window.smwAbort) return 'abort';
            const n = (Module.FS.readFile('/dump.txt', { encoding: 'utf8' }).match(/^F /gm) || []).length;
            return n >= ${frames};
        } catch { return false; }
    })()`);
    if (done === 'abort') {
        console.error(`the game aborted; last console lines:\n${consoleLines.slice(-20).join('\n')}`);
        process.exit(1);
    }
    if (Date.now() - started > Math.max(120000, frames * 100)) {
        console.error(`timed out; last console lines:\n${consoleLines.slice(-20).join('\n')}`);
        process.exit(1);
    }
}

writeFileSync(join(out, 'dump.txt'), await evaluate(`Module.FS.readFile('/dump.txt', { encoding: 'utf8' })`));
const shots = await evaluate(`Module.FS.readdir('/shots').filter((f) => f.endsWith('.bmp')).map((f) => {
    const bytes = Module.FS.readFile('/shots/' + f);
    let bin = '';
    for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode.apply(null, bytes.subarray(i, i + 0x8000));
    return [f, btoa(bin)];
})`);
for (const [name, b64] of shots) writeFileSync(join(out, name), Buffer.from(b64, 'base64'));
writeFileSync(join(out, 'stdout.log'), consoleLines.join('\n') + '\n');

console.log(out);
process.exit(0);
