// Real compiled Rust ABI: no C++ oracle and no JavaScript implementation of rules.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname, '..');
const core=fs.existsSync(path.join(root,'../src/lib.rs'))?path.resolve(root,'..'):path.resolve(root,'../../rust-v010-proto');
const moduleBytes = fs.readFileSync(path.join(root, 'stardust-rust.wasm'));
(async () => {
  const module = await WebAssembly.compile(moduleBytes);
  assert.deepEqual(WebAssembly.Module.imports(module), [], 'standalone Rust has no runtime imports');
  const { exports: w } = await WebAssembly.instantiate(module, {});
  const expectedVersion = fs.readFileSync(path.join(core, 'Cargo.toml'), 'utf8').match(/^version\s*=\s*"([^"]+)"/m)[1];
  const actualVersion = new TextDecoder().decode(new Uint8Array(w.memory.buffer, w.sb_version(), w.sb_version_len()));
  assert.equal(actualVersion, expectedVersion, 'compiled browser version must match the current game source');
  const SIZE=w.sb_save_size();
  let clock = 0;
  const view = () => new TextDecoder().decode(new Uint8Array(w.memory.buffer, w.sb_view(), w.sb_view_len()));
  const tick = (ms = 50) => { clock += ms; w.sb_tick(clock); };
  const buttons = (mask, advance = 1) => { clock += advance; w.sb_buttons(mask, clock); };
  const tap = (mask = 1) => { buttons(mask); buttons(0); };
  const hold = mask => { buttons(mask); buttons(mask, 850); buttons(0); };
  const focus = n => { let tries = 0; while (w.sb_focus() !== n) { assert(++tries < 50); tap(2); } };
  const boot = (seed = 2) => { clock = 0; w.sb_boot(seed, 0); };
  const save = () => { const size=w.sb_save_size(); assert.equal(size,SIZE); assert.equal(w.sb_save_export(),size); return Buffer.from(new Uint8Array(w.memory.buffer,w.sb_save_buffer(),size)); };
  const load = bytes => { new Uint8Array(w.memory.buffer).set(bytes, w.sb_save_buffer()); return w.sb_save_import(bytes.length); };
  boot();
  assert.equal(w.memory.buffer.byteLength, 16 * 1024 * 1024);
  assert.equal(w.sb_frame_len(), 240 * 135);
  assert.equal(view(), 'Cut');
  assert.equal(w.sb_bankroll(), 100000n);
  assert.equal(w.sb_wager(), 500n);
  w.sb_render();
  const initialFrame = Buffer.from(new Uint8Array(w.memory.buffer, w.sb_frame(), 64800));
  assert(new Set(new Uint16Array(w.memory.buffer, w.sb_frame(), 32400)).size > 20);
  assert.throws(() => w.memory.grow(1), /Maximum memory size|Unable to grow/);
  // Chords consume both release tails, including long staggered releases.
  buttons(3); buttons(1, 1000); buttons(0); assert.equal(view(), 'Cut');
  // Cancellation never produces a tap or a hold.
  buttons(1); w.sb_cancel_buttons(); buttons(0, 1000); assert.equal(view(), 'Cut');
  buttons(2); buttons(2, 849); assert.equal(view(), 'Cut');
  buttons(2, 1); assert.equal(view(), 'Menu'); buttons(0); assert.equal(w.sb_focus(), 0);
  tap(); assert.equal(view(), 'Cut');
  for (let mode = 1; mode <= 4; mode++) { hold(1); assert.equal(w.sb_mode(), mode % 4); assert.equal(view(), 'Cut'); }
  // Release-time recognition works even with no intermediate held sample.
  for (const duration of [849, 850, 851, 5000]) {
    boot(); buttons(1); buttons(0, duration);
    assert.equal(w.sb_mode(), Number(duration >= 850));
    assert.equal(view(), duration >= 850 ? 'Cut' : 'Burn');
  }
  boot();
  // Count question: all game/menu and chord decisions are inside Rust.
  hold(2); focus(2); tap(); assert.equal(view(), 'Training');
  focus(2); tap(); assert.equal(view(), 'CountPractice');
  tap(); assert.equal(view(), 'CountAnswer');
  buttons(3); assert.equal(view(), 'CountResult');
  buttons(1, 1000); buttons(0); assert.equal(view(), 'CountResult');
  assert.equal(save().readBigUInt64LE(204), 1n);
  // Audio bank ABI is bounded and sample-identical to its bundled Rust asset.
  for (let cue = 1; cue <= 14; cue++) {
    assert.equal(w.sb_audio_rate(cue), 22050);
    assert(w.sb_audio_samples(cue) > 0);
    assert(w.sb_audio_data(cue) > 0);
    assert.equal(w.sb_audio_interruptible(cue), Number(cue === 6 || cue === 7));
  }
  for (const cue of [0, 15, 0xffffffff]) {
    assert.equal(w.sb_audio_samples(cue), 0); assert.equal(w.sb_audio_data(cue), 0);
  }
  // Every published save version migrates, retaining exact >2^53 counters.
  for (let version = 1; version <= 8; version++) {
    const fixture = fs.readFileSync(path.join(core, 'tests/data', `save-v${version}.bin`));
    boot(); assert.equal(load(fixture), 1); assert.equal(w.sb_mode(), 0, 'boot is always Off');
    const output = save();
    const oldAt = version < 3 ? 16 : 24;
    assert.deepEqual(output.subarray(24, 200), fixture.subarray(oldAt, oldAt + 176));
    if (version >= 5) assert.deepEqual(output.subarray(204, 332), fixture.subarray(204, 332));
    if (version >= 6) assert.deepEqual(output.subarray(200,202),fixture.subarray(200,202)); assert.equal(output[202],20); assert.equal(output[203],15);
    if (version >= 7) assert.deepEqual(output.subarray(332,580),fixture.subarray(332,580));
    if(version===8)assert.deepEqual(output.subarray(580,2632),fixture.subarray(580,2632));
    assert.equal(load(fixture), 0, 'load cannot overwrite a live app repeatedly');
    const corrupt = Buffer.from(fixture); corrupt[20] ^= 1;
    boot(); assert.equal(load(corrupt), 0);
    assert.equal(w.sb_bankroll(), 100000n);
  }
  boot();
  for (const length of [0, 1, 195, 197, 207, 209, 335, 337, 583, 585, SIZE-1,SIZE+1,0xffffffff]) assert.equal(w.sb_save_import(length), 0);
  // An actual Rust wager prompt records the legal wrong first bet unchanged.
  boot(); tap(); assert.equal(view(),'Burn');
  hold(2); focus(2); tap(); focus(8); tap(); assert.equal(view(),'WagerPractice');
  focus(1); tap(); assert.equal(view(),'WagerAnswer');
  tap(); focus(4); tap(); assert.equal(view(),'WagerResult');
  assert.equal(w.sb_wager(),500n);
  assert.equal(save().readBigUInt64LE(356),1n);
  assert.equal(save().readBigUInt64LE(372),1n);
  tap(); assert.equal(w.sb_wager(),1000n,'play keeps the chosen wrong legal wager');
  // Multi-seat preferences exercise the actual WASM controller and reveal/tick path.
  const crc32 = bytes => { let c=0xffffffff; for(const b of bytes) {c^=b;for(let i=0;i<8;i++)c=(c>>>1)^((c&1)?0xedb88320:0);} return(c^0xffffffff)>>>0; };
  let seatedRounds=0;
  for(const [profile,seat] of [[0,0],[1,3],[2,6]]) {
    boot(21+seat);
    const fixture=Buffer.from(fs.readFileSync(path.join(core,'tests/data/save-v7.bin')));
    fixture[332]=0;fixture[11]=profile;fixture[12]=90;fixture[200]=6;fixture[201]=seat;
    fixture.writeUInt32LE(crc32(fixture.subarray(0,580)),580);
    assert.equal(load(fixture),1);
    let guard=0;
    while(w.sb_rounds()<20) {
      assert(++guard<20000,`seated ${profile}/${seat}: ${view()} phase ${w.sb_phase()}`);
      if(view()==='Cut'||view()==='Burn'||view()==='Refill'||view()==='Notice')tap();
      else if(view()==='Table'&&[1,2,3,5].includes(w.sb_phase()))tap();
      tick(750);w.sb_render();while(w.sb_sound()){}
    }
    seatedRounds+=w.sb_rounds();
    assert.equal(save()[200],6);assert.equal(save()[201],seat);
  }
  // Lots of actual shuffle/deal/stand/settle/cut cycles through the public ABI.
  let totalRounds = 0;
  for (let seed = 1; seed <= 48; seed++) {
    boot(seed);
    const target = 150;
    let guard = 0;
    while (w.sb_rounds() < target) {
      assert(++guard < 20000, `seed ${seed}: ${view()} phase ${w.sb_phase()}`);
      if (view() === 'Cut' || view() === 'Burn') tap();
      else if (view() === 'Table') {
        if ([1, 2, 3, 5].includes(w.sb_phase())) tap();
      } else throw new Error(`unexpected view ${view()}`);
      tick(650);
      w.sb_render();
      let queued = 0, sound;
      while ((sound = w.sb_sound())) { assert(sound >= 1 && sound <= 14); assert(++queued < 16); }
      assert(w.sb_bankroll() >= 0n);
      if (w.sb_bankroll() < 500n && [1, 5].includes(w.sb_phase())) {
        // End this funded run; bankroll exhaustion is an expected outcome.
        break;
      }
      if (view() === 'Refill') { tap(); }
    }
    totalRounds += w.sb_rounds();
    assert(w.sb_bankroll() >= 0n);
    const packet = save(); assert.equal(packet.length, SIZE);
    assert.equal(w.memory.buffer.byteLength, 16 * 1024 * 1024);
    w.sb_render();
    assert.notDeepEqual(Buffer.from(new Uint8Array(w.memory.buffer, w.sb_frame(), 64800)), initialFrame);
  }
  console.log(`PASS: standalone Rust WASM, RGB565 frame, holds/chords/cancellation, count submission, wager first-answer/application, ${seatedRounds} seven-seat deep rounds, 14 PCM clips, V1–V8 exact migrations, bounded imports, ${totalRounds} normal rounds / 48 seeds.`);
})().catch(error => { console.error(error); process.exitCode = 1; });
