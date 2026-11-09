import test from 'node:test';
import assert from 'node:assert/strict';
import { createNarrator, bindNarratorGestures } from '../web/speech.mjs';
function setup() {
  const played = [];
  const primed = [];
  let cancelled = 0;
  class Utterance { constructor(text) { this.text = text; } }
  const voices = [{ lang: 'en-US' }, { lang: 'ja-JP' }, { lang: 'zh-TW' }];
  const synth = { cancel() { cancelled++; }, getVoices: () => voices,
    speak: u => (u.volume === 0 ? primed : played).push(u) };
  return { narrator: createNarrator(synth, Utterance), played, primed, voices, cancels: () => cancelled };
}
test('waits for interaction and retains only the newest guide', () => {
  const { narrator, played } = setup();
  narrator.say('old scene', 'en-US');
  narrator.say('new scene', 'ja-JP');
  assert.equal(played.length, 0);
  narrator.unlock();
  assert.equal(played.length, 1);
  assert.equal(played[0].text, 'new scene');
  narrator.unlock();
  assert.equal(played.length, 1);
});
test('uses the selected language and a slower speaking rate', () => {
  const { narrator, played, voices } = setup();
  narrator.unlock();
  narrator.say('こんにちは', 'ja-JP');
  assert.equal(played[0].voice, voices[1]);
  assert.equal(played[0].lang, 'ja-JP');
  assert.equal(played[0].rate, 0.85);
  narrator.say('你好', 'zh-CN');
  assert.equal(played[1].voice, voices[2]);
});
test('replay interrupts previous speech without appending stale guides', () => {
  const { narrator, played, cancels } = setup();
  narrator.unlock();
  narrator.say('guide', 'en-US');
  narrator.say('guide', 'en-US');
  assert.equal(played.length, 2);
  assert.equal(cancels(), 3);
});
test('turning off before interaction discards pending speech', () => {
  const { narrator, played } = setup();
  narrator.say('guide', 'ja-JP');
  narrator.say('', 'ja-JP');
  narrator.unlock();
  assert.equal(played.length, 0);
});
test('stop cancels current speech and clears pending content', () => {
  const { narrator, played, cancels } = setup();
  narrator.unlock();
  narrator.say('guide', 'ja-JP');
  narrator.stop();
  narrator.unlock();
  assert.equal(cancels(), 3);
  assert.equal(played.length, 1);
});
test('unsupported speech is harmless', () => {
  const narrator = createNarrator(undefined, undefined);
  assert.equal(narrator.supported, false);
  narrator.say('guide', 'ja-JP');
  narrator.unlock();
  narrator.stop();
});

test('an early tap primes iOS speech before a guide exists', () => {
  let inGesture = false;
  let allowed = false;
  const heard = [];
  class Utterance { constructor(text) { this.text = text; } }
  const synth = {
    cancel() {}, getVoices: () => [],
    speak(u) {
      if (inGesture) allowed = true;
      if (allowed && u.text) heard.push(u.text);
    },
  };
  const narrator = createNarrator(synth, Utterance);
  inGesture = true;
  narrator.unlock();
  inGesture = false;
  narrator.say('Guide from the next animation frame', 'ja-JP');
  assert.deepEqual(heard, ['Guide from the next animation frame']);
});

test('silent initialization happens once and never interrupts later guides', () => {
  const { narrator, played, primed } = setup();
  narrator.unlock();
  narrator.unlock();
  assert.equal(primed.length, 1);
  assert.equal(primed[0].text, '');
  assert.equal(played.length, 0);
  narrator.say('guide', 'ja-JP');
  narrator.unlock();
  assert.equal(played.length, 1);
});

test('trusted touch release unlocks speech without relying on pointerdown', () => {
  const listeners = new Map();
  const { narrator, played } = setup();
  bindNarratorGestures({ addEventListener(type, fn) { listeners.set(type, fn); } }, narrator);
  assert.equal(listeners.has('pointerdown'), false);
  narrator.say('iPad guide', 'ja-JP');
  listeners.get('touchend')({ isTrusted: false });
  assert.equal(played.length, 0);
  listeners.get('touchend')({ isTrusted: true });
  assert.equal(played[0].text, 'iPad guide');
  for (const type of ['mousedown', 'keydown', 'click']) {
    listeners.get(type)({ isTrusted: true });
  }
  assert.equal(played.length, 1);
});

test('long guides are spoken sentence by sentence, in order', () => {
  const { narrator, played, cancels } = setup();
  narrator.unlock();
  const sentence = 'カエサルは前44年3月15日に元老院の会議で暗殺された。';
  const text = sentence.repeat(10);
  narrator.say(text, 'ja-JP');
  assert.ok(played.length > 1);
  assert.equal(played.map(u => u.text).join(''), text);
  assert.ok(played.every(u => u.text.length <= 240 && u.lang === 'ja-JP' && u.rate === 0.85));
  const before = played.length;
  narrator.say('つぎ', 'ja-JP');
  assert.equal(played.length, before + 1);
  assert.ok(cancels() >= 2, 'a new guide cancels the rest of the old one');
});

test('splitting keeps short text whole and breaks very long sentences', async () => {
  const { splitSpeech } = await import('../web/speech.mjs');
  assert.deepEqual(splitSpeech('こんにちは。'), ['こんにちは。']);
  const long = 'あ、'.repeat(200);
  const parts = splitSpeech(long);
  assert.equal(parts.join(''), long);
  assert.ok(parts.every(p => p.length <= 240));
});
