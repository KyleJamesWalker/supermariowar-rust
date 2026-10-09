// Text entry on touch screens. While the game edits a text field (it calls Module.onTextField), an HTML input
// sits over the field so tapping it opens the on-screen keyboard. What is typed reaches the game as the key
// events a keyboard sends, so the menus and the session recording see ordinary key presses.
(function () {
  'use strict';

  var STEP_MS = 40;
  var SHIFTED_DIGITS = ')!@#$%^&*(';
  var PUNCTUATION = [
    ['-', '_', 'Minus', 189], ['=', '+', 'Equal', 187], ['`', '~', 'Backquote', 192],
    ['[', '{', 'BracketLeft', 219], ['\\', '|', 'Backslash', 220], [']', '}', 'BracketRight', 221],
    [';', ':', 'Semicolon', 186], ["'", '"', 'Quote', 222], [',', '<', 'Comma', 188],
    ['.', '>', 'Period', 190], ['/', '?', 'Slash', 191]
  ];
  var KEYS = {
    Backspace: { key: 'Backspace', code: 'Backspace', keyCode: 8 },
    Enter: { key: 'Enter', code: 'Enter', keyCode: 13 },
    Escape: { key: 'Escape', code: 'Escape', keyCode: 27 },
    ArrowLeft: { key: 'ArrowLeft', code: 'ArrowLeft', keyCode: 37 },
    ArrowRight: { key: 'ArrowRight', code: 'ArrowRight', keyCode: 39 },
    Shift: { key: 'Shift', code: 'ShiftLeft', keyCode: 16, location: 1 }
  };
  // The characters MI_TextField accepts, as US-layout keys plus Shift.
  var CHARS = { ' ': { key: ' ', code: 'Space', keyCode: 32 } };
  for (var i = 0; i < 26; i++) {
    var lower = String.fromCharCode(97 + i);
    var upper = lower.toUpperCase();
    CHARS[lower] = { key: lower, code: 'Key' + upper, keyCode: 65 + i };
    CHARS[upper] = { key: upper, code: 'Key' + upper, keyCode: 65 + i, shift: true };
  }
  for (var d = 0; d < 10; d++) {
    CHARS[String(d)] = { key: String(d), code: 'Digit' + d, keyCode: 48 + d };
    CHARS[SHIFTED_DIGITS[d]] = { key: SHIFTED_DIGITS[d], code: 'Digit' + d, keyCode: 48 + d, shift: true };
  }
  PUNCTUATION.forEach(function (p) {
    CHARS[p[0]] = { key: p[0], code: p[2], keyCode: p[3] };
    CHARS[p[1]] = { key: p[1], code: p[2], keyCode: p[3], shift: true };
  });

  var root = document.documentElement;
  var input = document.createElement('input');
  input.id = 'textEntry';
  input.type = 'text';
  input.hidden = true;
  input.setAttribute('autocomplete', 'off');
  input.setAttribute('autocorrect', 'off');
  input.setAttribute('autocapitalize', 'off');
  input.setAttribute('spellcheck', 'false');
  input.setAttribute('enterkeyhint', 'done');
  input.setAttribute('aria-label', 'Text entry');

  var field = null;
  var model = { value: '', cursor: 0 };
  var queue = [];
  var pumping = false;
  var lastSent = 0;
  var closing = false;
  var keyboardOpen = false;

  var dispatch = function (k, down) {
    var ev = new KeyboardEvent(down ? 'keydown' : 'keyup', {
      key: k.key, code: k.code, location: k.location || 0, shiftKey: !!k.shift, bubbles: true, cancelable: true
    });
    Object.defineProperty(ev, 'keyCode', { value: k.keyCode });
    Object.defineProperty(ev, 'which', { value: k.keyCode });
    // From the canvas, so the page's own key listeners see an element target; SDL listens on window.
    document.getElementById('canvas').dispatchEvent(ev);
  };
  // The game reads one pressed key per frame and Shift from the keyboard state, so each press is held for a step.
  var press = function (k) {
    if (k.shift) queue.push([KEYS.Shift, true]);
    queue.push([k, true], null, [k, false]);
    if (k.shift) queue.push([KEYS.Shift, false]);
    queue.push(null);
    pump();
  };
  var pump = function () {
    if (pumping) return;
    pumping = true;
    var next = function () {
      while (queue.length && queue[0]) {
        var e = queue.shift();
        dispatch(e[0], e[1]);
      }
      lastSent = Date.now();
      if (!queue.length) {
        pumping = false;
        return;
      }
      queue.shift();
      setTimeout(next, STEP_MS);
    };
    next();
  };

  var allowed = function (c) {
    return CHARS[c] && field.disallowed.indexOf(c) < 0;
  };
  var showModel = function () {
    if (input.value !== model.value) input.value = model.value;
    try { input.setSelectionRange(model.cursor, model.cursor); } catch (e) {}
  };

  // Turns the input's new text into cursor moves, Backspaces and characters for the game's field.
  var onInput = function () {
    if (!field) return;
    var before = model.value;
    var after = input.value;
    var p = 0;
    while (p < before.length && p < after.length && before[p] === after[p]) p++;
    var s = 0;
    while (s < before.length - p && s < after.length - p && before[before.length - 1 - s] === after[after.length - 1 - s]) s++;
    var removed = before.length - p - s;
    var inserted = after.slice(p, after.length - s);
    var target = p + removed;
    for (; model.cursor < target; model.cursor++) press(KEYS.ArrowRight);
    for (; model.cursor > target; model.cursor--) press(KEYS.ArrowLeft);
    for (var r = 0; r < removed; r++) press(KEYS.Backspace);
    model.value = before.slice(0, p) + before.slice(p + removed);
    model.cursor = p;
    for (var j = 0; j < inserted.length; j++) {
      var c = inserted[j];
      if (!allowed(c) || model.value.length >= field.max) continue;
      press(CHARS[c]);
      model.value = model.value.slice(0, model.cursor) + c + model.value.slice(model.cursor);
      model.cursor++;
    }
    if (input.value !== model.value) showModel();
  };

  var finish = function (k) {
    if (!field || closing) return;
    closing = true;
    press(k);
  };

  var place = function () {
    if (!field) return;
    var canvas = document.getElementById('canvas');
    var r = canvas.getBoundingClientRect();
    var scale = r.width / 640;
    var h = 32 * scale;
    var top = r.top + field.y * scale;
    var vv = window.visualViewport;
    if (vv) top = Math.max(vv.offsetTop + 4, Math.min(top, vv.offsetTop + vv.height - h - 4));
    input.style.left = (r.left + (field.x + field.indent) * scale) + 'px';
    input.style.top = top + 'px';
    input.style.width = ((field.width - field.indent) * scale) + 'px';
    input.style.height = h + 'px';
    input.style.fontSize = Math.max(16, 18 * scale) + 'px';
  };

  var close = function () {
    field = null;
    // Drop presses still waiting, but release what is down, or SDL keeps it held.
    var ups = queue.filter(function (e) { return !e || !e[1]; });
    queue.length = 0;
    Array.prototype.push.apply(queue, ups);
    closing = false;
    input.hidden = true;
    if (document.activeElement === input) input.blur();
  };

  window.Module = window.Module || {};
  Module.onTextField = function (f) {
    if (!f.active) {
      close();
      return;
    }
    if (!root.classList.contains('touch')) return;
    var opening = !field;
    field = f;
    if (opening || (!queue.length && Date.now() - lastSent > 300)) {
      model = { value: f.value, cursor: f.cursor };
      showModel();
    }
    if (opening) {
      closing = false;
      input.hidden = false;
      place();
      input.focus({ preventScroll: true });
    }
  };

  input.addEventListener('input', onInput);
  // Keys typed into the input reach the game only as the events above; Enter confirms and Escape cancels.
  ['keydown', 'keypress', 'keyup'].forEach(function (type) {
    input.addEventListener(type, function (e) {
      e.stopPropagation();
      if (type !== 'keydown') return;
      if (e.key === 'Enter') {
        e.preventDefault();
        finish(KEYS.Enter);
      } else if (e.key === 'Escape') {
        e.preventDefault();
        finish(KEYS.Escape);
      }
    });
  });
  input.addEventListener('blur', function () { finish(KEYS.Enter); });
  // Hiding the on-screen keyboard (Android's Back) leaves the input focused; treat it as done.
  if (window.visualViewport) {
    window.visualViewport.addEventListener('resize', function () {
      var open = window.visualViewport.height < window.innerHeight * 0.8;
      if (keyboardOpen && !open && field) input.blur();
      keyboardOpen = open;
      place();
    });
    window.visualViewport.addEventListener('scroll', place);
  }
  window.addEventListener('resize', place);

  var mount = function () { document.body.appendChild(input); };
  if (document.body) mount();
  else document.addEventListener('DOMContentLoaded', mount);
})();
