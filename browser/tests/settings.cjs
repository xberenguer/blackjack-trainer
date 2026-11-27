const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
(async()=>{const bytes=fs.readFileSync(path.join(__dirname,'../stardust-rust.wasm'));const {instance:{exports:w}}=await WebAssembly.instantiate(bytes,{});let clock=0;
const text=(p,n)=>new TextDecoder().decode(new Uint8Array(w.memory.buffer,p,n));const view=()=>text(w.sb_view(),w.sb_view_len());const item=()=>text(w.sb_menu_item(),w.sb_menu_item_len());
const key=(mask,dt=1)=>{clock+=dt;w.sb_buttons(mask,clock);w.sb_render();};const tap=(mask=1)=>{key(mask);key(0);};const hold=mask=>{key(mask);key(0,850);};const focus=n=>{for(let i=0;w.sb_focus()!==n;i++){assert(i<40,view());tap(2);}};const save=()=>{const n=w.sb_save_export();return Buffer.from(new Uint8Array(w.memory.buffer,w.sb_save_buffer(),n));};
const select=label=>{for(let i=0;!item().toLowerCase().includes(label.toLowerCase());i++){assert(i<40,`missing ${label}: ${view()}/${item()}`);tap(2);}tap();};
const boot=()=>{clock=0;w.sb_boot(2,0);w.sb_render();};const load=b=>{new Uint8Array(w.memory.buffer,w.sb_save_buffer(),b.length).set(b);assert.equal(w.sb_save_import(b.length),1);w.sb_render();};const drain=()=>{const result=[];for(let n=0;n<30;n++){const c=w.sb_sound();if(!c)break;result.push(c);}return result;};
boot();assert.equal(save()[15],1,'fresh defaults select Hi-Lo');
hold(2);select('Training');select('Strategy');assert.equal(save()[15],0,'Basic remains selectable');const basic=save();boot();load(basic);assert.equal(save()[15],0,'explicit saved Basic is preserved');boot();assert.equal(save()[15],1);
assert.equal(w.sb_audio_volume(),20);assert.equal(w.sb_audio_mask(),7);assert.equal(w.sb_fast_mode(),1);
hold(2);select('Settings');select('Sound');select('Volume');assert.equal(view(),'AudioVolume');
for(let i=0;i<2;i++)tap(2);assert.equal(w.sb_audio_volume(),10);tap(2);assert.equal(w.sb_audio_volume(),9);tap();assert.equal(w.sb_audio_volume(),10);tap();assert.equal(w.sb_audio_volume(),15);
for(let i=0;i<25;i++)tap(2);assert.equal(w.sb_audio_volume(),0);drain();hold(1);assert.deepEqual(drain(),[],'zero-volume test remains silent');
for(let i=0;i<30;i++)tap();assert.equal(w.sb_audio_volume(),100);tap();assert.equal(w.sb_audio_volume(),100,'no loud wrap');
drain();hold(1);assert(drain().includes(3),'holdBlue auditions card deal');assert.equal(w.sb_mode(),0,'editor test does not switch training mode');
hold(2);select('Categories');select('Menu');select('Card');select('Result');assert.equal(w.sb_audio_mask(),0);select('Back');
select('Test sounds');drain();select('Deal');assert(drain().includes(3),'explicit audition works with category off');select('Back');select('Master');assert.equal(w.sb_sound_enabled(),0);select('Test sounds');drain();select('Deal');assert.deepEqual(drain(),[],'preview cannot unmute');select('Back');select('Master');
select('Back');select('Speed');assert.equal(w.sb_fast_mode(),0);select('Speed');assert.equal(w.sb_fast_mode(),1);select('Display');select('Brightness');assert.equal(view(),'Brightness');const old=w.sb_brightness();tap(2);assert(w.sb_brightness()<old);hold(2);assert.equal(view(),'Display');hold(2);assert.equal(view(),'Settings');
const saved=save();assert.equal(saved.readUInt16LE(4),14);boot();load(saved);assert.equal(w.sb_audio_volume(),100);assert.equal(w.sb_audio_mask(),0);assert.equal(w.sb_fast_mode(),1);assert.equal(w.sb_sound_enabled(),1);assert.equal(w.sb_brightness(),saved[9]);
// A V9 packet has zero reserved bytes202/203; migration must initialize new prefs.
const legacy=Buffer.from(saved);legacy.writeUInt16LE(9,4);legacy[202]=legacy[203]=0;let crc=0xffffffff;for(const b of legacy.subarray(0,-4)){crc^=b;for(let i=0;i<8;i++)crc=(crc>>>1)^((crc&1)?0xedb88320:0);}legacy.writeUInt32LE((crc^0xffffffff)>>>0,legacy.length-4);
boot();load(legacy);assert.equal(w.sb_audio_volume(),20);assert.equal(w.sb_audio_mask(),7);assert.equal(w.sb_fast_mode(),1);assert.equal(save().readBigUInt64LE(24),legacy.readBigUInt64LE(24));
// Compact and round themes remain independent. V13 used bit 7 of byte 10 for compact Black.
const seal=bytes=>{let c=0xffffffff;for(const b of bytes.subarray(0,-4)){c^=b;for(let i=0;i<8;i++)c=(c>>>1)^((c&1)?0xedb88320:0);}bytes.writeUInt32LE((c^0xffffffff)>>>0,bytes.length-4);return bytes;};
const packet=(version,bg=0,back=0,black=0)=>{const b=Buffer.from(saved);b.writeUInt16LE(version,4);b[10]=(b[10]&7)|(bg<<3)|(back<<5)|(black<<7);return seal(b);};
const frame=()=>Buffer.from(new Uint8Array(w.memory.buffer,w.sb_frame(),w.sb_frame_len()*2));
boot();assert.equal(save().readUInt16LE(4),14);assert.equal(save()[10]&0xe0,0,'fresh compact defaults remain Blue and Gold');
for(let back=0;back<3;back++)for(let black=0;black<2;black++){
 boot();load(packet(13,0,back,black));const reference=frame();
 for(let bg=0;bg<4;bg++){
  boot();load(packet(13,bg,back,black));const current=save();assert.equal(current.readUInt16LE(4),14);assert.equal(current[10]&0xf8,(bg<<3)|(back<<5));assert.equal(current[203]>>4,black);assert.deepEqual(frame(),reference,'round theme must not alter the compact display');
 }
}
for(const version of [10,11,12]){
 const bg=version>=11?3:0,back=version>=12?2:0;boot();load(packet(version,bg,back));const current=save();assert.equal(current.readUInt16LE(4),14);assert.equal(current[10]&0xf8,(bg<<3)|(back<<5),'old saves migrate to Blue while retaining available theme/back preferences');
}
for(const invalid of [packet(10,1),packet(11,0,1),packet(12,0,3),packet(12,0,0,1),packet(13,0,3),packet(14,0,0,1),packet(15)]){
 boot();const before=save();new Uint8Array(w.memory.buffer,w.sb_save_buffer(),invalid.length).set(invalid);assert.equal(w.sb_save_import(invalid.length),0);assert.deepEqual(save(),before,'invalid packets cannot alter a live session');
}
boot();hold(2);select('Settings');select('Display');assert.equal(view(),'Display');const displayRows=[];
for(let i=0;i<4;i++){displayRows.push([w.sb_focus(),item()]);tap(2);}
assert.deepEqual(displayRows,[[0,'Brightness'],[1,'Flip screen'],[4,'Colors'],[2,'Back']]);
select('Colors');assert.equal(view(),'CardBackPreview');assert.equal(item(),'Background');
const backgroundFrames=[];
const tableColors=[0x0000,0x080d,0x0341,0x6840];
for(const color of [2,3,0,1,2]){
 tap();assert.equal((save()[203]>>4)^1,color);assert.equal(item(),'Background');
 const pixels=frame();
 assert.equal(pixels.readUInt16LE(0),tableColors[color],'Colors paints the selected table color');
 backgroundFrames.push(pixels);
}
assert.equal(new Set(backgroundFrames.map(pixels=>pixels.toString('base64'))).size,4);
tap(2);assert.equal(item(),'Card back');const backFrames=[];
for(let back=0;back<4;back++){assert.equal((save()[10]>>5)&3,back);backFrames.push(frame());tap();}
assert.equal(new Set(backFrames.map(pixels=>pixels.toString('base64'))).size,4);
for(let i=0;i<3;i++)tap();
assert.equal((save()[10]>>5)&3,3);tap(2);assert.equal(item(),'Back');tap();
assert.equal(view(),'Display');assert.equal(w.sb_focus(),4);assert.equal(item(),'Colors');
const chosen=save();boot();load(chosen);assert.equal(save()[10]&0x60,0x60);assert.equal((save()[203]>>4)^1,2);
hold(2);select('Settings');select('Display');select('Colors');hold(2);assert.equal(view(),'Display');assert.equal(w.sb_focus(),4,'hold Grey returns to Colors');
for(let cue=1;cue<=14;cue++){let peak=0;const n=w.sb_audio_samples(cue);assert(n>0);for(let i=0;i<n;i+=97)peak=Math.max(peak,Math.abs(w.sb_audio_playback_sample(cue,i)));assert(peak>0,`cue${cue} has a nonzero playback signal`);}
// Grouped navigation and cancel feedback must reach PCM exactly once, while
// master mute, zero volume and disabled Menus filter ordinary navigation.
boot();const navigationBase=save();
for(const [name,enabled,volume,mask]of [['audible',1,20,7],['master mute',0,20,7],['zero volume',1,0,7],['Menus category off',1,20,6]]){
 const b=Buffer.from(navigationBase);b[10]=(b[10]&~1)|enabled;b[202]=volume;b[203]=(b[203]&8)|mask;seal(b);boot();load(b);
 const ordinary=enabled&&volume>0&&(mask&1);hold(2);select('Settings');drain();tap(2);assert.deepEqual(drain(),ordinary?[1]:[],`${name}: grouped next cue`);assert.equal(item(),'Sound');
 tap();assert.equal(view(),'Sound');assert.deepEqual(drain(),ordinary?[1]:[],`${name}: grouped selection cue`);hold(2);assert.equal(view(),'Settings');assert.deepEqual(drain(),ordinary?[2]:[],`${name}: grouped back cue`);
 hold(2);select('Session');select('New session');assert.equal(view(),'NewSessionConfirm');drain();tap();assert.equal(view(),'SessionMenu');assert.deepEqual(drain(),ordinary?[1]:[],`${name}: cancel cue`);
 hold(2);select('Settings');select('Sound');select('Test sounds');
 for(const [label,cue]of [['Deal',3],['Win',6],['Menu',2]]){
  while(item()!==label)tap(2);drain();tap();assert.deepEqual(drain(),enabled&&volume>0?[cue]:[],`${name}: ${label} audition has one cue, no navigation overlay`);
 }
}
console.log('PASS actualWASM granular sound/mute/categories/audition/fine volume/display/Normal-Fast controls and filtered grouped navigation/cancel/single-cue auditions; V14 compact four-color felt and Charcoal card-back roundtrip, V10–V13 migration and strict bits, independent round themes, live card-back previews and stable Display IDs and V9 defaults with exact counters.');
})().catch(e=>{console.error(e);process.exitCode=1});
