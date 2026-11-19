const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
(async()=>{const bytes=fs.readFileSync(path.join(__dirname,'../stardust-rust.wasm'));const {instance:{exports:w}}=await WebAssembly.instantiate(bytes,{});let clock=0;
const text=(p,n)=>new TextDecoder().decode(new Uint8Array(w.memory.buffer,p,n));const view=()=>text(w.sb_view(),w.sb_view_len());const item=()=>text(w.sb_menu_item(),w.sb_menu_item_len());
const key=(mask,dt=1)=>{clock+=dt;w.sb_buttons(mask,clock);w.sb_render();};const tap=(mask=1)=>{key(mask);key(0);};const hold=mask=>{key(mask);key(0,850);};const focus=n=>{for(let i=0;w.sb_focus()!==n;i++){assert(i<40,view());tap(2);}};const save=()=>{const n=w.sb_save_export();return Buffer.from(new Uint8Array(w.memory.buffer,w.sb_save_buffer(),n));};
const boot=()=>{clock=0;w.sb_boot(2,0);w.sb_render();};const load=b=>{new Uint8Array(w.memory.buffer,w.sb_save_buffer(),b.length).set(b);assert.equal(w.sb_save_import(b.length),1);w.sb_render();};const drain=()=>{const result=[];for(let n=0;n<30;n++){const c=w.sb_sound();if(!c)break;result.push(c);}return result;};
boot();assert.equal(save()[15],1,'fresh defaults select Hi-Lo');
hold(2);focus(2);tap();assert.equal(view(),'Training');focus(1);tap();assert.equal(save()[15],0,'Basic remains selectable');const basic=save();boot();load(basic);assert.equal(save()[15],0,'explicit saved Basic is preserved');boot();assert.equal(save()[15],1);
assert.equal(w.sb_audio_volume(),20);assert.equal(w.sb_audio_mask(),7);assert.equal(w.sb_fast_mode(),1);
hold(2);focus(3);tap();assert.equal(view(),'Settings');focus(1);tap();assert.equal(view(),'Sound');focus(1);tap();assert.equal(view(),'AudioVolume');
for(let i=0;i<2;i++)tap(2);assert.equal(w.sb_audio_volume(),10);tap(2);assert.equal(w.sb_audio_volume(),9);tap();assert.equal(w.sb_audio_volume(),10);tap();assert.equal(w.sb_audio_volume(),15);
for(let i=0;i<25;i++)tap(2);assert.equal(w.sb_audio_volume(),0);drain();hold(1);assert.deepEqual(drain(),[],'zero-volume test remains silent');
for(let i=0;i<30;i++)tap();assert.equal(w.sb_audio_volume(),100);tap();assert.equal(w.sb_audio_volume(),100,'no loud wrap');
drain();hold(1);assert(drain().includes(3),'holdBlue auditions card deal');assert.equal(w.sb_mode(),0,'editor test does not switch training mode');
hold(2);assert.equal(view(),'Sound');focus(2);tap();focus(3);tap();focus(4);tap();assert.equal(w.sb_audio_mask(),0);
focus(5);drain();tap();assert(drain().includes(3),'explicit audition works with category off');focus(0);tap();assert.equal(w.sb_sound_enabled(),0);focus(5);drain();tap();assert.deepEqual(drain(),[],'preview cannot unmute');focus(0);tap();
hold(2);assert.equal(view(),'Settings');focus(2);tap();assert.equal(w.sb_fast_mode(),0);tap();assert.equal(w.sb_fast_mode(),1);focus(0);tap();assert.equal(view(),'Display');tap();assert.equal(view(),'Brightness');const old=w.sb_brightness();tap(2);assert(w.sb_brightness()<old);hold(2);assert.equal(view(),'Display');hold(2);assert.equal(view(),'Settings');
const saved=save();assert.equal(saved.readUInt16LE(4),12);boot();load(saved);assert.equal(w.sb_audio_volume(),100);assert.equal(w.sb_audio_mask(),0);assert.equal(w.sb_fast_mode(),1);assert.equal(w.sb_sound_enabled(),1);assert.equal(w.sb_brightness(),saved[9]);
// A V9 packet has zero reserved bytes202/203; migration must initialize new prefs.
const legacy=Buffer.from(saved);legacy.writeUInt16LE(9,4);legacy[202]=legacy[203]=0;let crc=0xffffffff;for(const b of legacy.subarray(0,-4)){crc^=b;for(let i=0;i<8;i++)crc=(crc>>>1)^((crc&1)?0xedb88320:0);}legacy.writeUInt32LE((crc^0xffffffff)>>>0,legacy.length-4);
boot();load(legacy);assert.equal(w.sb_audio_volume(),20);assert.equal(w.sb_audio_mask(),7);assert.equal(w.sb_fast_mode(),1);assert.equal(save().readBigUInt64LE(24),legacy.readBigUInt64LE(24));
// Round-only preferences survive the shared packet without changing web UI.
const seal=bytes=>{let c=0xffffffff;for(const b of bytes.subarray(0,-4)){c^=b;for(let i=0;i<8;i++)c=(c>>>1)^((c&1)?0xedb88320:0);}bytes.writeUInt32LE((c^0xffffffff)>>>0,bytes.length-4);return bytes;};
const packet=(version,bg=0,back=0)=>{const b=Buffer.from(saved);b.writeUInt16LE(version,4);b[10]=(b[10]&7)|(bg<<3)|(back<<5);return seal(b);};
const frame=()=>Buffer.from(new Uint8Array(w.memory.buffer,w.sb_frame(),w.sb_frame_len()*2));
boot();load(packet(12));const unchangedWebFrame=frame();
for(let bg=0;bg<4;bg++)for(let back=0;back<3;back++){
 boot();load(packet(12,bg,back));const current=save();assert.equal(current.readUInt16LE(4),12);assert.equal(current[10]&0x78,(bg<<3)|(back<<5));assert.deepEqual(frame(),unchangedWebFrame,'round theme/back settings must not change fixed blue/gold web pixels');
}
for(const version of [10,11]){
 boot();load(packet(version,version===11?3:0));const current=save();assert.equal(current.readUInt16LE(4),12);assert.equal(current[10]&0x78,version===11?24:0,'legacy packets migrate without inventing a card-back selection');
}
for(const invalid of [packet(10,1),packet(11,0,1),packet(12,0,3),seal(Buffer.from(saved).map((b,i)=>i===10?b|0x80:b))]){
 boot();const before=save();new Uint8Array(w.memory.buffer,w.sb_save_buffer(),invalid.length).set(invalid);assert.equal(w.sb_save_import(invalid.length),0);assert.deepEqual(save(),before,'invalid theme/back packets cannot alter a live session');
}
boot();hold(2);focus(3);tap();focus(0);tap();assert.equal(view(),'Display');const displayRows=[];
for(let i=0;i<4;i++){displayRows.push(item());tap(2);}assert.deepEqual(displayRows,['Brightness','Flip screen','Return','Brightness'],'round-only choices stay hidden in the web display menu');
for(let cue=1;cue<=14;cue++){let peak=0;const n=w.sb_audio_samples(cue);assert(n>0);for(let i=0;i<n;i+=97)peak=Math.max(peak,Math.abs(w.sb_audio_playback_sample(cue,i)));assert(peak>0,`cue${cue} has a nonzero playback signal`);}
console.log('PASS actualWASM granular sound/mute/categories/audition/fine volume/display/Normal-Fast controls; V12 theme/back roundtrip, V10/V11 migration and strict reserved bits, fixed blue/gold web rendering, three Display rows and V9 defaults with exact counters.');
})().catch(e=>{console.error(e);process.exitCode=1});
