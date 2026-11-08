// Keep only the latest guide; scene changes and replay interrupt stale speech.
export function createNarrator(synth, Utterance) {
  let unlocked = false;
  let pending = null;
  let current = null; // Keep a reference until completion (also needed by Safari).
  const supported = !!synth && typeof Utterance === 'function';
  function stop() {
    pending = null;
    current = null;
    if (supported) synth.cancel();
  }
  function speak({ text, lang }, silent = false) {
    synth.cancel();
    const utterance = new Utterance(text);
    if (lang) utterance.lang = lang;
    utterance.rate = 0.85;
    if (silent) utterance.volume = 0;
    const voices = synth.getVoices();
    if (lang) {
      utterance.voice = voices.find(v => v.lang.toLowerCase() === lang.toLowerCase())
        || voices.find(v => v.lang.split('-')[0] === lang.split('-')[0]) || null;
    }
    utterance.onend = utterance.onerror = () => {
      if (current === utterance) current = null;
    };
    current = utterance;
    synth.speak(utterance);
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
