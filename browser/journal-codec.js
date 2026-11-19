/* Private lexical adapter. Byte transport and scheduler glue only; Rust decides validity/UI. */
export function journalCodec(wasm,clock=()=>performance.now()){
 const buffer=()=>new Uint8Array(wasm.memory.buffer,wasm.sb_store_buffer());
 const put=bytes=>{if(bytes.length>buffer().length)throw Error('bounded-transport');buffer().set(bytes);};
 const load=(kind,slot,value)=>{if(value instanceof Uint8Array){put(value);wasm.sb_store_load(kind,slot,1,value.length);}else wasm.sb_store_load(kind,slot,value===null?0:value==='corrupt'?3:2,0);};
 const verify=(pub,priv)=>{wasm.sb_store_verify_begin();pub.forEach((v,i)=>load(2,i,v));priv?.forEach((v,i)=>load(3,i,v));};
 const plan=(len,slot)=>len?{token:wasm.sb_store_token(),slot,bytes:buffer().subarray(0,len)}:null;
 return Object.freeze({
  capacity:kind=>wasm.sb_store_capacity(kind==='public'?0:1),legacyCapacity:()=>wasm.sb_save_size(),
  beginLoad:()=>wasm.sb_store_begin(),loadSlot:(kind,slot,value)=>load(kind==='public'?0:1,slot,value),
  readFailure:kind=>{load(kind==='public'?0:1,0,undefined);},
  loadLegacy:bytes=>{try{put(bytes);return !!wasm.sb_store_legacy(bytes.length);}finally{bytes.fill(0);}},
  finishPublic:()=>!!wasm.sb_store_finish_public(),wantsPrivate:()=>!!wasm.sb_store_wants_private(),
  finishBoot:now=>wasm.sb_store_finish_boot(Math.floor(now)>>>0)?'resume-available':'fresh',
  bootReadFailure:()=>wasm.sb_store_read_failure(),
  baseMatches:pub=>{verify(pub);return !!wasm.sb_store_base_matches();},
  recoverBase:pub=>{verify(pub);const result=wasm.sb_store_recover();if(result===2)wasm.sb_store_commit_clock(Math.floor(clock())>>>0);return result;},
  needsCheckpoint:()=>!!wasm.sb_store_needs(),
  prepareCheckpoint:(nonce,newSession)=>{try{put(nonce);const n=wasm.sb_store_prepare(+newSession);if(!n)throw Error('capture-invalid');return plan(n,wasm.sb_store_plan_slot());}finally{nonce.fill(0);}},
  publicCommitted:(token,bytes)=>{put(bytes);const okay=!!wasm.sb_store_public_committed(token,bytes.length);if(okay)wasm.sb_store_commit_clock(Math.floor(clock())>>>0);return okay;},
  publicFailed:()=>wasm.sb_store_public_failed(),
  capturePending:()=>!!wasm.sb_store_capture_pending(),
  captureStep:n=>['idle','more','ready','obsolete'][wasm.sb_store_step(n)]||'obsolete',
  privatePlan:()=>plan(wasm.sb_store_private_plan(),wasm.sb_store_private_slot()),
  privateStillCurrent:(token,pub,priv)=>{verify(pub,priv);return !!wasm.sb_store_private_current(token);},
  privateCommitted:(token,bytes)=>{put(bytes);return !!wasm.sb_store_private_committed(token,bytes.length);},
  privateObsolete:()=>wasm.sb_store_private_failed(),privateFailed:()=>wasm.sb_store_private_failed(),
  startNew:seed=>{put(seed);wasm.sb_store_fresh_seed();wasm.sb_store_start_new();},startUnsaved:seed=>{put(seed);wasm.sb_store_fresh_seed();wasm.sb_store_start_unsaved();},
  restoreLatest:(pub,priv,now)=>{verify(pub,priv);return !!wasm.sb_store_restore(Math.floor(now)>>>0);},
  refuseResume:()=>wasm.sb_store_refuse_resume()
 });
}
