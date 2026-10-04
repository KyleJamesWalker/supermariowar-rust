// On-screen controls for touch screens. They press player 1's default keys by dispatching
// keyboard events on window, where SDL's Emscripten port listens.
(function () {
  'use strict';

  var PREF_KEY = 'smw-touch-controls';
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
    '</div>' +
    '<div class="tc-col tc-col-right">' +
      '<div class="tc-zone tc-key" data-key="start">Start</div>' +
      '<button class="tc-util" type="button" data-action="hide" title="Hide touch controls">' +
        '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">' +
        '<path d="M6 6l12 12M18 6L6 18"/></svg></button>' +
    '</div>';
  var dpad = controls.querySelector('.tc-dpad');
  var keyElements = controls.querySelectorAll('[data-key]');
  var arms = controls.querySelectorAll('.tc-arm');

  var render = function () {
    Array.prototype.forEach.call(keyElements, function (el) {
      el.classList.toggle('on', pointerHolds(el.dataset.key, 'keys'));
    });
    Array.prototype.forEach.call(arms, function (el) {
      el.classList.toggle('on', pointerHolds(el.dataset.dir, 'dpad'));
    });
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
    var dx = x - (r.left + r.width / 2);
    var dy = y - (r.top + r.height / 2);
    if (Math.hypot(dx, dy) < r.width * 0.12) return [];
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

  var track = function (e) {
    var p = pointers[e.pointerId];
    if (!p) return;
    setKeys(p, p.zone === 'dpad' ? dpadKeys(e.clientX, e.clientY) : buttonKeys(e.clientX, e.clientY));
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
    var zone = e.target.closest('.tc-dpad') ? 'dpad' : e.target.closest('.tc-buttons, .tc-key') ? 'keys' : null;
    if (!zone) return;
    e.preventDefault();
    try { controls.setPointerCapture(e.pointerId); } catch (err) {}
    pointers[e.pointerId] = { zone: zone, type: e.pointerType, keys: [] };
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

  var readPref = function () {
    try { return localStorage.getItem(PREF_KEY); } catch (e) { return null; }
  };
  var writePref = function (value) {
    try { localStorage.setItem(PREF_KEY, value); } catch (e) {}
  };
  var coarse = window.matchMedia('(pointer: coarse)');
  var toggleLink = document.createElement('a');
  toggleLink.href = '#';
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
    if (footer) footer.append(' · ', toggleLink);
    var start = document.getElementById('start');
    if (start && !requestFullscreen && !navigator.standalone && !window.matchMedia('(display-mode: fullscreen), (display-mode: standalone)').matches) {
      var tip = document.createElement('div');
      tip.className = 'hint tc-tip';
      tip.textContent = 'For fullscreen, use Share → Add to Home Screen.';
      start.appendChild(tip);
    }
    apply();
  };
  if (document.body) mount();
  else document.addEventListener('DOMContentLoaded', mount);
})();
