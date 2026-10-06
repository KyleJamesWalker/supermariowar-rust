#!/usr/bin/env node
// Drive the web build's touch controls in headless Chrome: an emulated phone in landscape taps Play,
// walks the menus with the on-screen Start and D-pad, starts a match, then holds right + run + jump
// with simultaneous touches, switches to the floating stick and drags it; then checks portrait and desktop layouts and
// the on/off toggle.
// Exits non-zero on the first failed check. Screenshots go to out_dir.
//
// Usage: node tools/web_touch_test.mjs [out_dir]
//   CHROME   Chrome binary (default: the macOS Google Chrome app)
//   WEB_DIR  the web build to serve (default: dist/web, see tools/package_web.sh)
//   SMW_MAP  the match's map (default: 2skyfight, open floor right of player 1's spawn with SMW_SEED=1)
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, extname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const web = resolve(process.env.WEB_DIR ?? join(repo, 'dist', 'web'));
const chrome = process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const out = resolve(process.argv[2] ?? join(tmpdir(), 'smw-web-touch-out'));
mkdirSync(out, { recursive: true });

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
const deadlineMs = 240000;
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
ws.addEventListener('message', (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.id && pending.has(msg.id)) {
        const { ok, fail } = pending.get(msg.id);
        pending.delete(msg.id);
        msg.error ? fail(new Error(`${msg.error.message}`)) : ok(msg.result);
    }
});
let sessionId;
const send = (method, params = {}) =>
    new Promise((ok, fail) => {
        const id = nextId++;
        pending.set(id, { ok, fail });
        ws.send(JSON.stringify({ id, method, params, sessionId }));
    });
const MAP = process.env.SMW_MAP || '2skyfight';
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

const phone = async (landscape) => {
    await send('Emulation.setDeviceMetricsOverride', {
        width: landscape ? 844 : 390, height: landscape ? 390 : 844, deviceScaleFactor: 3, mobile: true,
        screenOrientation: landscape ? { type: 'landscapePrimary', angle: 90 } : { type: 'portraitPrimary', angle: 0 },
    });
    await send('Emulation.setTouchEmulationEnabled', { enabled: true, maxTouchPoints: 5 });
};
const desktop = async () => {
    await send('Emulation.setTouchEmulationEnabled', { enabled: false });
    await send('Emulation.setDeviceMetricsOverride', { width: 1280, height: 800, deviceScaleFactor: 1, mobile: false });
};

// Multi-touch: every event lists all fingers still down, as a real touch screen reports them.
const fingers = new Map();
const touch = async (type, id, point) => {
    if (type === 'touchEnd') fingers.delete(id);
    else fingers.set(id, { x: point.x, y: point.y, id });
    await send('Input.dispatchTouchEvent', { type, touchPoints: [...fingers.values()] });
};
const down = (id, p) => touch('touchStart', id, p);
const move = (id, p) => touch('touchMove', id, p);
const up = (id) => touch('touchEnd', id);
const tap = async (p, holdMs = 120) => {
    await down(9, p);
    await sleep(holdMs);
    await up(9);
};
const center = (selector, fx = 0.5, fy = 0.5) => evaluate(`(() => {
    const r = document.querySelector(${JSON.stringify(selector)}).getBoundingClientRect();
    return { x: r.left + r.width * ${fx}, y: r.top + r.height * ${fy} };
})()`);
const keyLog = () => evaluate('window.__keys.splice(0)');
const recording = () => evaluate(`(() => {
    const names = Module.FS.readdir(REPLAY_DIR).filter((n) => n.endsWith('.txt')).sort();
    return names.length ? Module.FS.readFile(REPLAY_DIR + '/' + names[names.length - 1], { encoding: 'utf8' }) : '';
})()`);
// The harness dump (docs/REPLAY.md) of the newest frame: state, menu focus and player records.
const frameState = async () => {
    const dump = await evaluate(`(() => { try { return Module.FS.readFile('/dump.txt', { encoding: 'utf8' }).slice(-4000); } catch { return ''; } })()`);
    const block = dump.slice(dump.lastIndexOf('\nF ') + 1).split('\n');
    const [, frame, state] = (block[0] ?? '').split(' ');
    const menu = block.find((l) => l.startsWith('M ')) ?? '';
    const p1 = Object.fromEntries((block.find((l) => l.startsWith('P id=0 ')) ?? '').split(' ').slice(1).map((kv) => kv.split('=')));
    return { frame: Number(frame), state, menu, p1: { x: Number(p1.fx), y: Number(p1.fy), velx: Number(p1.velx), vely: Number(p1.vely) } };
};
// Player 1's record in every gameplay frame after `since`.
const p1Since = async (since) => {
    const dump = await evaluate(`(() => { try { return Module.FS.readFile('/dump.txt', { encoding: 'utf8' }).slice(-300000); } catch { return ''; } })()`);
    const frames = [];
    let frame = -1;
    for (const line of dump.split('\n')) {
        if (line.startsWith('F ')) frame = Number(line.split(' ')[1]);
        else if (frame > since && line.startsWith('P id=0 ')) {
            const p = Object.fromEntries(line.split(' ').slice(1).map((kv) => kv.split('=')));
            frames.push({ frame, x: Number(p.fx), y: Number(p.fy), velx: Number(p.velx), vely: Number(p.vely), inair: p.inair === '1' });
        }
    }
    return frames;
};
const waitP1 = async (since, pred, what, ms = 3000) => {
    const until = Date.now() + ms;
    for (;;) {
        const hit = (await p1Since(since)).find(pred);
        if (hit) return hit;
        if (Date.now() > until) throw new Error(`timed out waiting for ${what}`);
        await sleep(50);
    }
};

const { targetId } = await send('Target.createTarget', { url: 'about:blank' });
({ sessionId } = await send('Target.attachToTarget', { targetId, flatten: true }));
await send('Runtime.enable');
await send('Page.enable');
await send('Page.addScriptToEvaluateOnNewDocument', {
    source: `window.__keys = [];
        for (const t of ['keydown', 'keyup']) addEventListener(t, (e) => __keys.push([t, e.code, e.keyCode, e.location].join(' ')), true);
        window.alert = (msg) => console.error('[alert] ' + msg);
        let module;
        Object.defineProperty(window, 'Module', {
            configurable: true,
            get() { return module; },
            set(m) {
                if (m === module) return;
                module = m;
                // A fixed seed and map pin the spawn points; input still comes live from the page.
                m.preRun = [...(m.preRun ?? []), () => Object.assign(m.ENV, { SMW_DUMP: '/dump.txt', SMW_SEED: '1', SMW_MAP: ${JSON.stringify(MAP)} })];
            },
        });`,
});

try {
    // Phone, landscape.
    await phone(true);
    await send('Page.navigate', { url });
    await waitFor(`document.getElementById('start')?.hidden === false`, 'the start screen', 120000);
    check(await evaluate(`matchMedia('(pointer: coarse)').matches`), 'emulated phone reports pointer: coarse');
    check(await evaluate(`document.documentElement.classList.contains('touch') && !document.getElementById('touch').hidden`), 'touch controls shown');
    check(await evaluate(`getComputedStyle(document.querySelector('.toolbar')).display === 'none'`), 'key hints hidden');
    check(await evaluate(`(() => {
        const s = document.getElementById('stage').getBoundingClientRect();
        return s.top >= 0 && s.bottom <= innerHeight + 0.5 && Math.abs(s.width / s.height - 4 / 3) < 0.01;
    })()`), 'game fits the landscape viewport at 4:3');
    check(await evaluate(`[...document.querySelectorAll('#touch [data-key]')].every((el) => {
        const r = el.getBoundingClientRect();
        return r.width >= 56 && r.height >= 56;
    })`), 'every control is at least 56 px');
    await screenshot('landscape-start');

    await tap(await center('#play'));
    await waitFor('started', 'Play to start the game', 10000);
    check(true, 'tapping Play started the game');
    await sleep(2500);
    await screenshot('splash');

    // Splash, then the main menu: D-pad down and up again must move the cursor and come back.
    const start = await center('[data-key=start]');
    const dpad = await center('.tc-dpad');
    const r = await evaluate(`document.querySelector('.tc-dpad').getBoundingClientRect().width / 2`);
    const arm = { up: { x: dpad.x, y: dpad.y - r * 0.7 }, down: { x: dpad.x, y: dpad.y + r * 0.7 }, right: { x: dpad.x + r * 0.7, y: dpad.y } };
    await tap(start);
    await sleep(1000);
    await screenshot('menu');
    const menuFocus = (await frameState()).menu;
    await tap(arm.down);
    await sleep(500);
    await screenshot('menu-down');
    const downFocus = (await frameState()).menu;
    await tap(arm.up);
    await sleep(500);
    check(downFocus !== menuFocus && (await frameState()).menu === menuFocus,
        `D-pad down/up moves the menu focus and back (${menuFocus} -> ${downFocus})`);
    let keys = await keyLog();
    check(keys.includes('keydown Enter 13 0') && keys.includes('keyup Enter 13 0'), 'Start sends Enter (code, keyCode)');
    check(keys.includes('keydown ArrowDown 40 0') && keys.includes('keydown ArrowUp 38 0'), 'D-pad sends arrow keys');

    // Main menu -> match type -> match settings -> game, as tools/replays/start_classic.txt does with Return.
    // Player 2 readies up from the keyboard (E), as in start_classic.txt; everything else is touch.
    for (let i = 0; i < 9 && (await frameState()).state !== 'gameplay'; i++) {
        if (i === 3) {
            for (const type of ['keyDown', 'keyUp']) {
                await send('Input.dispatchKeyEvent', { type, key: 'e', code: 'KeyE', windowsVirtualKeyCode: 69 });
            }
        }
        await tap(start);
        await sleep(900);
    }
    check((await frameState()).state === 'gameplay', 'Start taps reach a match');
    await sleep(2500);
    await screenshot('match');
    await keyLog();

    // Hold right, then run, then jump: three fingers down at once; slide the D-pad thumb to up-right.
    const run = await center('[data-key=run]');
    const jump = await center('.tc-btn[data-key=up]');
    const item = await center('[data-key=item]');
    // Each step waits on the dump for the frame that proves it, so slow frames do not cause flakes.
    const idle = await waitP1(0, (p) => !p.inair && p.vely >= 0, 'player 1 standing');
    console.log(`     P1 spawned at ${idle.x},${idle.y} (frame ${idle.frame})`);
    await down(1, arm.right);
    const walk = await waitP1(idle.frame, (p) => p.velx >= 4, 'player 1 walking right');
    await down(2, run);
    const runStart = (await frameState()).frame;
    const fast = await waitP1(runStart, (p) => p.velx > walk.velx, 'player 1 running', 2000).catch(() => null);
    check(!!fast, `P1 walks right (velx ${walk.velx}) and runs faster with Run held (velx ${fast?.velx})`);
    await screenshot('running');
    const grounded = await waitP1(runStart, (p) => !p.inair, 'player 1 on the ground');
    await down(3, jump);
    const leap = await waitP1(grounded.frame, (p) => p.vely < 0, 'player 1 jumping', 2000).catch(() => null);
    check(!!leap && leap.velx > walk.velx, `P1 jumps while still running right (vely ${leap?.vely}, velx ${leap?.velx})`);
    await screenshot('jumping');
    await move(1, { x: dpad.x + r * 0.6, y: dpad.y - r * 0.6 });
    await sleep(150);
    const held = await evaluate(`[...document.querySelectorAll('#touch .on')].map((el) => el.dataset.key || el.dataset.dir).sort().join(',')`);
    check(held === 'right,run,up,up', `right + run + jump held together, D-pad slid to up-right (lit: ${held})`);
    await up(3);
    await up(2);
    await up(1);
    await sleep(300);
    await tap(item);
    await sleep(300);
    keys = await keyLog();
    check(['keydown ArrowRight 39 0', 'keydown ControlRight 17 2', 'keydown ArrowUp 38 0', 'keydown ShiftRight 16 2']
        .every((k) => keys.includes(k)), 'right, Right Ctrl and Right Shift (location 2) and Up dispatched');
    check(['ArrowRight', 'ControlRight', 'ArrowUp', 'ShiftRight'].every((c) => keys.filter((k) => k.startsWith(`keyup ${c} `)).length >= 1),
        'every key released');
    // The recording is flushed every 60 frames; wait for the Item release, the last key above.
    const until = Date.now() + 5000;
    while (!/ up Right Shift$/m.test(await recording()) && Date.now() < until) await sleep(200);
    const rec = await recording();
    const lines = rec.split('\n');
    const at = (name) => lines.findIndex((l) => / down /.test(l) && l.endsWith(` down ${name}`));
    check(['Right', 'Right Ctrl', 'Up', 'Right Shift'].every((n) => at(n) >= 0),
        'the game saw Right, Right Ctrl, Up and Right Shift (session recording)');
    const frameOf = (name, dir) => Number((lines.findLast((l) => l.endsWith(` ${dir} ${name}`)) ?? 'NaN').split(' ')[0]);
    check(frameOf('Up', 'down') > frameOf('Right Ctrl', 'down') && frameOf('Up', 'down') < frameOf('Right Ctrl', 'up'),
        'jump pressed while run was held');
    writeFileSync(join(out, 'recording.txt'), rec);

    // Stuck keys: a finger still down when the window loses focus or the touch is cancelled is released.
    await down(1, arm.right);
    await sleep(100);
    await evaluate(`dispatchEvent(new Event('blur'))`);
    keys = await keyLog();
    check(keys.at(-1) === 'keyup ArrowRight 39 0', 'blur releases held keys');
    await up(1);
    await down(1, run);
    await sleep(100);
    await send('Input.dispatchTouchEvent', { type: 'touchCancel', touchPoints: [] });
    fingers.clear();
    keys = await keyLog();
    check(keys.at(-1) === 'keyup ControlRight 17 2', 'touchcancel releases held keys');

    await tap(await center('[data-key=back]'));
    await sleep(500);
    await screenshot('back');
    check((await keyLog()).includes('keydown Escape 27 0'), 'Back sends Escape');

    // Floating stick: the mode button switches to it and remembers the choice; the stick spawns under the
    // thumb, presses 8-way directions past a small dead zone, and its base follows the thumb past its radius.
    await evaluate(`document.querySelector('[data-action=mode]').click()`);
    check(await evaluate(`document.documentElement.classList.contains('stick') && localStorage.getItem('smw-touch-stick') === 'stick'
        && document.querySelector('[data-action=mode]').textContent === 'Stick'`), 'the mode button switches to the floating stick');
    await keyLog();
    const zone = await evaluate(`(() => { const r = document.querySelector('.tc-stick').getBoundingClientRect(); return { left: r.left, top: r.top, width: r.width, height: r.height }; })()`);
    const p0 = { x: zone.left + 55, y: zone.top + zone.height * 0.6 };
    const baseCenter = () => evaluate(`(() => {
        const el = document.querySelector('.tc-stick-base');
        if (el.hidden) return null;
        const r = el.getBoundingClientRect();
        return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
    })()`);
    await down(4, p0);
    const b0 = await baseCenter();
    check(!!b0 && Math.hypot(b0.x - p0.x, b0.y - p0.y) < 1, `the stick appears under the thumb (${JSON.stringify(b0)})`);
    await move(4, { x: p0.x + 6, y: p0.y });
    check((await keyLog()).length === 0, 'no key inside the dead zone');
    await move(4, { x: p0.x + 30, y: p0.y - 30 });
    keys = await keyLog();
    check(keys.includes('keydown ArrowRight 39 0') && keys.includes('keydown ArrowUp 38 0'), 'dragging up-right presses right and up');
    await move(4, { x: p0.x + 90, y: p0.y });
    const b1 = await baseCenter();
    check(!!b1 && Math.abs(b1.x - (p0.x + 50)) < 1 && Math.abs(b1.y - p0.y) < 1, `the base follows the thumb past its radius (${JSON.stringify(b1)})`);
    await keyLog();
    await move(4, { x: p0.x + 20, y: p0.y });
    keys = await keyLog();
    check(keys.includes('keyup ArrowRight 39 0') && keys.includes('keydown ArrowLeft 37 0'), 'pulling back past the base turns left at once');
    await move(4, { x: zone.left + zone.width + 300, y: p0.y });
    await screenshot('stick');
    check(await evaluate(`(() => {
        const z = document.querySelector('.tc-stick').getBoundingClientRect();
        return [...document.querySelectorAll('.tc-stick-base, .tc-stick-knob')].every((el) => el.getBoundingClientRect().right <= z.right + 0.5);
    })()`), 'dragged onto the game, the stick stays in its zone');
    await keyLog();
    await move(4, { x: zone.left, y: p0.y });
    keys = await keyLog();
    check(keys.includes('keyup ArrowRight 39 0') && keys.includes('keydown ArrowLeft 37 0'), 'pulling back from over the game turns left at once');
    await up(4);
    keys = await keyLog();
    check(keys.at(-1) === 'keyup ArrowLeft 37 0' && (await baseCenter()) === null, 'releasing the thumb releases the key and hides the stick');

    // Portrait: the game on top, the controls below it.
    await phone(false);
    await sleep(500);
    check(await evaluate(`(() => {
        const s = document.getElementById('stage').getBoundingClientRect();
        const below = [...document.querySelectorAll('#touch .tc-zone, #touch .tc-util:not([hidden])')].every((el) => el.getBoundingClientRect().top >= s.bottom);
        return s.top >= 0 && s.width <= innerWidth + 0.5 && below;
    })()`), 'portrait: controls sit below the game');
    await screenshot('portrait');

    // Desktop: no controls, key hints back; the footer link forces them on and the choice is remembered.
    await desktop();
    await send('Page.reload');
    await waitFor(`document.getElementById('start')?.hidden === false`, 'the start screen (desktop)', 120000);
    check(await evaluate(`!matchMedia('(pointer: coarse)').matches && !document.documentElement.classList.contains('touch')
        && getComputedStyle(document.getElementById('touch')).display === 'none'
        && getComputedStyle(document.querySelector('.toolbar')).display !== 'none'`), 'desktop: controls hidden, key hints shown');
    await screenshot('desktop');
    await evaluate(`[...document.querySelectorAll('footer a')].find((a) => a.textContent === 'show touch controls').click()`);
    await send('Page.reload');
    await waitFor(`document.getElementById('start')?.hidden === false`, 'the start screen (forced on)', 120000);
    check(await evaluate(`document.documentElement.classList.contains('touch') && localStorage.getItem('smw-touch-controls') === 'on'`),
        'desktop: forcing controls on survives a reload');
    await screenshot('desktop-forced-on');
    await evaluate(`document.querySelector('[data-action=hide]').click()`);
    check(await evaluate(`!document.documentElement.classList.contains('touch') && localStorage.getItem('smw-touch-controls') === 'off'`),
        'the hide button turns them off');
} catch (e) {
    console.error(e.message);
    failures++;
    await screenshot('error').catch(() => {});
}

console.log(failures ? `${failures} check(s) failed` : 'all checks passed');
process.exit(failures ? 1 : 0);
