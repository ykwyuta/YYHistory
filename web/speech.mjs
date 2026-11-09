// Keep only the latest guide; scene changes and replay interrupt stale speech.
// Split a guide into sentences no longer than about LIMIT characters.
const LIMIT = 120;
export function splitSpeech(text) {
  if (text.length <= LIMIT) return [text];
  const parts = [];
  let part = '';
  for (const sentence of text.match(/[^。！？!?]+[。！？!?」』）)]*|[。！？!?]+/g) || [text]) {
    if (part && part.length + sentence.length > LIMIT) {
      parts.push(part);
      part = '';
    }
    part += sentence;
    while (part.length > LIMIT * 2) {
      // A very long sentence: break at a comma, or hard at the limit.
      const at = Math.max(part.lastIndexOf('、', LIMIT), part.lastIndexOf('，', LIMIT));
      const cut = at > 0 ? at + 1 : LIMIT;
      parts.push(part.slice(0, cut));
      part = part.slice(cut);
    }
  }
  if (part) parts.push(part);
  return parts;
}

export function createNarrator(synth, Utterance) {
  let unlocked = false;
  let pending = null;
  let current = null; // The queued utterances of the current guide.
  const supported = !!synth && typeof Utterance === 'function';
  function stop() {
    pending = null;
    current = null;
    if (supported) synth.cancel();
  }
  function speak({ text, lang }, silent = false) {
    synth.cancel();
    const voices = synth.getVoices();
    const voice = lang
      ? voices.find(v => v.lang.toLowerCase() === lang.toLowerCase())
        || voices.find(v => v.lang.split('-')[0] === lang.split('-')[0]) || null
      : null;
    // Long guides are queued sentence by sentence: some engines stop a single
    // long utterance part-way, and short parts start sooner on iPad.
    const parts = splitSpeech(text);
    const queue = parts.map((part) => {
      const utterance = new Utterance(part);
      if (lang) utterance.lang = lang;
      utterance.rate = 0.85;
      if (silent) utterance.volume = 0;
      if (lang) utterance.voice = voice;
      return utterance;
    });
    current = queue; // Keep references until completion (also needed by Safari).
    const last = queue[queue.length - 1];
    last.onend = last.onerror = () => {
      if (current === queue) current = null;
    };
    for (const utterance of queue) synth.speak(utterance);
  }
  function play() {
    if (!supported || !unlocked || !pending) return;
    const guide = pending;
    pending = null;
    speak(guide);
  }
  return {
    supported,
    stop,
    unlock() {
      if (!supported || unlocked) return;
      // iOS removes its restriction only when speak() runs inside a user
      // gesture. A flag alone cannot unlock guides produced by a later frame,
      // including when the first tap happens while the game is still loading.
      unlocked = true;
      if (pending) play();
      else speak({ text: '' }, true);
    },
    say(text, lang) {
      if (!text) { stop(); return; }
      pending = { text, lang };
      play();
    },
  };
}

export function bindNarratorGestures(target, narrator) {
  const unlock = (event) => {
    if (event.isTrusted) narrator.unlock();
  };
  // Touch pointerdown is too early for Safari's user-gesture recognition.
  // Keep touchend for older iPads and click for keyboard/assistive activation.
  for (const type of ['mousedown', 'touchend', 'keydown', 'click']) {
    target.addEventListener(type, unlock, { capture: true, passive: true });
  }
}
