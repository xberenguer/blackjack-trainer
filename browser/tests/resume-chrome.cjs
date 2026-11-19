const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const version=fs.readFileSync(path.resolve(__dirname,'../../Cargo.toml'),'utf8').match(/^version\s*=\s*"([^"]+)"/m)[1];
const {chromium,launchOptions}=require('./chrome-config.cjs');
const baseURL=process.env.STARDUST_BROWSER_URL || 'http://127.0.0.1:8784';
let browser, page;
(async()=>{
 const out=path.resolve(__dirname,'../validation/resume');fs.mkdirSync(out,{recursive:true});
 browser=await chromium.launch(launchOptions({args:['--no-sandbox']}));
 const context=await browser.newContext({viewport:{width:1100,height:850}});
 await context.addInitScript(()=>{
  window.ioTrace=[];window.failPublic=false;window.blockJournal=false;
  const originRequest=navigator.locks.request.bind(navigator.locks);
  navigator.locks.request=(...args)=>window.blockJournal?new Promise((resolve,reject)=>{window.releaseJournal=()=>originRequest(...args).then(resolve,reject);}):originRequest(...args);
  const get=Storage.prototype.getItem,set=Storage.prototype.setItem;
  Storage.prototype.getItem=function(k){if(location.search.includes('public-read-failure=1')&&k.includes('.public.'))throw new DOMException('read unavailable');const value=get.call(this,k);if(k.includes('.public.'))window.ioTrace.push('public-read');return value;};
  Storage.prototype.setItem=function(k,v){if(k.includes('.public.')){window.ioTrace.push('public-write');if(window.failPublic)throw new DOMException('quota','QuotaExceededError');}return set.call(this,k,v);};
  class Audio{
   constructor(){this.state='running';this.destination={};}resume(){return Promise.resolve();}
   createBuffer(_c,n){const channel=new Float32Array(n);return{getChannelData:()=>channel};}
   createGain(){return{gain:{value:1},connect(){return this;}};}
   createBufferSource(){return{buffer:null,connect(){return this;},stop(){this.onended?.();},start(){window.ioTrace.push('audio');setTimeout(()=>this.onended?.(),5);}};}
  }
  window.AudioContext=Audio;
 });
 page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(baseURL);await page.waitForFunction(()=>window.stardust?.info().view==='Cut');
 const state=()=>page.evaluate(()=>window.stardust.info());
 const wait=async view=>{await page.waitForFunction(v=>window.stardust?.info().view===v,view);if(view==='ResumeChooser'||view==='ResumeReadFailure')await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));};
 const tap=async(key='ArrowRight')=>{await page.keyboard.down(key);await page.waitForTimeout(20);await page.keyboard.up(key);await page.waitForTimeout(90);};
 const hold=async key=>{await page.keyboard.down(key);await page.waitForTimeout(900);await page.keyboard.up(key);await page.waitForTimeout(90);};
 const focus=async n=>{for(let i=0;(await state()).focus!==n;i++){assert(i<40);await tap('ArrowLeft');}};
 const select=async label=>{for(let i=0;!(await state()).menu_item.includes(label);i++){assert(i<40,`missing ${label}`);await tap('ArrowLeft');}await tap();};
 assert.equal((await state()).training_mode,0);assert.equal(await page.title(),`Stardust Blackjack · v${version}`);
 await hold('ArrowLeft');await select('Settings');await wait('Settings');await focus(5);await tap();
 await page.screenshot({path:out+'/01-opt-in.png'});
 await focus(6);await tap();await focus(0);await tap();await hold('ArrowRight');await hold('ArrowRight');await hold('ArrowRight');assert.equal((await state()).training_mode,3);
 await hold('ArrowLeft');await select('Training');await focus(2);await tap();await wait('CountPractice');await tap();await wait('CountAnswer');
 await page.waitForFunction(()=>window.stardust.storageStatus().state==='saved');
 await page.screenshot({path:out+'/02-pending.png'});
 await page.reload();await wait('ResumeChooser');assert.equal((await state()).training_mode,0);
 await page.screenshot({path:out+'/03-chooser.png'});
 const publicKeys=await page.evaluate(()=>Object.keys(window.stardust));assert.deepEqual(publicKeys,['info','storageStatus']);
 await tap();await wait('CountAnswer');assert.equal((await state()).training_mode,3);
 await page.waitForTimeout(100);
 // Hold the origin lock Promise before acknowledgment: every normal query and
 // cue remains at the last presented question until the verified commit.
 await page.evaluate(()=>{window.ioTrace=[];window.blockJournal=true;});
 await page.keyboard.down('ArrowRight');await page.keyboard.down('ArrowLeft');await page.waitForTimeout(40);await page.keyboard.up('ArrowRight');await page.keyboard.up('ArrowLeft');
 await page.waitForFunction(()=>typeof window.releaseJournal==='function');
 const pending=await page.evaluate(()=>({info:window.stardust.info(),trace:window.ioTrace}));assert.equal(pending.info.view,'CountAnswer');assert(!pending.trace.includes('audio'));assert(!pending.trace.includes('public-write'));
 await page.evaluate(()=>{window.blockJournal=false;window.releaseJournal();delete window.releaseJournal;});await wait('CountResult');
 await tap('ArrowLeft');await wait('CountAnswer');await page.waitForFunction(()=>window.stardust.storageStatus().state==='saved');await page.waitForTimeout(80);
 await page.evaluate(()=>{window.ioTrace=[];window.failPublic=true;});
 await page.keyboard.down('ArrowRight');await page.keyboard.down('ArrowLeft');await page.waitForTimeout(40);await page.keyboard.up('ArrowRight');await page.keyboard.up('ArrowLeft');await wait('StorageRetry');
 const blocked=await page.evaluate(()=>({trace:window.ioTrace,status:window.stardust.storageStatus(),info:window.stardust.info()}));
 assert.equal(blocked.status.state,'public-unsaved');assert(!blocked.trace.includes('audio'),'no new feedback cue before ACK');assert.equal(blocked.info.view,'StorageRetry');
 await page.screenshot({path:out+'/04-save-retry.png'});
 await page.evaluate(()=>{window.ioTrace=[];window.failPublic=false;});await wait('CountResult');
 await page.waitForFunction(()=>window.ioTrace.includes('audio'));
 const trace=await page.evaluate(()=>window.ioTrace);const audio=trace.indexOf('audio');assert(audio>=0);assert(trace.slice(0,audio).includes('public-write'));assert(trace.slice(trace.indexOf('public-write')+1,audio).includes('public-read'),'feedback begins only after write readback');
 await page.screenshot({path:out+'/05-acknowledged-result.png'});
 await page.waitForFunction(()=>window.stardust.storageStatus().state==='saved');
 await page.reload();await wait('ResumeChooser');await tap();await wait('CountResult');
 await page.screenshot({path:out+'/06-restored-result.png'});
 // Permanent write failure has a reachable, explicitly unsaved fresh RAM exit.
 await tap('ArrowLeft');await wait('CountAnswer');await page.waitForFunction(()=>window.stardust.storageStatus().state==='saved');
 await page.evaluate(()=>{window.failPublic=true;window.ioTrace=[];});
 await page.keyboard.down('ArrowRight');await page.keyboard.down('ArrowLeft');await page.waitForTimeout(40);await page.keyboard.up('ArrowRight');await page.keyboard.up('ArrowLeft');await wait('StorageRetry');
 const durable=await page.evaluate(()=>Object.entries(localStorage).sort());await tap('ArrowLeft');assert.equal((await state()).menu_item,'Start fresh without saving');await page.screenshot({path:out+'/07-fresh-ram-choice.png'});await tap();await wait('Cut');assert.equal((await state()).training_mode,0);assert.equal((await state()).chips,1000_00);assert.equal((await state()).wager,5_00);
 assert.equal((await page.evaluate(()=>window.stardust.storageStatus())).state,'playing-unsaved');assert.deepEqual(await page.evaluate(()=>Object.entries(localStorage).sort()),durable);
 await page.screenshot({path:out+'/08-fresh-ram-unsaved.png'});await page.evaluate(()=>window.ioTrace=[]);await tap();await page.waitForTimeout(150);assert(!(await page.evaluate(()=>window.ioTrace)).includes('public-write'));assert.deepEqual(await page.evaluate(()=>Object.entries(localStorage).sort()),durable);
 // Reboot can still offer the unchanged last valid durable checkpoint.
 await page.reload();await wait('ResumeChooser');assert.equal((await state()).training_mode,0);
 // A corrupted opaque private slot never replaces the latest public progress.
 await page.evaluate(()=>{for(const key of Object.keys(localStorage))if(key.includes('.private.'))localStorage.setItem(key,'sbj1:AAAA');});
 await page.reload();await wait('Cut');assert.equal((await state()).training_mode,0);
 // Unknown public preferences/scores on a boot read-I/O failure require an
 // explicit Retry/Fresh decision, with no fallback restore or storage mutation.
 const beforeReadFailure=await page.evaluate(()=>Object.entries(localStorage).sort());
 await page.goto(baseURL+'/?public-read-failure=1');await wait('ResumeReadFailure');assert((await page.textContent('#status')).includes('saved totals unavailable; fresh defaults only'));
 await page.screenshot({path:out+'/09-boot-read-failure.png'});await tap('ArrowLeft');await tap();await wait('Cut');assert.equal((await state()).training_mode,0);assert.equal((await page.evaluate(()=>window.stardust.storageStatus())).state,'playing-unsaved');assert.deepEqual(await page.evaluate(()=>Object.entries(localStorage).sort()),beforeReadFailure);assert(!(await page.evaluate(()=>window.ioTrace)).includes('public-write'));
 await page.screenshot({path:out+'/10-boot-fresh-unsaved.png'});
 assert.deepEqual(errors,[]);
 console.log('PASS actual Chrome: opt-in defaultOff, explicit Guide resume, pending count and immutable result, pending and failed public ACK gate grade frame/status/audio, automatic retry, corrupted private fallback, strict no-write fresh RAM fallback, boot read-I/O Retry/Fresh, dynamic version and ten screenshots.');
 await browser.close();
})().catch(async e=>{console.error(e);if(page){console.error(await page.evaluate(()=>({info:window.stardust?.info(),storage:window.stardust?.storageStatus(),status:document.getElementById('status')?.textContent})).catch(()=>null));await page.screenshot({path:path.resolve(__dirname,'../validation/resume/failure.png')}).catch(()=>{});}await browser?.close();process.exitCode=1;});
