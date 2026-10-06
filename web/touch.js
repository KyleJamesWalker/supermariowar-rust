// On-screen controls for touch screens. They press player 1's default keys, which the game keeps on player 1
// while they are on, by dispatching keyboard events on window, where SDL's Emscripten port listens.
(function () {
  'use strict';

  var PREF_KEY = 'smw-touch-controls';
  var STICK_KEY = 'smw-touch-stick';
  var STICK_RADIUS = 40;
  var STICK_DEAD = 12;
  var STICK_BASE = 50;
  var STICK_KNOB = 26;
  var KEYS = {
    left: { key: 'ArrowLeft', code: 'ArrowLeft', keyCode: 37 },
    up: { key: 'ArrowUp', code: 'ArrowUp', keyCode: 38 },
    right: { key: 'ArrowRight', code: 'ArrowRight', keyCode: 39 },
    down: { key: 'ArrowDown', code: 'ArrowDown', keyCode: 40 },
    // SDL takes the scancode from `code` and the keycode from `keyCode` plus `location` (2 = right).
    run: { key: 'Control', code: 'ControlRight', keyCode: 17, location: 2 },
    item: { key: 'Shift', code: 'ShiftRight', keyCode: 16, location: 2 },
    start: { key: 'Enter', code: 'Enter', keyCode: 13 },
    back: { key: 'Escape', code: 'Escape', keyCode: 27 }
  };

  var root = document.documentElement;
  var held = {};
  var pointers = {};

  var send = function (name, down) {
    var k = KEYS[name];
    var ev = new KeyboardEvent(down ? 'keydown' : 'keyup', {
      key: k.key,
      code: k.code,
      location: k.location || 0,
      ctrlKey: !!held.run,
      shiftKey: !!held.item,
      bubbles: true,
      cancelable: true
    });
    // KeyboardEventInit has no keyCode or which, and SDL's keycode comes from keyCode.
    Object.defineProperty(ev, 'keyCode', { value: k.keyCode });
    Object.defineProperty(ev, 'which', { value: k.keyCode });
    window.dispatchEvent(ev);
  };

  var controls = document.createElement('div');
  controls.id = 'touch';
  controls.setAttribute('aria-hidden', 'true');
  controls.innerHTML =
    '<div class="tc-zone tc-stick"></div>' +
    '<div class="tc-stick-base" hidden><div class="tc-stick-knob"></div></div>' +
    '<div class="tc-zone tc-dpad">' +
      '<div class="tc-arm" data-dir="up"></div><div class="tc-arm" data-dir="down"></div>' +
      '<div class="tc-arm" data-dir="left"></div><div class="tc-arm" data-dir="right"></div>' +
    '</div>' +
    '<div class="tc-zone tc-buttons">' +
      '<div class="tc-btn" data-key="item">Item</div>' +
      '<div class="tc-btn" data-key="run">Run</div>' +
      '<div class="tc-btn" data-key="up">Jump</div>' +
    '</div>' +
    '<div class="tc-col tc-col-left">' +
      '<div class="tc-zone tc-key" data-key="back">Back</div>' +
      '<button class="tc-util" type="button" data-action="fullscreen" title="Fullscreen">' +
        '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">' +
        '<path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"/></svg></button>' +
      '<button class="tc-util tc-mode" type="button" data-action="mode" title="Switch between D-pad and floating stick"></button>' +
      '<button class="tc-util" type="button" data-action="menu" title="Menu">' +
        '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">' +
        '<path d="M4 6h16M4 12h16M4 18h16"/></svg></button>' +
    '</div>' +
    '<div class="tc-col tc-col-right">' +
      '<div class="tc-zone tc-key" data-key="start">Start</div>' +
      '<button class="tc-util" type="button" data-action="hide" title="Hide touch controls">' +
        '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">' +
        '<path d="M6 6l12 12M18 6L6 18"/></svg></button>' +
    '</div>';
  var dpad = controls.querySelector('.tc-dpad');
  var stickZone = controls.querySelector('.tc-stick');
  var stickBase = controls.querySelector('.tc-stick-base');
  var stickKnob = controls.querySelector('.tc-stick-knob');
  var keyElements = controls.querySelectorAll('[data-key]');
  var arms = controls.querySelectorAll('.tc-arm');

  var render = function () {
    Array.prototype.forEach.call(keyElements, function (el) {
      el.classList.toggle('on', pointerHolds(el.dataset.key, 'keys'));
    });
    Array.prototype.forEach.call(arms, function (el) {
      el.classList.toggle('on', pointerHolds(el.dataset.dir, 'dpad'));
    });
    var stick = null;
    Object.keys(pointers).forEach(function (id) { if (pointers[id].zone === 'stick') stick = pointers[id]; });
    stickBase.hidden = !stick;
    if (stick) {
      var dx = stick.x - stick.origin.x;
      var dy = stick.y - stick.origin.y;
      var scale = Math.min(1, STICK_RADIUS / (Math.hypot(dx, dy) || 1));
      var knob = inStickZone({ x: stick.origin.x + dx * scale, y: stick.origin.y + dy * scale }, STICK_KNOB);
      stickBase.style.transform = 'translate(' + stick.origin.x + 'px, ' + stick.origin.y + 'px)';
      stickKnob.style.transform = 'translate(' + (knob.x - stick.origin.x) + 'px, ' + (knob.y - stick.origin.y) + 'px)';
    }
  };
  var pointerHolds = function (name, zone) {
    return Object.keys(pointers).some(function (id) {
      return pointers[id].zone === zone && pointers[id].keys.indexOf(name) >= 0;
    });
  };

  var setKeys = function (p, keys) {
    var pressed = false;
    p.keys.forEach(function (name) {
      if (keys.indexOf(name) < 0 && --held[name] === 0) send(name, false);
    });
    keys.forEach(function (name) {
      if (p.keys.indexOf(name) >= 0) return;
      held[name] = (held[name] || 0) + 1;
      if (held[name] === 1) {
        send(name, true);
        pressed = true;
      }
    });
    p.keys = keys;
    if (pressed && navigator.vibrate) {
      try { navigator.vibrate(10); } catch (e) {}
    }
  };

  var dpadKeys = function (x, y) {
    var r = dpad.getBoundingClientRect();
    return directionKeys(x - (r.left + r.width / 2), y - (r.top + r.height / 2), r.width * 0.12);
  };
  var directionKeys = function (dx, dy, dead) {
    if (Math.hypot(dx, dy) < dead) return [];
    var angle = Math.atan2(dy, dx) * 180 / Math.PI;
    var keys = [];
    if (Math.abs(angle) < 67.5) keys.push('right');
    if (Math.abs(angle) > 112.5) keys.push('left');
    if (angle > 22.5 && angle < 157.5) keys.push('down');
    if (angle < -22.5 && angle > -157.5) keys.push('up');
    return keys;
  };

  // A thumb between two buttons presses both, and sliding onto another button presses it.
  var buttonKeys = function (x, y) {
    var keys = [];
    Array.prototype.forEach.call(keyElements, function (el) {
      var r = el.getBoundingClientRect();
      var reach = Math.max(r.width, r.height) / 2 * 1.2;
      if (Math.hypot(x - (r.left + r.width / 2), y - (r.top + r.height / 2)) < reach && keys.indexOf(el.dataset.key) < 0) {
        keys.push(el.dataset.key);
      }
    });
    return keys;
  };

  // Keeps a circle of radius r around p inside the stick zone, so the stick never covers the game.
  var inStickZone = function (p, r) {
    var z = stickZone.getBoundingClientRect();
    var clamp = function (v, lo, hi) { return lo <= hi ? Math.min(Math.max(v, lo), hi) : (lo + hi) / 2; };
    return { x: clamp(p.x, z.left + r, z.right - r), y: clamp(p.y, z.top + r, z.bottom - r) };
  };

  // The floating stick's base appears under the thumb and follows it past STICK_RADIUS.
  var stickKeys = function (p, x, y) {
    var dx = x - p.origin.x;
    var dy = y - p.origin.y;
    var dist = Math.hypot(dx, dy);
    if (dist > STICK_RADIUS) {
      p.origin = { x: x - dx / dist * STICK_RADIUS, y: y - dy / dist * STICK_RADIUS };
    }
    p.origin = inStickZone(p.origin, STICK_BASE);
    p.x = x;
    p.y = y;
    return directionKeys(x - p.origin.x, y - p.origin.y, STICK_DEAD);
  };

  var track = function (e) {
    var p = pointers[e.pointerId];
    if (!p) return;
    var keys = p.zone === 'dpad' ? dpadKeys(e.clientX, e.clientY)
      : p.zone === 'stick' ? stickKeys(p, e.clientX, e.clientY) : buttonKeys(e.clientX, e.clientY);
    setKeys(p, keys);
    render();
  };
  var release = function (id) {
    var p = pointers[id];
    if (!p) return;
    setKeys(p, []);
    delete pointers[id];
    render();
  };
  var releaseAll = function () {
    Object.keys(pointers).forEach(release);
  };

  controls.addEventListener('pointerdown', function (e) {
    var zone = e.target.closest('.tc-buttons, .tc-key') ? 'keys'
      : stickMode() ? (e.target.closest('.tc-stick, .tc-dpad') ? 'stick' : null)
      : e.target.closest('.tc-dpad') ? 'dpad' : null;
    if (!zone) return;
    e.preventDefault();
    try { controls.setPointerCapture(e.pointerId); } catch (err) {}
    pointers[e.pointerId] = { zone: zone, type: e.pointerType, keys: [], origin: { x: e.clientX, y: e.clientY } };
    track(e);
  });
  controls.addEventListener('pointermove', track);
  controls.addEventListener('pointerup', function (e) { release(e.pointerId); });
  controls.addEventListener('pointercancel', function (e) { release(e.pointerId); });
  controls.addEventListener('lostpointercapture', function (e) {
    if (e.target === controls) release(e.pointerId);
  });
  controls.addEventListener('touchstart', function (e) {
    if (e.target.closest('.tc-zone')) e.preventDefault();
  }, { passive: false });
  controls.addEventListener('touchend', function (e) {
    if (!e.touches.length) {
      Object.keys(pointers).forEach(function (id) { if (pointers[id].type !== 'mouse') release(id); });
    }
  });
  controls.addEventListener('touchcancel', releaseAll);
  controls.addEventListener('contextmenu', function (e) { e.preventDefault(); });
  window.addEventListener('blur', releaseAll);
  window.addEventListener('pagehide', releaseAll);
  document.addEventListener('visibilitychange', function () { if (document.hidden) releaseAll(); });

  var requestFullscreen = root.requestFullscreen || root.webkitRequestFullscreen;
  var fullscreenButton = controls.querySelector('[data-action=fullscreen]');
  fullscreenButton.hidden = !requestFullscreen;
  fullscreenButton.addEventListener('click', function () {
    if (document.fullscreenElement || document.webkitFullscreenElement) {
      (document.exitFullscreen || document.webkitExitFullscreen).call(document);
      return;
    }
    var entered = requestFullscreen.call(root, { navigationUI: 'hide' });
    if (entered && entered.then) {
      entered.then(function () {
        if (screen.orientation && screen.orientation.lock) return screen.orientation.lock('landscape');
      }).catch(function () {});
    }
  });

  controls.querySelector('[data-action=menu]').addEventListener('click', function () {
    var menu = document.getElementById('menu');
    if (menu) menu.click();
  });

  var readPref = function () {
    try { return localStorage.getItem(PREF_KEY); } catch (e) { return null; }
  };
  var writePref = function (value) {
    try { localStorage.setItem(PREF_KEY, value); } catch (e) {}
  };
  var coarse = window.matchMedia('(pointer: coarse)');
  var toggleLink = document.createElement('a');
  toggleLink.href = '#';
  var modeLink = document.createElement('a');
  modeLink.href = '#';
  var modeButton = controls.querySelector('[data-action=mode]');
  var stickMode = function () {
    try { return localStorage.getItem(STICK_KEY) === 'stick'; } catch (e) { return false; }
  };
  var applyMode = function () {
    var stick = stickMode();
    releaseAll();
    root.classList.toggle('stick', stick);
    modeButton.textContent = stick ? 'Stick' : 'D-pad';
    modeLink.textContent = stick ? 'use the touch D-pad' : 'use a floating touch stick';
  };
  var toggleMode = function (e) {
    e.preventDefault();
    try { localStorage.setItem(STICK_KEY, stickMode() ? 'dpad' : 'stick'); } catch (err) {}
    applyMode();
  };
  modeButton.addEventListener('click', toggleMode);
  modeLink.addEventListener('click', toggleMode);
  var enabled = function () {
    var pref = readPref();
    return pref === 'on' || (pref !== 'off' && coarse.matches);
  };
  var apply = function () {
    var on = enabled();
    if (!on) releaseAll();
    root.classList.toggle('touch', on);
    controls.hidden = !on;
    toggleLink.textContent = on ? 'hide touch controls' : 'show touch controls';
    modeLink.hidden = !on;
  };
  var toggle = function (e) {
    e.preventDefault();
    writePref(enabled() ? 'off' : 'on');
    apply();
  };
  toggleLink.addEventListener('click', toggle);
  controls.querySelector('[data-action=hide]').addEventListener('click', toggle);
  if (coarse.addEventListener) coarse.addEventListener('change', apply);
  else if (coarse.addListener) coarse.addListener(apply);
  document.addEventListener('gesturestart', function (e) {
    if (root.classList.contains('touch')) e.preventDefault();
  });

  var mount = function () {
    document.body.appendChild(controls);
    var footer = document.querySelector('footer');
    if (footer) footer.append(' · ', toggleLink, ' ', modeLink);
    var start = document.getElementById('start');
    if (start && !requestFullscreen && !navigator.standalone && !window.matchMedia('(display-mode: fullscreen), (display-mode: standalone)').matches) {
      var tip = document.createElement('div');
      tip.className = 'hint tc-tip';
      tip.textContent = 'For fullscreen, use Share → Add to Home Screen.';
      start.appendChild(tip);
    }
    applyMode();
    apply();
  };
  if (document.body) mount();
  else document.addEventListener('DOMContentLoaded', mount);
})();
