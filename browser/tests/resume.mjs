import assert from 'node:assert/strict';
import fs from 'node:fs';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {JournalHost} from '../journal-host.js';
import {journalCodec} from '../journal-codec.js';
const root=fileURLToPath(new URL('../',import.meta.url));
const core=fs.existsSync(root+'../src/lib.rs')?root+'../':root+'../../rust-v010-proto/';
const module=await WebAssembly.compile(fs.readFileSync(root+'stardust-rust.wasm'));
assert.deepEqual(WebAssembly.Module.imports(module),[]);
const locks={request:async(_name,_opts,callback)=>callback({name:'test'})};
class Storage{
 constructor(entries=[]){this.data=new Map(entries);this.reads=0;this.writes=0;this.trace=[];this.failRead=0;this.failWrite=0;this.failPrivate=false;this.failReadAfterWrite=false;}
 getItem(k){this.reads++;if(this.failRead===this.reads||this.failReadAfterWrite&&this.writes)throw Error('read-I/O');this.trace.push(['read',k]);return this.data.get(k)??null;}
 setItem(k,v){this.writes++;this.trace.push(['write',k]);if(this.failWrite===this.writes||this.failPrivate&&k.includes('.private.'))throw Error('quota');this.data.set(k,v);}
 removeItem(k){this.trace.push(['remove',k]);this.data.delete(k);}
 copy(){return new Storage(this.data);}
}
let nonce=0;
async function machine(storage=new Storage(),seed=2){
 const {exports:w}=await WebAssembly.instantiate(module,{});let clock=0;
 w.sb_boot(seed,0);
 const host=new JournalHost({storage,codec:journalCodec(w,()=>clock),locks,random:b=>{b.fill(0);new DataView(b.buffer,b.byteOffset,b.byteLength).setUint32(0,++nonce,true);},clock:()=>clock});
 const str=(p,n)=>new TextDecoder().decode(new Uint8Array(w.memory.buffer,p,n));
 const view=()=>str(w.sb_view(),w.sb_view_len());const item=()=>str(w.sb_menu_item(),w.sb_menu_item_len());
 const save=()=>{const n=w.sb_save_export();return Buffer.from(new Uint8Array(w.memory.buffer,w.sb_save_buffer(),n));};
 const info=()=>({view:view(),item:item(),focus:w.sb_focus(),mode:w.sb_mode(),phase:w.sb_phase(),rounds:w.sb_rounds(),chips:w.sb_bankroll(),wager:w.sb_wager()});
 const hash=()=>createHash('sha256').update(new Uint8Array(w.memory.buffer,w.sb_frame(),64800)).digest('hex');
 async function pump(){for(let i=0;i<5000;i++){await host.pump({budgetMs:100,maxSteps:256});if(!w.sb_store_capture_pending())return;}throw Error('capture did not complete');}
 async function flush(){w.sb_render();const command=w.sb_store_command();if(command)await host.command(['','retry-read','resume','start-new','retry-checkpoint','start-unsaved'][command]);await host.checkpoint();w.sb_render();}
 async function buttons(mask,ms=1){clock=(clock+ms)>>>0;w.sb_buttons(mask,clock);await flush();}
 async function tap(mask=1){await buttons(mask);await buttons(0);}
 async function hold(mask){await buttons(mask);await buttons(0,850);}
 async function tick(ms=50){clock=(clock+ms)>>>0;w.sb_tick(clock);await flush();}
 async function focus(n){for(let i=0;w.sb_focus()!==n;i++){assert(i<70,`focus ${n} in ${view()} / ${item()}`);await tap(2);}}
 async function select(label){for(let i=0;!item().toLowerCase().includes(label.toLowerCase());i++){assert(i<50,`missing ${label}: ${view()}/${item()}`);await tap(2);}await tap();}
 async function menu(){for(let i=0;view()!=='Menu';i++){assert(i<12,`cannot return to main menu from ${view()}`);await hold(2);}assert.equal(view(),'Menu');}
 async function enable(){await menu();await select('Settings');assert.equal(view(),'Settings');await select('Save/resume');assert.equal(save()[583],1);await menu();await select('Back to game');}
 async function neutral(){await buttons(0);await buttons(0);}
 await host.boot();w.sb_render();
 return {w,host,storage,view,item,save,info,hash,flush,pump,buttons,tap,hold,tick,focus,select,menu,enable,neutral,setClock:n=>{clock=n>>>0;},getClock:()=>clock};
}
let cases=0;
{
 const a=await machine();assert.equal(a.w.memory.buffer.byteLength,16777216);assert.equal(a.save()[583],0);assert.equal(a.w.sb_mode(),0);await a.flush();await a.pump();assert(![...a.storage.data.keys()].some(k=>k.includes('.private.')));
 const b=await machine(a.storage,999);assert.equal(b.view(),'Cut');assert.equal(b.w.sb_mode(),0);assert.equal(b.host.status().checkpointAvailable,false);cases++;
}
async function roundTrip(a,label){
 await a.flush();await a.pump();assert.equal(a.host.status().state,'saved',label);const before=a.info(),raw=a.save(),hash=a.hash();
 const b=await machine(a.storage.copy(),999);assert.equal(b.view(),'ResumeChooser',label);assert.equal(b.w.sb_mode(),0,'normal boot Off before explicit resume');
 // A key held across boot and its release cannot choose or act.
 await b.buttons(1);await b.buttons(0,1000);assert.equal(b.view(),'ResumeChooser');
 await b.tap();assert.notEqual(b.view(),'ResumeChooser',`${label} explicit restore`);assert.deepEqual(b.info(),before,label);assert.deepEqual(b.save(),raw,label);assert.equal(b.hash(),hash,label);
 // One neutral sample is consumed before allowing saved clock to run.
 await b.neutral();cases++;return b;
}
{
 const a=await machine();await a.enable();await a.hold(1);await a.hold(1);await a.hold(1);assert.equal(a.w.sb_mode(),3);const b=await roundTrip(a,'Guide Cut exact state');assert.equal(b.w.sb_mode(),3);
 // Deliberate fresh session preserves public progress but always boots Off.
 await b.pump();const c=await machine(b.storage.copy());await c.neutral();await c.tap(2);await c.tap();assert.equal(c.w.sb_mode(),0);assert.equal(c.view(),'Cut');await c.pump();cases++;
}
{
 const a=await machine();await a.enable();await a.tap();assert.equal(a.view(),'Burn');await a.tick(1200);assert.equal(a.view(),'Table');await a.tap();await a.tick(200);const b=await roundTrip(a,'paused initial reveal');const before=b.hash();await b.tick(100);assert.equal(b.hash(),before,'restored reveal gets full dwell, no flash');await b.tick(900);assert.notEqual(b.hash(),before);cases++;
}
{
 const a=await machine();await a.enable();await a.menu();await a.select('Training');await a.select('Practice');await a.select('Count');assert.equal(a.view(),'CountPractice');await a.tap();assert.equal(a.view(),'CountAnswer');await a.tap(2);
 const b=await roundTrip(a,'pending count answer');await b.buttons(3);await b.buttons(0);assert.equal(b.view(),'CountResult');const raw=b.save();assert.equal(raw.readBigUInt64LE(204),1n);const c=await roundTrip(b,'immutable count result');await c.buttons(3);await c.buttons(0);assert.equal(c.save().readBigUInt64LE(204),1n);cases++;
}
{
 const a=await machine();await a.enable();await a.tap();await a.menu();await a.select('Training');await a.select('Practice');await a.select('Bet');assert.equal(a.view(),'WagerPractice');await a.focus(1);await a.tap();assert.equal(a.view(),'WagerAnswer');
 const b=await roundTrip(a,'pending wager answer');await b.tap();await b.focus(4);await b.tap();assert.equal(b.view(),'WagerResult');assert.equal(b.save().readBigUInt64LE(356),1n);const c=await roundTrip(b,'wrong wager first grade');assert.equal(c.save().readBigUInt64LE(356),1n);cases++;
}

{
 const a=await machine();await a.enable();await a.menu();await a.select('Training');await a.select('Practice');await a.select('Deviation');assert.equal(a.view(),'DeviationPractice');await a.tap();assert.equal(a.view(),'DeviationQuestion');
 const b=await roundTrip(a,'pending deviation question');await b.focus(5);await b.tap();assert.equal(b.view(),'DeviationReference');const c=await roundTrip(b,'sticky helped deviation reference');await c.tap();assert.equal(c.view(),'DeviationQuestion');await c.focus(0);await c.tap();assert.equal(c.view(),'DeviationResult');assert.equal(c.save().readBigUInt64LE(584),1n);assert.equal(c.save().readBigUInt64LE(608),1n);const d=await roundTrip(c,'immutable helped deviation grade');assert.equal(d.save().readBigUInt64LE(584),1n);cases++;
}
// Public-first interruption: newest scores survive while an older private checkpoint is refused.
{
 const a=await machine();await a.enable();await a.menu();await a.select('Training');await a.select('Practice');await a.select('Count');await a.tap();await a.pump();
 const before=a.storage.copy();assert.equal(a.view(),'CountAnswer');
 await a.buttons(3);await a.buttons(0);assert.equal(a.view(),'CountResult');assert.equal(a.save().readBigUInt64LE(204),1n);
 // Cut power after public commit, before any private write. Old unanswered checkpoint cannot return.
 const afterPublic=a.storage.copy();const b=await machine(afterPublic,700);assert.equal(b.view(),'Cut');assert.equal(b.w.sb_mode(),0);assert.equal(b.save().readBigUInt64LE(204),1n);assert.equal(b.host.status().checkpointAvailable,false);cases++;
 // Failure before public setItem leaves old durable state intact and visibly unsaved.
 const c=await machine(before.copy());await c.neutral();await c.tap();await c.neutral();c.storage.failWrite=c.storage.writes+1;await c.buttons(3);await c.buttons(0);assert.equal(c.host.status().state,'public-unsaved');assert.equal(c.save().readBigUInt64LE(204),0n,'public export stays acknowledged behind the gate');
 const d=await machine(c.storage.copy());assert.equal(d.view(),'ResumeChooser');await d.neutral();await d.tap();assert.equal(d.view(),'CountAnswer');assert.equal(d.save().readBigUInt64LE(204),0n);cases++;
 // Public atomic write succeeds but verification read fails: do not authorize private write or acknowledge saved progress.
 const e=await machine(before.copy());await e.neutral();await e.tap();await e.neutral();e.storage.failReadAfterWrite=true;await e.buttons(3);await e.buttons(0);assert.equal(e.host.status().state,'public-unsaved');assert.equal(e.w.sb_store_capture_pending(),0);
 const f=await machine(e.storage.copy());assert.equal(f.view(),'Cut');assert.equal(f.save().readBigUInt64LE(204),1n);assert.equal(f.host.status().checkpointAvailable,false);cases++;
 // Private quota after public success retains newer public grades, refusing stale private state.
 const g=await machine(before.copy());await g.neutral();await g.tap();await g.neutral();g.storage.failPrivate=true;await g.buttons(3);await g.buttons(0);await g.pump();assert.equal(g.host.status().state,'scores-saved-checkpoint-unavailable');assert.equal(g.host.status().publicDurable,true);
 const h=await machine(g.storage.copy());assert.equal(h.view(),'Cut');assert.equal(h.save().readBigUInt64LE(204),1n);cases++;
 // Read I/O is never treated as blank storage and causes no writes. User may choose RAM-only play.
 const io=before.copy();io.failRead=1;const i=await machine(io);assert.equal(i.view(),'ResumeReadFailure');assert.equal(i.host.status().state,'read-unavailable');assert.equal(io.writes,0);const previous=new Map(io.data);await i.neutral();await i.tap(2);await i.tap();assert.equal(i.view(),'Cut');assert.equal(i.host.status().state,'playing-unsaved');assert.deepEqual(io.data,previous);await i.neutral();await i.tap();assert.equal(i.view(),'Burn');assert.equal(io.writes,0);cases++;
 // A corrupted public record blocks rollback to the older valid slot.
 const corrupt=before.copy();const key=[...corrupt.data.keys()].find(k=>k.includes('.public.'));corrupt.data.set(key,'sbj1:AAAA');const j=await machine(corrupt);assert.equal(j.view(),'ResumeReadFailure');assert.equal(j.host.status().state,'read-unavailable');assert.equal(corrupt.writes,0);cases++;
 // Fresh-reader conflict: another tab has advanced public epoch, so this writer must stop.
 const shared=before.copy();const k=await machine(shared),l=await machine(shared);await k.neutral();await k.tap();await k.neutral();await l.neutral();await l.tap();await l.neutral();await k.buttons(3);await k.buttons(0);const durable=new Map(shared.data);await l.tap(2);assert.equal(l.host.status().state,'public-unsaved');assert.deepEqual(shared.data,durable);cases++;
}

// Recovery verifies uncertain writes without rolling back the RAM answer or revealing it early.
for(const uncertain of [false,true]){
 const a=await machine();await a.enable();await a.menu();await a.select('Training');await a.select('Practice');await a.select('Count');await a.tap();await a.pump();
 const acknowledged=a.save();const facts=a.info();if(uncertain)a.storage.failReadAfterWrite=true;else a.storage.failWrite=a.storage.writes+1;
 await a.buttons(3);await a.buttons(0);assert.equal(a.view(),'StorageRetry');assert.equal(a.w.sb_sound(),0);assert.deepEqual(a.save(),acknowledged);assert.equal(a.w.sb_mode(),facts.mode);assert.equal(a.w.sb_bankroll(),facts.chips);assert.equal(a.w.sb_rounds(),facts.rounds);assert.equal(a.item(),'Retry saving session');assert.equal(a.w.sb_focus(),0);
 a.storage.failWrite=0;a.storage.failReadAfterWrite=false;await a.host.retryCheckpoint();a.w.sb_render();assert.equal(a.view(),'CountResult');assert.equal(a.save().readBigUInt64LE(204),1n);await a.pump();assert.equal(a.host.status().state,'saved');const b=await machine(a.storage.copy());assert.equal(b.view(),'ResumeChooser');await b.neutral();await b.tap();assert.equal(b.view(),'CountResult');assert.equal(b.save().readBigUInt64LE(204),1n);cases++;
}
// Default-off remains playable when storage fills: no private question can roll back.
{
 const a=await machine();await a.flush();a.storage.failWrite=a.storage.writes+1;await a.hold(1);assert.equal(a.host.status().state,'public-unsaved');assert.notEqual(a.view(),'StorageRetry');await a.tap();assert.equal(a.view(),'Burn');cases++;
}
// Same public bytes cannot authorize a checkpoint from a distinct nonce/session.
{
 const a=await machine(),b=await machine();await a.enable();await b.enable();await a.pump();await b.pump();
 const swapped=a.storage.copy();for(const [key,value] of b.storage.data)if(key.includes('.private.'))swapped.data.set(key,value);
 const c=await machine(swapped);assert.equal(c.view(),'Cut');assert.equal(c.host.status().checkpointAvailable,false);cases++;
}

// Boot taint: surviving marker/header cannot authorize an old unanswered question
// when the newer graded public payload/footer is damaged.
for(const damage of ['payload','unfinished']){
 const a=await machine();await a.enable();await a.menu();await a.select('Training');await a.select('Practice');await a.select('Count');await a.tap();await a.pump();
 await a.buttons(3);await a.buttons(0);assert.equal(a.save().readBigUInt64LE(204),1n);
 const corrupt=a.storage.copy();const records=[...corrupt.data].filter(([k])=>k.includes('.public.')).map(([key,value])=>({key,bytes:Buffer.from(value.slice(5),'base64')}));records.sort((a,b)=>Number(b.bytes.readBigUInt64LE(8)-a.bytes.readBigUInt64LE(8)));const latest=records[0];if(damage==='payload')latest.bytes[64+204]^=1;else latest.bytes=latest.bytes.subarray(0,latest.bytes.length-1);corrupt.data.set(latest.key,'sbj1:'+latest.bytes.toString('base64'));
 const b=await machine(corrupt);assert.equal(b.view(),'ResumeReadFailure');assert.equal(b.w.sb_mode(),0);assert.equal(b.host.status().checkpointAvailable,false);assert.equal(corrupt.writes,0);
 await b.neutral();await b.tap();assert.equal(b.view(),'ResumeReadFailure','retry cannot authorize the old matching unanswered private record');await b.neutral();await b.tap(2);await b.tap();assert.equal(b.view(),'Cut');assert.equal(b.host.status().state,'playing-unsaved');assert.equal(corrupt.writes,0);cases++;
}
// Waiting at a chooser cannot roll back public progress committed by another tab.
{
 const shared=new Storage();const a=await machine(shared);await a.enable();await a.menu();await a.select('Training');await a.select('Practice');await a.select('Count');await a.tap();await a.pump();const b=await machine(shared);assert.equal(b.view(),'ResumeChooser');await a.buttons(3);await a.buttons(0);await b.neutral();await b.tap();assert.equal(b.view(),'ResumeReadFailure');assert.equal(b.save().readBigUInt64LE(204),1n,'adopts newest verified public progress before refusing stale private');assert.equal(b.w.sb_mode(),0);cases++;
}

// Public ABI is gated even before an asynchronous storage lock is acquired.
{
 const a=await machine();await a.enable();const before=a.info(),raw=a.save(),frame=a.hash();while(a.w.sb_sound()){}
 a.w.sb_buttons(1,a.getClock()+1);a.w.sb_buttons(0,a.getClock()+2);a.w.sb_render();assert.equal(a.w.sb_store_output_pending(),1);assert.equal(a.view(),'Saving');assert.equal(a.w.sb_phase(),before.phase);assert.equal(a.w.sb_bankroll(),before.chips);assert.deepEqual(a.save(),raw);assert.equal(a.hash(),frame);assert.equal(a.w.sb_sound(),0);await a.flush();assert.equal(a.view(),'Burn');cases++;
}
// The adapter preserves ordered large-card NPC carousels, queued terminal faces,
// future actions and public grade outcomes across independent WASM instances.
const crc32=bytes=>{let c=0xffffffff;for(const b of bytes){c^=b;for(let i=0;i<8;i++)c=(c>>>1)^((c&1)?0xedb88320:0);}return(c^0xffffffff)>>>0;};
let seatedSteps=0;
for(const seat of [0,3,6]){
 const packet=Buffer.from(fs.readFileSync(core+'tests/data/save-v7.bin'));packet[332]=0;packet[12]=90;packet[200]=6;packet[201]=seat;packet.writeUInt32LE(crc32(packet.subarray(0,580)),580);
 const a=await machine(new Storage([['stardust-blackjack-userdata',JSON.stringify([...packet])]]),21+seat);await a.enable();let b=await roundTrip(a,`seated${seat} Cut`);
 for(let step=0;step<55;step++){
  if(step&&step%17===0)b=await roundTrip(a,`seated${seat} visibility${step}`);
  for(const m of [a,b]){await m.tick(750);if(['Cut','Burn','Refill','Notice','RoundReview','TrainingReview','CoachCheck'].includes(m.view())||m.view()==='Table'&&[1,2,3,5].includes(m.w.sb_phase()))await m.tap();}
  assert.deepEqual(b.info(),a.info(),`seat${seat} step${step}`);assert.deepEqual(b.save(),a.save());assert.equal(b.hash(),a.hash(),`seat${seat} frame${step}`);seatedSteps++;
 }
}

// Recorded training round review and current-session analytics survive transport.
{
 const a=await machine();await a.enable();await a.hold(1);assert.equal(a.w.sb_mode(),1);
 for(let guard=0;a.view()!=='RoundReview';guard++){assert(guard<120,'test round must reach review');await a.tick(750);if(['Cut','Burn','Refill','Notice'].includes(a.view())||a.view()==='Table'&&[1,2,3,5].includes(a.w.sb_phase()))await a.tap();}
 const b=await roundTrip(a,'Test round immutable review');assert.equal(b.w.sb_mode(),1);await b.menu();await b.select('Session');await b.select('Reports');assert.equal(b.view(),'AnalysisMenu');await b.tap();assert.equal(b.view(),'AnalysisSummary');await roundTrip(b,'current session metrics summary');cases++;
}

// The appended live-preview view and V13 display choices survive opaque
// private checkpoints without remapping any legacy view or Display row ID.
{
 let a=await machine();await a.enable();await a.menu();await a.select('Settings');await a.select('Display');await a.select('Background');await a.select('Card back');assert.equal(a.view(),'CardBackPreview');
 for(let back=0;back<3;back++){
  assert.equal(a.save().readUInt16LE(4),13);assert.equal(a.save()[10]&0xe0,0x80|(back<<5));a=await roundTrip(a,`V13 live card-back preview ${back}`);assert.equal(a.view(),'CardBackPreview');assert.equal(a.save()[10]&0xe0,0x80|(back<<5));await a.tap(2);
 }
 await a.tap();assert.equal(a.view(),'Display');assert.equal(a.w.sb_focus(),4);assert.equal(a.item(),'Card back');cases++;
}

// Permanent protected write failure has an explicit fresh RAM-only exit; it
// discards the unseen grade and never reopens the old pending question.
{
 const a=await machine();await a.enable();await a.menu();await a.select('Training');await a.select('Practice');await a.select('Count');await a.tap();await a.pump();a.storage.failWrite=a.storage.writes+1;
 await a.buttons(3);await a.buttons(0);assert.equal(a.view(),'StorageRetry');await a.neutral();await a.tap(2);assert.equal(a.item(),'Start fresh without saving');for(let retry=0;retry<3;retry++){a.storage.failWrite=a.storage.writes+1;await a.host.retryCheckpoint();a.w.sb_render();assert.equal(a.view(),'StorageRetry');assert.equal(a.item(),'Start fresh without saving','failed automatic retry preserves the choice');}const durable=new Map(a.storage.data);await a.tap();assert.equal(a.view(),'Cut');assert.equal(a.w.sb_mode(),0);assert.equal(a.w.sb_bankroll(),100000n);assert.equal(a.w.sb_wager(),500n);assert.equal(a.save()[583],0);assert.equal(a.save().readBigUInt64LE(204),0n);assert.equal(a.host.status().state,'playing-unsaved');assert.deepEqual(a.storage.data,durable,'fresh RAM fallback preserves both journals byte-for-byte');
 const writes=a.storage.writes;await a.neutral();await a.menu();await a.select('Settings');await a.select('Save/resume');assert.equal(a.save()[583],0,'RAM boot cannot reenable resume');await a.menu();await a.select('Back to game');await a.tap();assert.equal(a.view(),'Burn');await a.tick(3000);assert.equal(a.storage.writes,writes);const b=await machine(a.storage.copy());assert.equal(b.view(),'ResumeChooser','reboot may offer the last valid durable checkpoint');assert.equal(b.w.sb_mode(),0);cases++;
}

{
 const a=await machine();await a.enable();await a.hold(1);await a.hold(1);await a.hold(1);await a.menu();await a.select('Training');await a.select('Practice');await a.select('Count');await a.tap();await a.buttons(3);await a.buttons(0);assert.equal(a.view(),'CountResult');const b=await roundTrip(a,'Guide count immutable result');assert.equal(b.w.sb_mode(),3);cases++;
}
// New confirmation screens use the same acknowledged journal protocol. A
// canceled reset is inert; a confirmed reset cannot resurrect an older grade.
{
 const a=await machine();await a.enable();await a.menu();await a.select('Training');await a.select('Practice');await a.select('Count');await a.tap();await a.buttons(3);await a.buttons(0);assert.equal(a.view(),'CountResult');assert.equal(a.save().readBigUInt64LE(204),1n);
 const b=await roundTrip(a,'graded count before explicit reset');await b.menu();await b.select('Training');await b.select('Progress');await b.select('Reset training stats');assert.equal(b.view(),'ResetStatsConfirm');assert.equal(b.item(),'Cancel');const original=b.save(),facts=b.info();
 await b.tap();assert.equal(b.view(),'ProgressMenu');assert.deepEqual(b.save(),original,'default cancel preserves acknowledged training');
 await b.select('Reset training stats');await b.tap(2);assert.equal(b.item(),'Clear training stats');await b.tap();assert.equal(b.save().readBigUInt64LE(204),0n);assert.equal(b.w.sb_bankroll(),facts.chips);assert.equal(b.w.sb_phase(),facts.phase);
 const c=await roundTrip(b,'confirmed score reset with no stale-grade resurrection');assert.equal(c.save().readBigUInt64LE(204),0n);
 await c.menu();await c.select('Session');await c.select('New session');assert.equal(c.view(),'NewSessionConfirm');assert.equal(c.item(),'Cancel');const beforeNew=c.save();await c.tap();assert.equal(c.view(),'SessionMenu');assert.deepEqual(c.save(),beforeNew);
 await c.select('New session');await c.tap(2);assert.equal(c.item(),'Start new session');await c.tap();assert.equal(c.view(),'Cut');assert.equal(c.w.sb_bankroll(),100000n);assert.equal(c.w.sb_mode(),0);assert.equal(c.save()[583],1,'new session preserves the resume preference');
 const d=await roundTrip(c,'confirmed new session resumes its fresh shoe');assert.equal(d.view(),'Cut');assert.equal(d.save().readBigUInt64LE(204),0n);cases++;
}
// Exact 64-bit public totals across all legacy schemas and current V9, with default-off resume migration.
for(let v=1;v<=8;v++){
 const packet=fs.readFileSync(core+`tests/data/save-v${v}.bin`);
 const store=new Storage([['stardust-blackjack-userdata',JSON.stringify([...packet])]]);const a=await machine(store);assert.equal(a.w.sb_mode(),0);assert.equal(a.save()[583],0);assert.equal(a.save().readBigUInt64LE(24),packet.readBigUInt64LE(v<3?16:24));await a.flush();assert(store.writes>0);const b=await machine(store.copy());assert.deepEqual(b.save().subarray(24,200),a.save().subarray(24,200));cases++;
}
console.log(`PASS ${cases} actual WASM resume continuation/migration/fault cases; fixed16MiB, defaultOff/explicitGuide, neutral tails, paused reveal, count/wager/drill pending+stickyhelp+immutable grades, public-first power loss, IO/quota/corruption/conflicts, V1–8 exact u64; ${seatedSteps} independent seven-seat continuation steps.`);
