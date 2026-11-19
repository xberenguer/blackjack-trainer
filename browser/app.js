/* Platform glue only. Rules, screens, holds, chords and persistence codec are Rust. */
(async () => {
  'use strict';
  const canvas = document.getElementById('screen');
  const status = document.getElementById('status');
  const ctx = canvas.getContext('2d', { alpha: false });
  const image = ctx.createImageData(240, 135);
  ctx.imageSmoothingEnabled = false;
  const text = new TextDecoder();
  const sources = new Map();
  const { JournalHost } = await import('./journal-host.js');
  const { journalCodec } = await import('./journal-codec.js');
  let wasm, journal, audio, active, activeCue = 0, lastStatus = '', lastVolume = 20, lastSoundMask = 7;
  let busy = false, lastPresented, lastRetry = 0;
  const inputs = [];
  const now = () => Math.floor(performance.now()) >>> 0;
  const bytes = () => new Uint8Array(wasm.memory.buffer);
  const downmask = () => [...sources.values()].reduce((mask, bit) => mask | bit, 0);
  const rawInfo = () => ({
    version: text.decode(bytes().subarray(wasm.sb_version(), wasm.sb_version() + wasm.sb_version_len())), implementation: 'Rust',
    view: text.decode(bytes().subarray(wasm.sb_view(), wasm.sb_view() + wasm.sb_view_len())),
    menu_item: text.decode(bytes().subarray(wasm.sb_menu_item(), wasm.sb_menu_item() + wasm.sb_menu_item_len())),
    focus: wasm.sb_focus(), phase: wasm.sb_phase(), training_mode: wasm.sb_mode(),
    rounds: wasm.sb_rounds(), chips: Number(wasm.sb_bankroll()), wager: Number(wasm.sb_wager()),
    brightness: wasm.sb_brightness(), flipped: !!wasm.sb_flipped(), sound: !!wasm.sb_sound_enabled(), volume: wasm.sb_audio_volume(), sound_categories: wasm.sb_audio_mask(), fast: !!wasm.sb_fast_mode()
  });
  const info = () => wasm.sb_store_output_pending() && lastPresented ? {...lastPresented,view:wasm.sb_store_output_blocked()?'StorageRetry':lastPresented.view,focus:wasm.sb_store_output_blocked()?wasm.sb_focus():lastPresented.focus,menu_item:wasm.sb_store_output_blocked()?text.decode(bytes().subarray(wasm.sb_menu_item(),wasm.sb_menu_item()+wasm.sb_menu_item_len())):''} : rawInfo();
  function stop() {
    if (active) { try { active.stop(); } catch {} }
    active = undefined;
    activeCue = 0;
  }
  function unlock() {
    try {
      const Context = window.AudioContext || window.webkitAudioContext;
      if (!audio && Context) audio = new Context();
      if (audio?.state === 'suspended') audio.resume().catch(() => {});
    } catch { audio = undefined; }
  }
  function pumpAudio(enabled) {
    const volume = wasm.sb_audio_volume(), mask = wasm.sb_audio_mask();
    if (volume !== lastVolume || mask !== lastSoundMask) { stop(); lastVolume = volume; lastSoundMask = mask; }
    if (!enabled) {
      stop();
      while (wasm.sb_sound()) { /* Muting also discards pending playback. */ }
      return;
    }
    if (!audio || audio.state !== 'running' || active) return;
    const cue = wasm.sb_sound();
    if (!cue) return;
    const n = wasm.sb_audio_samples(cue), rate = wasm.sb_audio_rate(cue), ptr = wasm.sb_audio_data(cue);
    if (!n || !rate || !ptr) return;
    try {
      const buffer = audio.createBuffer(1, n, rate);
      const channel = buffer.getChannelData(0);
      for (let i = 0; i < n; i++) channel[i] = wasm.sb_audio_playback_sample(cue,i) / 32768;
      const source = audio.createBufferSource(), gain = audio.createGain();
      source.buffer = buffer;
      gain.gain.value = 1;
      source.connect(gain).connect(audio.destination);
      active = source;
      activeCue = cue;
      source.onended = () => { if (active === source) { active = undefined; activeCue = 0; } };
      source.start();
    } catch { stop(); }
  }
  function present() {
    const src = new Uint16Array(wasm.memory.buffer, wasm.sb_frame(), 240 * 135);
    for (let i = 0; i < src.length; i++) {
      const c = src[i], p = i * 4;
      image.data[p] = ((c >>> 11) & 31) * 255 / 31;
      image.data[p + 1] = ((c >>> 5) & 63) * 255 / 63;
      image.data[p + 2] = (c & 31) * 255 / 31;
      image.data[p + 3] = 255;
    }
    ctx.putImageData(image, 0, 0);
    const state = info();
    canvas.style.transform = state.flipped ? 'rotate(180deg)' : '';
    canvas.style.filter = `brightness(${state.brightness / 255})`;
    const saving = journal.status();
    const storageLabel = ({'public-unsaved':'UNSAVED — storage unavailable','playing-unsaved':'UNSAVED — storage unavailable','read-unavailable':wasm.sb_store_public_known()?'UNSAVED — saved session unavailable; known totals kept':'UNSAVED — saved totals unavailable; fresh defaults only','resume-refused':'Saved session could not be restored','scores-saved-checkpoint-unavailable':'Scores saved · resume unavailable','saving-checkpoint':'Saving session…','resume-available':'Saved session available'})[saving.state] || '';
    const message = `${storageLabel ? storageLabel + ' · ' : ''}Rust ${state.version} · ${state.view}${state.menu_item ? ' / ' + state.menu_item : ''} · ${state.rounds} rounds · $${(state.chips / 100).toFixed(2)} in chips`;
    if (message !== lastStatus) { status.textContent = message; lastStatus = message; }
    if (!wasm.sb_store_output_blocked()) { lastPresented = state; pumpAudio(state.sound); }
  }
  function setSource(id, bit, pressed) {
    if (!wasm) return;
    const before = downmask();
    if (pressed) sources.set(id, bit); else sources.delete(id);
    const mask = downmask();
    if (pressed && mask !== before) {
      unlock();
      if (active && wasm.sb_audio_interruptible(activeCue)) stop();
    }
    const time = now();
    inputs.push({mask, time});
    if (inputs.length > 128) { inputs.length = 0; inputs.push({cancel:true}); }
  }
  function cancel() {
    if (!wasm) return;
    sources.clear();
    inputs.length = 0;
    inputs.push({cancel:true});
  }
  function attach(button, bit) {
    button.addEventListener('pointerdown', e => {
      e.preventDefault();
      if (e.button !== undefined && e.button !== 0) return;
      button.setPointerCapture(e.pointerId);
      setSource(`pointer:${e.pointerId}`, bit, true);
    });
    button.addEventListener('pointerup', e => { e.preventDefault(); setSource(`pointer:${e.pointerId}`, bit, false); });
    button.addEventListener('pointercancel', cancel);
    button.addEventListener('lostpointercapture', e => { if (sources.has(`pointer:${e.pointerId}`)) cancel(); });
    button.addEventListener('click', e => {
      // Browser/assistive technology activation has no preceding pointer event.
      if (e.detail === 0) { setSource('activation', bit, true); setSource('activation', bit, false); }
    });
  }
  try {
    const response = await fetch('stardust-rust.wasm');
    if (!response.ok) throw new Error(`WASM HTTP ${response.status}`);
    const result = await WebAssembly.instantiate(await response.arrayBuffer(), {});
    wasm = result.instance.exports;
    const version = text.decode(bytes().subarray(wasm.sb_version(), wasm.sb_version() + wasm.sb_version_len()));
    document.title = `Stardust Blackjack · Rust ${version}`;
    const versionLabel = document.getElementById('build-version');
    if (versionLabel) versionLabel.textContent = `BLACKJACK · RUST ${version}`;
    const seed = new Uint32Array(2);
    crypto.getRandomValues(seed);
    wasm.sb_boot(seed[0], seed[1]);
    const unavailableStorage = {getItem(){throw Error('storage-unavailable');},setItem(){throw Error('storage-unavailable');}};
    let storage;try{storage=window.localStorage;}catch{storage=unavailableStorage;}
    journal = new JournalHost({storage,codec:journalCodec(wasm,now),locks:navigator.locks,random:out=>crypto.getRandomValues(out),clock:now});
    await journal.boot();
    attach(document.getElementById('blue'), 1);
    attach(document.getElementById('grey'), 2);
    document.addEventListener('keydown', e => {
      const bit = e.key === 'ArrowLeft' ? 2 : e.key === 'ArrowRight' ? 1 : 0;
      if (bit) { e.preventDefault(); if (!e.repeat) setSource(`key:${e.key}`, bit, true); }
    });
    document.addEventListener('keyup', e => {
      if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') { e.preventDefault(); setSource(`key:${e.key}`, 0, false); }
    });
    window.addEventListener('blur', cancel);
    document.addEventListener('visibilitychange', () => { if (document.hidden) cancel(); });
    // Every visible transition is rendered in Rust, publicly committed, then presented.
    // Input timestamps are queued while origin locking/storage is pending.
    async function advance(input) {
      if (input?.cancel) wasm.sb_cancel_buttons();
      else wasm.sb_buttons(input?.mask ?? downmask(), input?.time ?? now());
      if (!wasm.sb_sound_enabled()) stop();
      wasm.sb_render();
      const command = wasm.sb_store_command();
      if (command === 3 || command === 5) stop();
      if (command) await journal.command(['','retry-read','resume','start-new','retry-checkpoint','start-unsaved'][command]);
      await journal.checkpoint();
      wasm.sb_render();
      present();
    }
    wasm.sb_render();
    await journal.checkpoint();
    present();
    let last = 0;
    async function tick() {
      if (!busy) {
        busy = true;
        try {
          const time = now();
          if (wasm.sb_store_output_blocked() && ((time-lastRetry)>>>0)>=1000) { await journal.retryCheckpoint(); lastRetry=time; wasm.sb_render(); present(); }
          while (inputs.length) await advance(inputs.shift());
          if (((time - last) >>> 0) >= 30) { await advance(); last = time; }
          await journal.pump({budgetMs:3,maxSteps:64});
        } catch { status.textContent = 'UNSAVED — storage unavailable. Your current session can continue.'; }
        finally { busy = false; }
      }
      requestAnimationFrame(tick);
    }
    requestAnimationFrame(tick);
    window.stardust = Object.freeze({ info, storageStatus:()=>journal.status() });
  } catch (error) {
    status.textContent = 'The Rust game could not load. Serve this folder over HTTP; see README.';
    console.error(error);
  }
})();
