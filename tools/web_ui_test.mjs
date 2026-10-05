#!/usr/bin/env node
// The web page's manual and Menu button in headless Chrome. The manual (desktop and phone): every
// contents link has its heading, every image loads, nothing scrolls sideways at 390 px. The game page:
// a keyboard session reaches a match, Tab and Enter press Menu, and after the confirm the start screen
// lists that match; a watched clip ends on Menu / Watch again, and Menu mid-watch returns without a
// confirm; on an emulated phone the touch Menu button does the same. Exits non-zero on a failed check.
// Screenshots go to out_dir.
//
// Usage: node tools/web_ui_test.mjs [out_dir]
//   CHROME   Chrome binary (default: the macOS Google Chrome app)
//   WEB_DIR  the web build to serve (default: dist/web, see tools/package_web.sh)
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, extname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const web = resolve(process.env.WEB_DIR ?? join(repo, 'dist', 'web'));
const chrome = process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const out = resolve(process.argv[2] ?? join(tmpdir(), 'smw-web-ui-out'));
mkdirSync(out, { recursive: true });

const types = {
    '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm',
    '.png': 'image/png', '.webmanifest': 'application/manifest+json', '.data': 'application/octet-stream',
};
const missing = [];
const server = createServer((req, res) => {
    const path = decodeURIComponent(req.url.split('?')[0]);
    const file = join(web, path === '/' ? 'index.html' : path);
    try {
        const body = readFileSync(file);
        res.writeHead(200, { 'Content-Type': types[extname(file)] ?? 'application/octet-stream' });
        res.end(body);
    } catch {
        missing.push(path);
        res.writeHead(404);
        res.end();
    }
});
await new Promise((ok) => server.listen(0, '127.0.0.1', ok));
const url = `http://127.0.0.1:${server.address().port}/`;

const profile = mkdtempSync(join(tmpdir(), 'smw-web-chrome-'));
const browser = spawn(chrome, [
    '--headless=new', '--remote-debugging-port=0', `--user-data-dir=${profile}`, '--disable-extensions',
    '--autoplay-policy=no-user-gesture-required', '--mute-audio', '--window-size=1280,800', 'about:blank',
], { stdio: ['ignore', 'ignore', 'pipe'], detached: true });
const cleanup = () => {
    try {
        process.kill(-browser.pid, 'SIGKILL');
    } catch {}
    server.close();
    rmSync(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
};
process.on('exit', cleanup);
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => process.exit(1));
const deadlineMs = 300000;
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
const dialogs = [];
let sessionId;
const send = (method, params = {}, session = sessionId) =>
    new Promise((ok, fail) => {
        const id = nextId++;
        pending.set(id, { ok, fail });
        ws.send(JSON.stringify({ id, method, params, sessionId: session }));
    });
ws.addEventListener('message', (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.id && pending.has(msg.id)) {
        const { ok, fail } = pending.get(msg.id);
        pending.delete(msg.id);
        msg.error ? fail(new Error(`${msg.error.message}`)) : ok(msg.result);
    } else if (msg.method === 'Runtime.consoleAPICalled') {
        consoleLines.push(msg.params.args.map((a) => a.value ?? a.description ?? '').join(' '));
    } else if (msg.method === 'Page.javascriptDialogOpening') {
        dialogs.push(`${msg.params.type}: ${msg.params.message}`);
        send('Page.handleJavaScriptDialog', { accept: true });
    }
});
const sleep = (ms) => new Promise((ok) => setTimeout(ok, ms));
const evaluate = async (expression) => {
    const r = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
    if (r.exceptionDetails) throw new Error(`${expression}: ${r.exceptionDetails.exception?.description ?? r.exceptionDetails.text}`);
    return r.result.value;
};
const waitFor = async (expression, what, ms = 60000) => {
    const until = Date.now() + ms;
    while (!(await evaluate(expression).catch(() => false))) {
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
const screenshot = async (name, full = false) => {
    const { data } = await send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: full });
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
const KEYS = { Enter: 13, Tab: 9, Escape: 27, ArrowLeft: 37, ArrowUp: 38, ArrowRight: 39, e: 69 };
const key = async (name, holdMs = 80) => {
    const code = name.length === 1 ? `Key${name.toUpperCase()}` : name;
    const text = { Enter: '\r', e: 'e' }[name];
    for (const type of ['keyDown', 'keyUp']) {
        await send('Input.dispatchKeyEvent', { type, key: name, code, windowsVirtualKeyCode: KEYS[name], ...(type === 'keyDown' && text ? { text } : {}) });
        if (type === 'keyDown') await sleep(holdMs);
    }
    await sleep(250);
};
const frameState = async () => {
    const dump = await evaluate(`(() => { try { return Module.FS.readFile('/dump.txt', { encoding: 'utf8' }).slice(-4000); } catch { return ''; } })()`);
    const block = dump.slice(dump.lastIndexOf('\nF ') + 1).split('\n');
    return { state: (block[0] ?? '').split(' ')[2], menu: (block.find((l) => l.startsWith('M ')) ?? '').split(' ')[1] };
};
const recordings = () => evaluate(`Module.FS.readdir(REPLAY_DIR).filter((n) => n.endsWith('.txt')).sort()`);
const startScreen = `document.getElementById('start')?.hidden === false`;
// Run `action`, which makes the page reload itself, and wait for the fresh page's start screen.
const reloadsToStart = async (action, what) => {
    await evaluate('window.smwOldPage = true');
    await action();
    await waitFor(`!window.smwOldPage && ${startScreen}`, what, 60000);
};
const matchRows = () => evaluate(`[...document.querySelectorAll('#matchList .match-name')].map((e) => e.textContent)`);
const visible = (selector) => evaluate(`(() => {
    const el = document.querySelector(${JSON.stringify(selector)});
    if (!el) return false;
    const r = el.getBoundingClientRect();
    return r.width > 0 && r.height > 0 && getComputedStyle(el).visibility !== 'hidden';
})()`);
const desktop = () => send('Emulation.setDeviceMetricsOverride', { width: 1280, height: 800, deviceScaleFactor: 1, mobile: false });
const phone = async (width, height, angle) => {
    await send('Emulation.setDeviceMetricsOverride', {
        width, height, deviceScaleFactor: 3, mobile: true,
        screenOrientation: { type: angle ? 'landscapePrimary' : 'portraitPrimary', angle },
    });
    await send('Emulation.setTouchEmulationEnabled', { enabled: true, maxTouchPoints: 5 });
};

const { targetId } = await send('Target.createTarget', { url: 'about:blank' });
({ sessionId } = await send('Target.attachToTarget', { targetId, flatten: true }));
await send('Runtime.enable');
await send('Page.enable');
await send('Page.addScriptToEvaluateOnNewDocument', {
    source: `let module;
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

const checkManual = async (label) => {
    missing.length = 0;
    await send('Page.navigate', { url: `${url}manual.html` });
    await waitFor(`document.readyState === 'complete'`, 'the manual');
    const toc = await evaluate(`(() => {
        const links = [...document.querySelectorAll('#toc a')];
        return { count: links.length, broken: links.filter((a) => !document.getElementById(decodeURIComponent(a.hash.slice(1)))).map((a) => a.hash) };
    })()`);
    check(toc.count > 100 && toc.broken.length === 0, `${label}: ${toc.count} contents links, each with its heading (${toc.broken.join(' ') || 'none broken'})`);
    const images = await evaluate(`Promise.all([...document.images].map((img) => {
        img.loading = 'eager';
        return img.decode().then(() => img.naturalWidth > 0 ? null : img.src, () => img.src);
    })).then((bad) => ({ count: document.images.length, bad: bad.filter(Boolean) }))`);
    check(images.count > 100 && images.bad.length === 0 && missing.length === 0,
        `${label}: ${images.count} images load, no 404s (${[...images.bad, ...missing].join(' ') || 'none'})`);
    check(await evaluate(`document.documentElement.scrollWidth <= innerWidth`), `${label}: no horizontal scroll (${await evaluate('document.documentElement.scrollWidth')} <= ${await evaluate('innerWidth')})`);
    check(await evaluate(`(() => {
        const links = [...document.querySelectorAll('.top a')];
        return links.some((a) => a.textContent.includes('Back to the game') && a.href === ${JSON.stringify(url)})
            && links.some((a) => a.textContent.includes('View on GitHub') && a.href.endsWith('/docs/GAME_MANUAL.md'));
    })()`), `${label}: Back to the game and View on GitHub links`);
};

try {
    // The manual on a desktop: a sticky contents sidebar that follows the reader.
    await desktop();
    await checkManual('manual, desktop');
    await screenshot('manual-desktop');
    await evaluate(`document.querySelector('#toc a[href="#-capture-the-flag"]').click()`);
    await sleep(300);
    check(await evaluate(`(() => {
        const top = document.getElementById('-capture-the-flag').getBoundingClientRect().top;
        const toc = document.querySelector('#toc nav').getBoundingClientRect();
        return top >= 0 && top < 40 && toc.top >= 0 && toc.bottom <= innerHeight
            && document.querySelector('#toc a.current')?.hash === '#-capture-the-flag';
    })()`), 'manual, desktop: a contents link scrolls to its heading; the sidebar stays and marks it');
    await screenshot('manual-desktop-ctf');
    await evaluate(`document.getElementById('level-editor').scrollIntoView()`);
    await sleep(300);
    await screenshot('manual-desktop-editor');

    // The manual on a phone: contents collapse into a bar.
    await phone(390, 844, 0);
    await checkManual('manual, phone');
    check(await evaluate(`!document.getElementById('toc').open`), 'manual, phone: contents start collapsed');
    await screenshot('manual-phone');
    await tap(await center('#toc summary'));
    await sleep(200);
    check(await evaluate(`document.getElementById('toc').open`), 'manual, phone: a tap opens the contents');
    await screenshot('manual-phone-toc');
    await evaluate(`document.querySelector('#toc a[href="#items"]').scrollIntoView({ block: 'center' })`);
    await tap(await center('#toc a[href="#items"]'));
    await sleep(400);
    check(await evaluate(`(() => {
        const top = document.getElementById('items').getBoundingClientRect().top;
        const bar = document.getElementById('toc').getBoundingClientRect().bottom;
        return !document.getElementById('toc').open && top >= bar && top < bar + 40;
    })()`), 'manual, phone: a contents link closes them and scrolls its heading to just below the contents bar');
    await evaluate(`document.getElementById('game-controls').scrollIntoView()`);
    await sleep(200);
    await screenshot('manual-phone-table');
    await evaluate(`document.getElementById('-classic').scrollIntoView()`);
    await sleep(200);
    await screenshot('manual-phone-modes');
    await send('Emulation.setTouchEmulationEnabled', { enabled: false });

    // The game page on a desktop.
    await desktop();
    await send('Page.navigate', { url });
    await waitFor(startScreen, 'the start screen', 120000);
    check(await evaluate(`[...document.querySelectorAll('footer a, #start a')].filter((a) => /how to play/i.test(a.textContent))
        .every((a) => a.getAttribute('href') === 'manual.html' && a.target === '_blank')
        && document.querySelectorAll('footer a[href="manual.html"], #start a[href="manual.html"]').length === 2`),
        'game: "how to play" in the footer and on the start screen open manual.html in a new tab');
    check(await evaluate(`document.getElementById('menu').hidden`), 'game: no Menu button before playing');
    await screenshot('game-start');

    await evaluate(`document.getElementById('play').click()`);
    await sleep(1500);
    for (let i = 0; i < 12 && (await frameState()).state !== 'gameplay'; i++) {
        await key((await frameState()).menu === 'team_select' && i < 6 ? 'e' : 'Enter');
    }
    check((await frameState()).state === 'gameplay', 'game: the keyboard started a Classic game');
    await key('ArrowRight', 600);
    check(await visible('#menu'), 'game: Menu shows in the toolbar while playing');
    await screenshot('game-playing');
    const before = await recordings();
    let tabs = 0;
    while (tabs < 6 && !(await evaluate(`document.activeElement === document.getElementById('menu')`))) {
        await key('Tab', 30);
        tabs++;
    }
    check(await evaluate(`document.activeElement === document.getElementById('menu')`), `game: Tab from the game reaches Menu (${tabs} presses)`);
    const playing = await frameState();
    dialogs.length = 0;
    await reloadsToStart(() => key('Enter'), 'the start screen after Menu');
    check(dialogs.length === 1 && dialogs[0].startsWith('confirm:'), `game: Menu in a live game asks first (${dialogs.join(' | ')})`);
    const session = await evaluate(`Module.FS.readFile(REPLAY_DIR + '/' + ${JSON.stringify((await recordings().catch(() => [])).at(-1) ?? '')}, { encoding: 'utf8' })`).catch(() => '');
    const inMatch = session.slice(session.search(/^#@ mark frame=\d+ state=gameplay/m));
    check(playing.state === 'gameplay' && /^\d+ down Right$/m.test(inMatch) && !/^\d+ (down|up) (Tab|Return)$/m.test(inMatch),
        'game: the recording holds the match input but not the Tab and Enter that pressed Menu');
    const after = await recordings();
    const rows = await matchRows();
    check(after.length === before.length && after.at(-1) === before.at(-1) && rows.length === 1 && /^Match 1 — Classic, .*unfinished$/.test(rows[0]),
        `game: back on the start screen, which lists the match just played (${rows.join(' | ')})`);
    check(await evaluate(`/^#@ checkpoint match=1 /m.test(Module.FS.readFile(REPLAY_DIR + '/' + ${JSON.stringify(after.at(-1))}, { encoding: 'utf8' }))`),
        'game: the session recording was saved with its checkpoint');
    check(await evaluate(`document.getElementById('menu').hidden && document.activeElement === document.getElementById('play')`), 'game: Menu hidden again, Play focused');
    await screenshot('game-back-at-start');

    // Watch the clip to its end, then Menu.
    await evaluate(`[...document.querySelectorAll('#matchList button')].find((b) => b.textContent === 'Watch').click()`);
    await waitFor('window.smwQuit === true', 'the clip to end', 90000);
    await waitFor(`!document.getElementById('end').hidden`, 'the end screen', 5000);
    check(await evaluate(`document.getElementById('endText').textContent === 'Replay finished' && document.getElementById('playAgain').textContent === 'Menu'
        && !document.getElementById('watchAgain').hidden`), 'game: a finished clip offers Menu and Watch again');
    await screenshot('game-clip-finished');
    dialogs.length = 0;
    await reloadsToStart(() => evaluate(`document.getElementById('playAgain').click()`), 'the start screen after a finished clip');
    check(dialogs.length === 0 && (await matchRows()).length === 1 && (await recordings()).at(-1) === after.at(-1),
        'game: Menu after the clip returns to the same list, no confirm');

    // Watch again from the list, and leave mid-clip.
    await evaluate(`[...document.querySelectorAll('#matchList button')].find((b) => b.textContent === 'Watch').click()`);
    await sleep(300);
    check(await evaluate(`!window.smwQuit && document.getElementById('start').hidden`), 'game: a second Watch plays without a manual refresh');
    await reloadsToStart(() => evaluate(`document.getElementById('menu').click()`), 'the start screen after Menu mid-clip');
    check(dialogs.length === 0 && (await matchRows()).length === 1, 'game: Menu mid-clip returns to the list without a confirm');

    // A phone in landscape: the touch layout's Menu button.
    await phone(844, 390, 90);
    await send('Page.reload');
    await waitFor(startScreen, 'the start screen on a phone', 120000);
    check(await evaluate(`document.documentElement.classList.contains('touch')`), 'phone: touch controls on');
    check(!(await visible('#touch [data-action=menu]')), 'phone: no touch Menu button before playing');
    await tap(await center('#matchList button'));
    await sleep(300);
    check(await visible('#touch [data-action=menu]'), 'phone: the touch Menu button shows while watching');
    await screenshot('phone-watching');
    dialogs.length = 0;
    await reloadsToStart(async () => tap(await center('#touch [data-action=menu]')), 'the start screen after the touch Menu');
    check(dialogs.length === 0 && (await matchRows()).length === 1, 'phone: touch Menu mid-clip returns to the list');

    const count = (await recordings()).length;
    await tap(await center('#play'));
    await sleep(2000);
    await screenshot('phone-playing');
    await reloadsToStart(async () => tap(await center('#touch [data-action=menu]')), 'the start screen after the touch Menu in a live game');
    check(dialogs.length === 1 && dialogs[0].startsWith('confirm:'), 'phone: touch Menu in a live game asks first');
    check((await recordings()).length === count + 1, 'phone: the new session was recorded and saved');
    await screenshot('phone-back-at-start');

    await phone(390, 844, 0);
    await send('Page.reload');
    await waitFor(startScreen, 'the start screen on a portrait phone', 120000);
    await tap(await center('#play'));
    await sleep(1000);
    check(await visible('#touch [data-action=menu]'), 'phone, portrait: the touch Menu button shows');
    await screenshot('phone-portrait-playing');
} catch (e) {
    console.error(e.message);
    console.error(`last console lines:\n${consoleLines.slice(-15).join('\n')}`);
    failures++;
    await screenshot('error').catch(() => {});
}

console.log(failures ? `${failures} check(s) failed` : 'all checks passed');
process.exit(failures ? 1 : 0);
