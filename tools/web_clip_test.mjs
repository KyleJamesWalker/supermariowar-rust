#!/usr/bin/env node
// The web start screen's match list in headless Chrome: a session driven with key presses plays one
// Classic game and leaves it; after a reload the start screen lists that match, its clip and the
// session download as .smwrp (gzip) and as .txt, and on an emulated phone a tap on Watch plays the
// clip, whose dump must equal the native build's dump of the downloaded clip. Load replay plays the
// clip as .txt, .smwrp and .txt.gz. Exits non-zero on the first failed check. Screenshots go to out_dir.
//
// Usage: node tools/web_clip_test.mjs [out_dir]
//   CHROME   Chrome binary (default: the macOS Google Chrome app)
//   WEB_DIR  the web build to serve (default: dist/web, see tools/package_web.sh)
import { spawn, spawnSync } from 'node:child_process';
import { createServer } from 'node:http';
import { existsSync, mkdtempSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { gunzipSync, gzipSync } from 'node:zlib';
import { dirname, extname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const web = resolve(process.env.WEB_DIR ?? join(repo, 'dist', 'web'));
const chrome = process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const out = resolve(process.argv[2] ?? join(tmpdir(), 'smw-web-clip-out'));
mkdirSync(out, { recursive: true });
const downloads = join(out, 'downloads');
rmSync(downloads, { recursive: true, force: true });
mkdirSync(downloads);
let downloadCount = 0;

const types = {
    '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm',
    '.png': 'image/png', '.webmanifest': 'application/manifest+json', '.data': 'application/octet-stream',
};
const server = createServer((req, res) => {
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
await new Promise((ok) => server.listen(0, '127.0.0.1', ok));
const url = `http://127.0.0.1:${server.address().port}/`;

const profile = mkdtempSync(join(tmpdir(), 'smw-web-chrome-'));
const browser = spawn(chrome, [
    '--headless=new', '--remote-debugging-port=0', `--user-data-dir=${profile}`, '--disable-extensions',
    '--autoplay-policy=no-user-gesture-required', '--mute-audio', '--window-size=900,900', 'about:blank',
], { stdio: ['ignore', 'ignore', 'pipe'] });
const cleanup = () => {
    browser.kill('SIGKILL');
    server.close();
    rmSync(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
};
process.on('exit', cleanup);
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => process.exit(1));
const deadlineMs = 600000;
setTimeout(() => {
    console.error(`not finished after ${deadlineMs / 1000} s; giving up`);
    process.exit(1);
}, deadlineMs).unref();

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
        msg.error ? fail(new Error(`${msg.error.message}`)) : ok(msg.result);
    } else if (msg.method === 'Runtime.consoleAPICalled') {
        consoleLines.push(msg.params.args.map((a) => a.value ?? a.description ?? '').join(' '));
    }
});
let sessionId;
const send = (method, params = {}, session = sessionId) =>
    new Promise((ok, fail) => {
        const id = nextId++;
        pending.set(id, { ok, fail });
        ws.send(JSON.stringify({ id, method, params, sessionId: session }));
    });
const sleep = (ms) => new Promise((ok) => setTimeout(ok, ms));
const evaluate = async (expression) => {
    const r = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
    if (r.exceptionDetails) throw new Error(`${expression}: ${r.exceptionDetails.exception?.description ?? r.exceptionDetails.text}`);
    return r.result.value;
};
const waitFor = async (expression, what, ms = 60000) => {
    const until = Date.now() + ms;
    while (!(await evaluate(expression))) {
        if (Date.now() > until) throw new Error(`timed out waiting for ${what}`);
        await sleep(200);
    }
};
let failures = 0;
const check = (ok, what) => {
    console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}`);
    if (!ok) failures++;
};
let shot = 0;
const screenshot = async (name) => {
    const { data } = await send('Page.captureScreenshot', { format: 'png' });
    const file = join(out, `${String(++shot).padStart(2, '0')}-${name}.png`);
    writeFileSync(file, Buffer.from(data, 'base64'));
    console.log(`     ${file}`);
};
const center = (selector) => evaluate(`(() => {
    const r = document.querySelector(${JSON.stringify(selector)}).getBoundingClientRect();
    return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
})()`);
const tap = async (p) => {
    await send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ x: p.x, y: p.y, id: 1 }] });
    await sleep(100);
    await send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
};
// Clicks with `expression` and returns the one file that lands in a fresh download directory.
const download = async (expression) => {
    const dir = join(downloads, String(++downloadCount));
    mkdirSync(dir);
    await send('Browser.setDownloadBehavior', { behavior: 'allow', downloadPath: dir }, undefined);
    await evaluate(expression);
    const until = Date.now() + 10000;
    let name;
    while (!(name = readdirSync(dir).find((f) => !f.endsWith('.crdownload'))) && Date.now() < until) await sleep(200);
    if (!name) throw new Error(`no download from ${expression}`);
    await sleep(300);
    return { name, bytes: readFileSync(join(dir, name)) };
};
const isGzip = (b) => b[0] === 0x1f && b[1] === 0x8b;
const loadReplay = async (file, what) => {
    await send('Page.reload');
    await waitFor(startScreen, `the start screen for Load replay (${what})`, 120000);
    const { result: input } = await send('Runtime.evaluate', { expression: `document.getElementById('loadReplay')` });
    await send('DOM.setFileInputFiles', { files: [file], objectId: input.objectId });
    await waitFor('window.smwQuit === true', `the loaded ${what} to play to its end`, 90000);
    return evaluate(`Module.FS.readFile('/dump.txt', { encoding: 'utf8' })`);
};
const recording = () => evaluate(`(() => {
    const names = Module.FS.readdir(REPLAY_DIR).filter((n) => n.endsWith('.txt')).sort();
    return names.length ? Module.FS.readFile(REPLAY_DIR + '/' + names[names.length - 1], { encoding: 'utf8' }) : '';
})()`);
const startScreen = `document.getElementById('start')?.hidden === false`;
const KEYS = { Enter: 13, Escape: 27, ArrowLeft: 37, ArrowUp: 38, ArrowRight: 39, e: 69 };
const key = async (name, holdMs = 80) => {
    const code = name.length === 1 ? `Key${name.toUpperCase()}` : name;
    for (const type of ['keyDown', 'keyUp']) {
        await send('Input.dispatchKeyEvent', { type, key: name, code, windowsVirtualKeyCode: KEYS[name] });
        if (type === 'keyDown') await sleep(holdMs);
    }
    await sleep(250);
};
// The state and menu of the newest dump block (docs/REPLAY.md, "Dump format").
const frameState = async () => {
    const dump = await evaluate(`(() => { try { return Module.FS.readFile('/dump.txt', { encoding: 'utf8' }).slice(-4000); } catch { return ''; } })()`);
    const block = dump.slice(dump.lastIndexOf('\nF ') + 1).split('\n');
    return { state: (block[0] ?? '').split(' ')[2], menu: (block.find((l) => l.startsWith('M ')) ?? '').split(' ')[1] };
};

const { targetId } = await send('Target.createTarget', { url: 'about:blank' });
({ sessionId } = await send('Target.attachToTarget', { targetId, flatten: true }));
await send('Runtime.enable');
await send('Page.enable');
await send('Browser.setDownloadBehavior', { behavior: 'allow', downloadPath: downloads }, undefined);
await send('Page.addScriptToEvaluateOnNewDocument', {
    source: `window.alert = (msg) => console.error('[alert] ' + msg);
        let module;
        Object.defineProperty(window, 'Module', {
            configurable: true,
            get() { return module; },
            set(m) {
                if (m === module) return;
                module = m;
                m.preRun = [...(m.preRun ?? []), () => Object.assign(m.ENV, { SMW_NOLIMIT: '1', SMW_DUMP: '/dump.txt' })];
                const quit = m.onGameQuit;
                m.onGameQuit = () => { window.smwQuit = true; quit && quit(); };
            },
        });`,
});

try {
    await send('Emulation.setDeviceMetricsOverride', { width: 1280, height: 800, deviceScaleFactor: 1, mobile: false });
    await send('Page.navigate', { url });
    await waitFor(startScreen, 'the start screen', 120000);
    check(await evaluate(`document.getElementById('matches').hidden && document.getElementById('watchLast').hidden`), 'no recording yet: no match list');

    await evaluate(`document.getElementById('play').click()`);
    await evaluate(`document.getElementById('canvas').focus()`);
    await sleep(1500);
    // tools/replays/start_classic.txt's menu path: Enter through the menus, E readies player 2.
    for (let i = 0; i < 12 && (await frameState()).state !== 'gameplay'; i++) {
        await key((await frameState()).menu === 'team_select' && i < 6 ? 'e' : 'Enter');
    }
    check((await frameState()).state === 'gameplay', 'the keyboard started a Classic game');
    await sleep(1500);
    await key('ArrowRight', 600);
    await key('ArrowUp');
    await key('Escape');
    await key('ArrowLeft');
    await key('Enter');
    await waitFor(`(() => { try { return /^#@ mark frame=\\d+ state=menu match=1 /m.test(Module.FS.readFile(REPLAY_DIR + '/' + Module.FS.readdir(REPLAY_DIR).filter((n) => n.endsWith('.txt')).sort().pop(), { encoding: 'utf8' })); } catch { return false; } })()`,
        'the game to end through the exit dialog', 20000);
    const session = await recording();
    check(/^#@ checkpoint match=1 frame=\d+ z64=/m.test(session), 'the browser recording has a checkpoint for match 1');
    const footerGz = await download(`document.querySelector('#downloadReplay a:not([data-plain])').click()`);
    const footerTxt = await download(`document.querySelector('#downloadReplay a[data-plain]').click()`);
    check(/\.smwrp$/.test(footerGz.name) && isGzip(footerGz.bytes) && /\.txt$/.test(footerTxt.name) && !isGzip(footerTxt.bytes)
        && gunzipSync(footerGz.bytes).equals(footerTxt.bytes) && footerTxt.bytes.toString().startsWith('#'),
        `the footer downloads the recording as .smwrp and (.txt) as text (${footerGz.name} ${footerGz.bytes.length} B, ${footerTxt.name} ${footerTxt.bytes.length} B)`);
    await evaluate(`new Promise((ok) => Module.FS.syncfs(false, ok))`);

    await send('Page.reload');
    await waitFor(startScreen, 'the start screen after the session', 120000);
    const rows = await evaluate(`[...document.querySelectorAll('#matchList .match-name')].map((e) => e.textContent)`);
    check(rows.length === 1 && /^Match 1 — Classic, 0smw, 2 players, \d+ s$/.test(rows[0]), `the start screen lists the match (${rows.join(' | ')})`);
    check(await evaluate(`document.getElementById('watchLast').hidden && !document.getElementById('downloadLast').hidden
        && document.getElementById('downloadLast').textContent === 'Download session'`), 'Watch last game gives way to the list; the session still downloads');
    await screenshot('desktop-list');

    const clickClip = `[...document.querySelectorAll('#matchList button')].find((b) => b.textContent === 'Clip').click()`;
    const clickSession = `document.getElementById('downloadLast').click()`;
    check(await evaluate(`!document.getElementById('saveTextOption').hidden && !document.getElementById('saveText').checked`), 'the "save downloads as .txt" option shows, unchecked');
    const clipGz = await download(clickClip);
    const sessionGz = await download(clickSession);
    await evaluate(`document.getElementById('saveText').click()`);
    const clipTxt = await download(clickClip);
    const sessionTxt = await download(clickSession);
    await evaluate(`document.getElementById('saveText').click()`);
    const clip = clipTxt.bytes.toString('utf8');
    check(/_match1\.smwrp$/.test(clipGz.name) && isGzip(clipGz.bytes) && gunzipSync(clipGz.bytes).equals(clipTxt.bytes),
        `Clip downloads a .smwrp that gunzips to the .txt download (${clipGz.name})`);
    check(/_match1\.txt$/.test(clipTxt.name) && /^#@ segment=1$/m.test(clip) && /^#@ checkpoint match=1 /m.test(clip), `with the option, Clip downloads a text clip (${clipTxt.name})`);
    check(/\.smwrp$/.test(sessionGz.name) && isGzip(sessionGz.bytes) && /\.txt$/.test(sessionTxt.name) && gunzipSync(sessionGz.bytes).equals(sessionTxt.bytes),
        `Download session gives .smwrp, or .txt with the option (${sessionGz.name}, ${sessionTxt.name})`);
    for (const [what, gz, txt] of [['clip', clipGz, clipTxt], ['session', sessionGz, sessionTxt]]) {
        console.log(`     ${what}: ${txt.bytes.length} B as .txt, ${gz.bytes.length} B as .smwrp (${(100 * gz.bytes.length / txt.bytes.length).toFixed(0)}%)`);
    }

    // A phone: the list fits the start screen and a tap on Watch plays the clip.
    await send('Emulation.setDeviceMetricsOverride', {
        width: 844, height: 390, deviceScaleFactor: 3, mobile: true, screenOrientation: { type: 'landscapePrimary', angle: 90 },
    });
    await send('Emulation.setTouchEmulationEnabled', { enabled: true, maxTouchPoints: 5 });
    await send('Page.reload');
    await waitFor(startScreen, 'the start screen on a phone', 120000);
    check(await evaluate(`document.documentElement.classList.contains('touch')`), 'phone: touch controls on');
    check(await evaluate(`(() => {
        const s = document.getElementById('stage').getBoundingClientRect(), r = document.getElementById('saveText').getBoundingClientRect();
        return r.height > 0 && r.top >= s.top && r.bottom <= s.bottom;
    })()`), 'phone: the .txt option fits on the start screen');
    check(await evaluate(`(() => {
        const s = document.getElementById('stage').getBoundingClientRect();
        return [...document.querySelectorAll('#matchList button')].every((b) => {
            const r = b.getBoundingClientRect();
            return r.height >= 30 && r.top >= s.top && r.bottom <= s.bottom && r.left >= s.left && r.right <= s.right;
        });
    })()`), 'phone: the Watch and Clip buttons fit on the start screen');
    await screenshot('phone-list');
    await tap(await center('#matchList button'));
    await waitFor('window.smwQuit === true', 'the clip to play to its end', 60000);
    await waitFor(`!document.getElementById('end').hidden`, 'the end screen', 5000);
    check(await evaluate(`document.getElementById('endText').textContent === 'Replay finished'`), 'tapping Watch played the clip');
    await screenshot('phone-watched');

    const webDump = await evaluate(`Module.FS.readFile('/dump.txt', { encoding: 'utf8' })`);
    writeFileSync(join(out, 'clip.txt'), clip);
    writeFileSync(join(out, 'web_dump.txt'), webDump);
    const native = spawnSync(join(repo, 'tools', 'run_rust.sh'), [join(out, 'clip.txt'), join(out, 'native')], { encoding: 'utf8' });
    const nativeDump = native.status === 0 && existsSync(join(out, 'native', 'dump.txt')) ? readFileSync(join(out, 'native', 'dump.txt'), 'utf8') : null;
    const start = Number(clip.match(/^#@ checkpoint match=1 frame=(\d+)/m)[1]);
    check(nativeDump !== null && webDump === nativeDump && webDump.startsWith(`F ${start} gameplay\n`),
        `the watched clip's dump equals the native build's (${webDump.split('\nF ').length} frames from ${start})`);

    // Load replay takes the clip as text, as .smwrp and as .txt.gz (gzip told by its magic bytes).
    const clipSmwrp = join(out, 'clip.smwrp');
    const clipTxtGz = join(out, 'clip.txt.gz');
    writeFileSync(clipSmwrp, clipGz.bytes);
    writeFileSync(clipTxtGz, gzipSync(clipTxt.bytes));
    check(await evaluate(`document.getElementById('loadReplay').accept.split(',').includes('.smwrp')`), 'the file picker offers .smwrp');
    for (const [file, what] of [[join(out, 'clip.txt'), '.txt clip'], [clipSmwrp, '.smwrp clip'], [clipTxtGz, '.txt.gz clip']]) {
        const dump = await loadReplay(file, what);
        check(dump === nativeDump, `Load replay plays the ${what} like the native build (${dump.split('\nF ').length} frames)`);
    }
    // The game itself gunzips too (Emscripten's zlib), as smw --replay does natively.
    await send('Page.reload');
    await waitFor(startScreen, 'the start screen for a gzipped --replay', 120000);
    await evaluate(`(() => {
        Module.FS.writeFile('/tmp/raw.smwrp', Uint8Array.from(atob(${JSON.stringify(clipGz.bytes.toString('base64'))}), (c) => c.charCodeAt(0)));
        watch('/tmp/raw.smwrp');
    })()`);
    await waitFor('window.smwQuit === true', 'the gzipped --replay to play to its end', 90000);
    check(await evaluate(`Module.FS.readFile('/dump.txt', { encoding: 'utf8' })`) === nativeDump, 'the web build plays a gzipped --replay file like the native build');

    // A clip of an older recording (`b64=` checkpoints) still plays through Load replay.
    const oldClip = join(out, 'old_clip.txt');
    const cut = spawnSync('python3', [join(repo, 'tools', 'replay_clip.py'), join(repo, 'tools', 'checkpoint_fixtures', 'web_gamepad_ztar_b64.txt'), '--match', '1', '-o', oldClip], { encoding: 'utf8' });
    check(cut.status === 0 && /^#@ checkpoint match=1 frame=\d+ b64=/m.test(readFileSync(oldClip, 'utf8')), 'replay_clip.py cuts a b64 clip');
    const oldWeb = await loadReplay(oldClip, 'b64 clip');
    const oldNative = spawnSync(join(repo, 'tools', 'run_rust.sh'), [oldClip, join(out, 'old_native')], { encoding: 'utf8' });
    const oldNativeDump = oldNative.status === 0 ? readFileSync(join(out, 'old_native', 'dump.txt'), 'utf8') : null;
    check(oldNativeDump !== null && oldWeb === oldNativeDump, `Load replay plays a b64 clip like the native build (${oldWeb.split('\nF ').length} frames)`);
} catch (e) {
    console.error(e.message);
    console.error(`last console lines:\n${consoleLines.slice(-15).join('\n')}`);
    console.error(JSON.stringify((await recording().catch((x) => x.message)).slice(0, 1500)));
    failures++;
    await screenshot('error').catch(() => {});
}

console.log(failures ? `${failures} check(s) failed` : 'all checks passed');
process.exit(failures ? 1 : 0);
