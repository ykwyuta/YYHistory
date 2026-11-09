import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
import assert from 'node:assert/strict';

// Exercise the production input binding with browser events and a controlled clock.
const main = readFileSync(new URL('../web/main.js', import.meta.url), 'utf8');
const binding = main.slice(main.indexOf('function bindInput()'), main.indexOf('// 画面の大きさが変わったら描く領域を合わせる。'));
function setup() {
  let now = 0;
  let kind = 3;
  let next = 1;
  const frames = new Map();
  const events = [];
  const listeners = {};
  const surface = (name) => ({
    clientWidth: 800,
    addEventListener(type, fn) { (listeners[`${name}:${type}`] ??= []).push(fn); },
    getBoundingClientRect: () => ({ left: 0, top: 0, width: 800, height: 400 }),
    setPointerCapture() {},
    focus() {},
  });
  const canvas = surface('canvas');
  const window = surface('window');
  vm.runInNewContext(`${binding}\nbindInput();`, {
    uiCanvas: canvas, window, document: {}, screen: {}, navigator: {},
    pw: 1600, ph: 800, held: new Set(), pointerInside: true,
    wasm: { yy_hit: () => kind, yy_event: (...args) => events.push(args) },
    performance: { now: () => now },
    requestAnimationFrame: fn => { const id = next++; frames.set(id, fn); return id; },
    cancelAnimationFrame: id => frames.delete(id),
    setTimeout: () => next++, clearTimeout() {}, checkSize() {},
    vkOf: () => 27, shouldPrevent: () => true, historySearch: {},
  });
  const emit = (type, x, y, extra = {}, target = 'canvas') => {
    for (const fn of listeners[`${target}:${type}`] ?? []) fn({
      pointerType: 'touch', pointerId: 1, clientX: x, clientY: y,
      preventDefault() {}, ...extra,
    });
  };
  const tick = ms => {
    now += ms;
    const pending = [...frames.values()];
    frames.clear();
    for (const fn of pending) fn(now);
  };
  const swipe = () => {
    emit('pointerdown', 200, 300);
    tick(16);
    emit('pointermove', 200, 260);
    tick(16);
    emit('pointermove', 200, 220);
  };
  return { emit, tick, swipe, events, frames, setKind: value => { kind = value; } };
}

test('a card tap activates once; a swipe sends scaled scroll without activating', () => {
  const s = setup();
  s.emit('pointerdown', 200, 300);
  s.emit('pointerup', 200, 300);
  assert.equal(s.events.filter(e => e[0] === 1 && e[1] === 0).length, 1);
  s.events.length = 0;
  s.swipe();
  s.emit('pointerup', 200, 220);
  const deltas = s.events.filter(e => e[0] === 7).map(e => e[1]);
  assert.equal(deltas.length, 2);
  assert.ok(deltas.every(d => Math.abs(d + 80) < 1e-8));
  assert.equal(s.events.filter(e => e[0] === 1).length, 0);
});

test('a fling coasts, decays, and stops', () => {
  const s = setup();
  s.swipe();
  s.emit('pointerup', 200, 220);
  s.tick(16);
  const first = s.events.at(-1)[1];
  s.tick(16);
  assert.ok(first < 0 && Math.abs(s.events.at(-1)[1]) < Math.abs(first));
  for (let i = 0; i < 120; i++) s.tick(16);
  assert.equal(s.frames.size, 0);
});

test('pausing before release and pointer cancellation do not fling or tap', () => {
  for (const end of ['pointerup', 'pointercancel']) {
    const s = setup();
    s.swipe();
    s.tick(100);
    s.emit(end, 200, 220);
    assert.equal(s.frames.size, 0);
    assert.equal(s.events.filter(e => e[0] === 1).length, 0);
  }
});

test('new touches, mouse, keyboard, wheel, blur, and resize interrupt a fling', () => {
  for (const [target, type, extra] of [
    ['canvas', 'pointerdown', {}], ['canvas', 'pointerdown', { pointerType: 'mouse', button: 0 }],
    ['canvas', 'wheel', { deltaY: 10 }], ['window', 'keydown', { key: 'Escape' }],
    ['window', 'blur', {}], ['window', 'resize', {}],
  ]) {
    const s = setup();
    s.swipe();
    s.emit('pointerup', 200, 220);
    assert.equal(s.frames.size, 1);
    s.emit(type, 200, 220, extra, target);
    assert.equal(s.frames.size, 0, type);
  }
});

test('a changed panel or suspended frame stops the fling', () => {
  for (const suspended of [false, true]) {
    const s = setup();
    s.swipe();
    s.emit('pointerup', 200, 220);
    const count = s.events.length;
    if (!suspended) s.setKind(1);
    s.tick(suspended ? 200 : 16);
    assert.equal(s.events.length, count);
    assert.equal(s.frames.size, 0);
  }
});

test('adding a second finger cancels a scroll without tapping or coasting', () => {
  const s = setup();
  s.swipe();
  s.emit('pointerdown', 300, 220, { pointerId: 2 });
  s.emit('pointerup', 300, 220, { pointerId: 2 });
  s.emit('pointerup', 200, 220);
  assert.equal(s.frames.size, 0);
  assert.equal(s.events.filter(e => e[0] === 1 && e[1] === 0).length, 0);
});

test('coasting covers the same distance at different display refresh rates', () => {
  const distance = interval => {
    const s = setup();
    s.swipe();
    s.emit('pointerup', 200, 220);
    s.events.length = 0;
    for (let elapsed = 0; elapsed < 480; elapsed += interval) s.tick(interval);
    return s.events.filter(e => e[0] === 7).reduce((total, e) => total + e[1], 0);
  };
  assert.ok(Math.abs(distance(8) - distance(16)) < 1e-8);
});
