#!/usr/bin/env node
// Text entry on a touch screen, end to end: start smw_relay, open the web build (dist/web) as an emulated
// phone in headless Chrome, walk to Multiplayer with the touch controls, type a player name and a room name
// through the page's text input (web/textinput.js) the way an on-screen keyboard does, create the room, and
// check that the relay has a room of that name hosted by that player. Also checks that the touch D-pad still
// drives the menus after typing, that the session recording holds the typed keys, and that a desktop page
// never shows the input. Last, a gamepad-only replay types both names with the game's on-screen keyboard.
//
// Usage: node tools/web_text_entry_test.mjs [out_dir]
//   CHROME     Chrome binary (default: the macOS Google Chrome app)
//   RELAY_BIN  smw_relay binary (default: target/release/smw_relay)
//   WEB_DIR    web build (default: dist/web)
//   CPU_THROTTLE  slow the page's CPU by this factor (Chrome's emulation), as on a slow CI runner
import { spawn } from 'node:child_process';
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
const out = resolve(process.argv[2] ?? join(tmpdir(), 'smw-web-text-entry'));
mkdirSync(out, { recursive: true });

const PLAYER = 'Kyle_7!';
const ROOM = 'Phone Room';
const PAD_ROOM = 'Pad Room';

// A pad-only replay for the in-game on-screen keyboard (MI_TextField): its key grid and moves, as in the game.
const OSK_KEYS = ['1234567890', 'qwertyuiop', 'asdfghjkl-', 'zxcvbnm,./'];
const OSK_SHIFTED = ['!@#$%^&*()', 'QWERTYUIOP', 'ASDFGHJKL_', 'ZXCVBNM<>?'];
const OSK_BOTTOM = [0, 3, 6, 8];
const bottomKey = (col) => OSK_BOTTOM.findLastIndex((start) => col >= start);
const oskMove = ([r, c], d) => {
    if (d === 'up') return [(r + 4) % 5, c];
    if (d === 'down') return [(r + 1) % 5, c];
    const step = d === 'left' ? -1 : 1;
    if (r === 4) return [r, OSK_BOTTOM[(bottomKey(c) + step + 4) % 4]];
    return [r, (c + step + 10) % 10];
};
const oskPath = (from, goal) => {
    const queue = [[from, []]];
    const seen = new Set([String(from)]);
    while (queue.length) {
        const [p, moves] = queue.shift();
        if (goal(p)) return [p, moves];
        for (const d of ['up', 'down', 'left', 'right']) {
            const n = oskMove(p, d);
            if (!seen.has(String(n))) {
                seen.add(String(n));
                queue.push([n, [...moves, d]]);
            }
        }
    }
};
const padScript = () => {
    const HAT = { up: 1, right: 2, down: 4, left: 8 };
    const lines = ['0 jhat 0 0 0'];
    let f = 10;
    const hat = (d) => { lines.push(`${f} jhat 0 0 ${HAT[d]}`, `${f + 2} jhat 0 0 0`); f += 6; };
    const button = (b) => { lines.push(`${f} jbutton 0 ${b} 1`, `${f + 2} jbutton 0 ${b} 0`); f += 6; };
    let pos;
    const go = (goal) => {
        let moves;
        [pos, moves] = oskPath(pos, goal);
        moves.forEach(hat);
        button(0);
    };
    const type = (text) => {
        pos = [0, 0];
        for (const ch of text) {
            if (ch === ' ') {
                go(([r, c]) => r === 4 && bottomKey(c) === 1);
                continue;
            }
            let rows = OSK_KEYS;
            if (!OSK_KEYS.some((row) => row.includes(ch))) {
                rows = OSK_SHIFTED;
                go(([r, c]) => r === 4 && bottomKey(c) === 0);
            }
            const r = rows.findIndex((row) => row.includes(ch));
            go(([pr, pc]) => pr === r && pc === rows[r].indexOf(ch));
        }
        go(([r, c]) => r === 4 && bottomKey(c) === 3);
    };
    button(0);
    f += 20;
    hat('down');
    hat('down');
    button(0);
    f += 20;
    hat('up');
    button(0);
    f += 10;
    for (let i = 0; i < 'Player'.length; i++) button(1);
    type(PLAYER);
    hat('down');
    hat('down');
    button(0);
    f += 120;
    button(0);
    f += 20;
    button(0);
    f += 20;
    button(0);
    f += 10;
    type(PAD_ROOM);
    hat('down');
    hat('down');
    button(0);
    f += 120;
    return { frames: f, text: `#@ seed=1\n#@ frames=${f}\n${lines.join('\n')}\n` };
};

const children = [];
const profile = mkdtempSync(join(tmpdir(), 'smw-text-chrome-'));
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
    rmSync(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
};
process.on('exit', cleanup);
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => process.exit(1));
const deadlineMs = 240000;
setTimeout(() => {
    console.error(`not finished after ${deadlineMs / 1000} s; giving up`);
    process.exit(1);
}, deadlineMs).unref();

const types = {
    '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm',
    '.png': 'image/png', '.webmanifest': 'application/manifest+json', '.data': 'application/octet-stream',
};
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

const relayPort = await new Promise((ok) => {
    const s = createNetServer();
    s.listen(0, '127.0.0.1', () => {
        const { port } = s.address();
        s.close(() => ok(port));
    });
});
let relayLog = '';
const relay = spawn(relayBin, ['--port', String(relayPort)], {
    cwd: out,
    env: { ...process.env, SMW_RELAY_ALLOWED_ORIGINS: origin },
    stdio: ['ignore', 'pipe', 'pipe'],
    detached: true,
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

const browser = spawn(chrome, [
    '--headless=new', '--remote-debugging-port=0', `--user-data-dir=${profile}`, '--disable-extensions',
    '--autoplay-policy=no-user-gesture-required', '--mute-audio', '--window-size=900,900', 'about:blank',
], { stdio: ['ignore', 'ignore', 'pipe'], detached: true });
children.push(browser);
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
const pageErrors = [];
ws.addEventListener('message', (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.id && pending.has(msg.id)) {
        const { ok, fail } = pending.get(msg.id);
        pending.delete(msg.id);
        msg.error ? fail(new Error(`${msg.error.message}`)) : ok(msg.result);
    } else if (msg.method === 'Runtime.consoleAPICalled') {
        consoleLines.push(msg.params.args.map((a) => a.value ?? a.description ?? '').join(' '));
    } else if (msg.method === 'Runtime.exceptionThrown') {
        const d = msg.params.exceptionDetails;
        pageErrors.push(d.exception?.description ?? d.text);
    }
});
let sessionId;
const send = (method, params = {}) =>
    new Promise((ok, fail) => {
        const id = nextId++;
        pending.set(id, { ok, fail });
        ws.send(JSON.stringify({ id, method, params, sessionId }));
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
        await sleep(100);
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

const fingers = new Map();
const touch = async (type, id, point) => {
    if (type === 'touchEnd') fingers.delete(id);
    else fingers.set(id, { x: point.x, y: point.y, id });
    await send('Input.dispatchTouchEvent', { type, touchPoints: [...fingers.values()] });
};
const tap = async (p, holdMs = 120) => {
    await touch('touchStart', 9, p);
    await sleep(holdMs);
    await touch('touchEnd', 9);
};
const center = (selector) => evaluate(`(() => {
    const r = document.querySelector(${JSON.stringify(selector)}).getBoundingClientRect();
    return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
})()`);
// The harness dump's newest menu record: "M <menu> focus=<n> modifying=<0|1>".
const menu = async () => {
    const dump = await evaluate(`(() => { try { return Module.FS.readFile('/dump.txt', { encoding: 'utf8' }).slice(-3000); } catch { return ''; } })()`);
    const line = dump.split('\n').filter((l) => l.startsWith('M ')).pop() ?? '';
    const m = line.match(/^M (\S+) focus=(-?\d+) modifying=(\d)/);
    return m ? { name: m[1], focus: Number(m[2]), modifying: m[3] === '1' } : { name: '', focus: -1, modifying: false };
};
const waitMenu = async (pred, what, ms = 10000) => {
    const until = Date.now() + ms;
    for (;;) {
        const m = await menu();
        if (pred(m)) return m;
        if (Date.now() > until) throw new Error(`timed out waiting for ${what} (menu ${JSON.stringify(m)})`);
        await sleep(100);
    }
};
const recording = () => evaluate(`(() => {
    const names = Module.FS.readdir(REPLAY_DIR).filter((n) => /\\.(txt|smwrp)$/.test(n)).sort();
    return names.length ? Module.FS.readFile(REPLAY_DIR + '/' + names[names.length - 1], { encoding: 'utf8' }) : '';
})()`);

const { targetId } = await send('Target.createTarget', { url: 'about:blank' });
({ sessionId } = await send('Target.attachToTarget', { targetId, flatten: true }));
await send('Runtime.enable');
await send('Page.enable');
await send('Page.addScriptToEvaluateOnNewDocument', {
    source: `window.__fields = [];
        let module;
        Object.defineProperty(window, 'Module', {
            configurable: true,
            get() { return module; },
            set(m) {
                if (m === module) return;
                module = m;
                m.preRun = [...(m.preRun ?? []), () => Object.assign(m.ENV, { SMW_DUMP: '/dump.txt', SMW_SEED: '1' })];
            },
        });
        window.alert = (msg) => console.error('[alert] ' + msg);`,
});
const page = `${origin}/index.html?relay=${encodeURIComponent(`ws://127.0.0.1:${relayPort}`)}`;

// Press the touch D-pad or a touch key until the dump shows the wanted menu state.
let start, arm;
const press = async (p, ms = 350) => {
    await tap(p);
    await sleep(ms);
};
const focusOn = async (target, what) => {
    for (let i = 0; i < 12; i++) {
        const m = await menu();
        if (m.focus === target) return;
        await press(m.focus < target ? arm.down : arm.up);
    }
    throw new Error(`could not reach ${what}`);
};
// Types the way an on-screen keyboard does: Backspaces on the focused input, then committed text.
const typeInto = async (clear, text) => {
    for (let i = 0; i < clear; i++) {
        for (const type of ['keyDown', 'keyUp']) {
            await send('Input.dispatchKeyEvent', { type, key: 'Backspace', code: 'Backspace', windowsVirtualKeyCode: 8 });
        }
    }
    await send('Input.insertText', { text });
};
const lastField = () => evaluate('__fields.filter((f) => f.active).pop()');

try {
    await send('Emulation.setDeviceMetricsOverride', {
        width: 844, height: 390, deviceScaleFactor: 3, mobile: true, screenOrientation: { type: 'landscapePrimary', angle: 90 },
    });
    await send('Emulation.setTouchEmulationEnabled', { enabled: true, maxTouchPoints: 5 });
    if (process.env.CPU_THROTTLE) await send('Emulation.setCPUThrottlingRate', { rate: Number(process.env.CPU_THROTTLE) });
    await send('Page.navigate', { url: page });
    await waitFor(`document.getElementById('start')?.hidden === false`, 'the start screen', 120000);
    await evaluate(`(() => {
        const hook = Module.onTextField;
        Module.onTextField = (f) => { __fields.push(f); hook(f); };
    })()`);
    await tap(await center('#play'));
    await waitFor('started', 'Play to start the game', 10000);
    start = await center('[data-key=start]');
    const dpad = await center('.tc-dpad');
    const r = await evaluate(`document.querySelector('.tc-dpad').getBoundingClientRect().width / 2`);
    arm = { up: { x: dpad.x, y: dpad.y - r * 0.7 }, down: { x: dpad.x, y: dpad.y + r * 0.7 } };
    for (let i = 0; i < 20 && (await menu()).name !== 'main'; i++) await press(start, 1000);
    await waitMenu((m) => m.name === 'main', 'the main menu');

    // Main menu -> Multiplayer, all with the touch controls.
    await focusOn(3, 'Multiplayer');
    await press(start);
    await waitMenu((m) => m.name === 'net_servers', 'the servers menu');

    // Player name: Start opens the field and the input over it takes the focus, as a tap would.
    await focusOn(0, 'Your name');
    await press(start);
    await waitMenu((m) => m.modifying, 'the name field to open');
    await waitFor(`document.activeElement?.id === 'textEntry' && !document.getElementById('textEntry').hidden`, 'the text input to take focus', 5000);
    check(await evaluate(`(() => {
        const i = document.getElementById('textEntry').getBoundingClientRect();
        const c = document.getElementById('canvas').getBoundingClientRect();
        const s = c.width / 640;
        return Math.abs(i.top - (c.top + 120 * s)) < 2 && Math.abs(i.left - (c.left + (70 + 150) * s)) < 2;
    })()`), 'the input sits over the name field');
    check((await lastField()).value === 'Player', 'the input starts with the current name');
    await screenshot('name-open');
    await typeInto('Player'.length, PLAYER);
    await waitFor(`__fields.at(-1).value === ${JSON.stringify(PLAYER)}`, 'the game to hold the typed name', 10000);
    await screenshot('name-typed');
    for (const type of ['keyDown', 'keyUp']) await send('Input.dispatchKeyEvent', { type, key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13 });
    await waitMenu((m) => !m.modifying, 'Enter to close the name field');
    await waitFor(`document.getElementById('textEntry').hidden && document.activeElement?.id !== 'textEntry'`, 'the input to close', 3000);
    check(true, `typed the player name "${PLAYER}" (Backspace, then committed text, then Enter)`);

    // The touch controls still drive the menus after typing.
    check(await evaluate(`!document.getElementById('touch').hidden`), 'touch controls still shown after typing');
    await focusOn(3, 'Connect');
    check(true, 'the touch D-pad moves the menu after typing');
    await press(start);
    await waitMenu((m) => m.name === 'net_lobby', 'the lobby', 15000);
    await press(start);
    await waitMenu((m) => m.name === 'net_new_room_settings', 'the new room settings');
    await press(start);
    await waitMenu((m) => m.name === 'net_new_room', 'the new room menu');

    // Room name: typed, then confirmed by closing the keyboard (the input loses focus).
    await focusOn(0, 'Room name');
    await press(start);
    await waitMenu((m) => m.modifying, 'the room name field to open');
    await waitFor(`document.activeElement?.id === 'textEntry'`, 'the text input to take focus', 5000);
    await typeInto(0, ROOM);
    await waitFor(`__fields.at(-1).value === ${JSON.stringify(ROOM)}`, 'the game to hold the room name', 10000);
    await screenshot('room-typed');
    await evaluate(`document.getElementById('textEntry').blur()`);
    await waitMenu((m) => !m.modifying, 'closing the keyboard to confirm the room name');
    check(true, `typed the room name "${ROOM}" and confirmed it by closing the keyboard`);

    // Password: typed and erased, then Escape closes the field the way the game's Back does.
    await focusOn(1, 'Password');
    await press(start);
    await waitMenu((m) => m.modifying, 'the password field to open');
    await waitFor(`document.activeElement?.id === 'textEntry'`, 'the text input to take focus', 5000);
    await typeInto(0, 'x');
    await waitFor(`__fields.at(-1).value === 'x'`, 'the game to hold the password', 10000);
    await typeInto(1, '');
    await waitFor(`__fields.at(-1).value === ''`, 'Backspace to erase the password', 10000);
    for (const type of ['keyDown', 'keyUp']) await send('Input.dispatchKeyEvent', { type, key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 });
    await waitMenu((m) => !m.modifying, 'Escape to close the password field');
    check(await evaluate(`document.getElementById('textEntry').hidden`), 'Escape closes the field and the input');

    await focusOn(2, 'Create');
    await press(start);
    const until = Date.now() + 10000;
    while (!relayLog.includes('New room by') && Date.now() < until) await sleep(200);
    await sleep(500);
    await screenshot('room');
    const created = relayLog.split('\n').find((l) => l.includes('New room by')) ?? '';
    console.log(`     relay: ${created.trim()}`);
    check(created.includes(`[${PLAYER}@`) && created.includes(`name: ${ROOM};`), 'the relay lists the room with the typed room and player names');

    const rec = await recording();
    writeFileSync(join(out, 'recording.txt'), rec);
    const keys = rec.split('\n').filter((l) => / (down|up) /.test(l));
    const has = (name) => keys.some((l) => l.endsWith(` down ${name}`));
    check(['Left Shift', 'K', '7', '1', 'Backspace', 'Space', 'R', 'Escape'].every(has),
        'the session recording holds the typed keys (letters, Shift, digits, Space, Backspace, Escape)');

    // Desktop: the input never appears, and the keyboard types into the field as before.
    await send('Emulation.setTouchEmulationEnabled', { enabled: false });
    await send('Emulation.setDeviceMetricsOverride', { width: 1280, height: 800, deviceScaleFactor: 1, mobile: false });
    await send('Page.navigate', { url: page });
    await waitFor(`document.getElementById('start')?.hidden === false`, 'the start screen (desktop)', 120000);
    await evaluate(`(() => {
        const hook = Module.onTextField;
        Module.onTextField = (f) => { __fields.push(f); hook(f); };
    })()`);
    check(await evaluate(`!document.documentElement.classList.contains('touch')`), 'desktop: no touch controls');
    await evaluate(`document.getElementById('play').click()`);
    await waitFor('started', 'Play (desktop)', 10000);
    const key = async (k, code, vk, mods = 0) => {
        for (const type of ['keyDown', 'keyUp']) await send('Input.dispatchKeyEvent', { type, key: k, code, windowsVirtualKeyCode: vk, modifiers: mods });
        await sleep(150);
    };
    for (let i = 0; i < 20 && (await menu()).name !== 'main'; i++) {
        await key('Enter', 'Enter', 13);
        await sleep(850);
    }
    await waitMenu((m) => m.name === 'main', 'the main menu (desktop)');
    for (let i = 0; i < 12 && (await menu()).focus !== 3; i++) await key('ArrowDown', 'ArrowDown', 40);
    await key('Enter', 'Enter', 13);
    await waitMenu((m) => m.name === 'net_servers', 'the servers menu (desktop)');
    for (let i = 0; i < 4 && (await menu()).focus !== 0; i++) await key('ArrowUp', 'ArrowUp', 38);
    await key('Enter', 'Enter', 13);
    await waitMenu((m) => m.modifying, 'the name field (desktop)');
    for (let i = 0; i < PLAYER.length; i++) await key('Backspace', 'Backspace', 8);
    await send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Shift', code: 'ShiftLeft', windowsVirtualKeyCode: 16, location: 1, modifiers: 8 });
    await key('D', 'KeyD', 68, 8);
    await send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Shift', code: 'ShiftLeft', windowsVirtualKeyCode: 16, location: 1 });
    await key('k', 'KeyK', 75);
    await waitFor(`__fields.at(-1).value === 'Dk'`, 'desktop typing to reach the field', 5000).catch(() => {});
    check((await lastField())?.value === 'Dk' && (await evaluate(`document.getElementById('textEntry').hidden && document.activeElement?.id !== 'textEntry'`)),
        `desktop: keys type straight into the field, no input shown (value ${JSON.stringify((await evaluate('__fields.at(-1)'))?.value)})`);
    await key('Enter', 'Enter', 13);
    await waitMenu((m) => !m.modifying, 'Enter to close the field (desktop)');
    await screenshot('desktop');
    // A gamepad only: the game's own on-screen keyboard types the player and room names, from a replay.
    const pad = padScript();
    const logStart = relayLog.length;
    await send('Page.addScriptToEvaluateOnNewDocument', {
        source: `(() => {
            const replay = ${JSON.stringify(pad.text)};
            const set = Object.getOwnPropertyDescriptor(window, 'Module').set;
            Object.defineProperty(window, 'Module', {
                configurable: true,
                get() { return window.__module; },
                set(m) {
                    if (m === window.__module) return;
                    window.__module = m;
                    m.noInitialRun = false;
                    m.preRun = [...(m.preRun ?? []), () => {
                        Object.assign(m.ENV, { SMW_REPLAY: '/pad.txt', SMW_FRAMES: '${pad.frames}' });
                        m.FS.writeFile('/pad.txt', replay);
                    }];
                    set(m);
                },
            });
        })();`,
    });
    await send('Page.navigate', { url: page });
    const padUntil = Date.now() + 120000;
    while (!relayLog.slice(logStart).includes('New room by') && Date.now() < padUntil) await sleep(500);
    await sleep(500);
    await screenshot('pad-room');
    const padRoom = relayLog.slice(logStart).split('\n').find((l) => l.includes('New room by')) ?? '';
    console.log(`     relay: ${padRoom.trim()}`);
    check(padRoom.includes(`[${PLAYER}@`) && padRoom.includes(`name: ${PAD_ROOM};`),
        'pad only: the on-screen keyboard typed the player and room names the relay lists');
} catch (e) {
    console.error(e.message);
    failures++;
    await screenshot('error').catch(() => {});
    writeFileSync(join(out, 'recording.txt'), await recording().catch(() => ''));
}

check(!pageErrors.length, `no page errors${pageErrors.length ? ':\n' + pageErrors.join('\n') : ''}`);
writeFileSync(join(out, 'relay.log'), relayLog);
writeFileSync(join(out, 'console.log'), consoleLines.join('\n') + '\n');
console.log(failures ? `${failures} check(s) failed` : 'all checks passed');
process.exit(failures ? 1 : 0);
