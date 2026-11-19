/* Browser I/O only. Opaque record validation, binding and game/UI decisions are Rust. */
export class JournalHost {
  #storage; #codec; #locks; #random; #clock; #keys; #state='loading';
  #blocked=false; #publicDurable=false; #privateAvailable=false; #pumping=false;
  constructor({storage,codec,locks,random,clock=()=>performance.now(),keys={
    public:['stardust-blackjack.public.a','stardust-blackjack.public.b'],
    private:['stardust-blackjack.private.a','stardust-blackjack.private.b'],
    legacy:'stardust-blackjack-userdata'
  }}) { this.#storage=storage;this.#codec=codec;this.#locks=locks;this.#random=random;this.#clock=clock;this.#keys=keys; }
  status(){return Object.freeze({state:this.#state,publicDurable:this.#publicDurable,checkpointAvailable:this.#privateAvailable});}
  #nonce(){const bytes=new Uint8Array(16);this.#random(bytes);if(bytes.every(n=>n===0))throw Error('entropy');return bytes;}
  async #locked(action){
    if(!this.#locks?.request)throw Error('locking-unavailable');
    return this.#locks.request('stardust-blackjack-storage',{mode:'exclusive',ifAvailable:true},lock=>{if(!lock)throw Error('storage-busy');return action();});
  }
  #decode(value,capacity){
    if(value===null)return null;
    if(typeof value!=='string'||!value.startsWith('sbj1:')||value.length>5+Math.ceil(capacity/3)*4)throw Error('record-format');
    const raw=atob(value.slice(5));if(raw.length>capacity||btoa(raw)!==value.slice(5))throw Error('record-format');
    const bytes=new Uint8Array(raw.length);for(let i=0;i<raw.length;i++)bytes[i]=raw.charCodeAt(i);return bytes;
  }
  #encode(bytes){let raw='';for(let at=0;at<bytes.length;at+=2048)raw+=String.fromCharCode(...bytes.subarray(at,at+2048));return 'sbj1:'+btoa(raw);}
  #read(kind){
    const values=[];
    for(let slot=0;slot<2;slot++){
      let text;try{text=this.#storage.getItem(this.#keys[kind][slot]);}catch{return {io:true,values:[]};}
      try{values.push(this.#decode(text,this.#codec.capacity(kind)));}catch{values.push('corrupt');}
    }
    return {io:false,values};
  }
  #feed(kind,result){
    if(result.io){this.#codec.readFailure(kind);return false;}
    for(let slot=0;slot<2;slot++){const bytes=result.values[slot];try{this.#codec.loadSlot(kind,slot,bytes);}finally{if(bytes instanceof Uint8Array)bytes.fill(0);}}
    return true;
  }
  #legacy(){
    const text=this.#storage.getItem(this.#keys.legacy);if(text===null)return;
    let array;try{array=JSON.parse(text);}catch{throw Error('legacy-invalid');}
    if(!Array.isArray(array)||array.length>this.#codec.legacyCapacity()||!array.every(n=>Number.isInteger(n)&&n>=0&&n<=255))throw Error('legacy-invalid');
    if(!this.#codec.loadLegacy(Uint8Array.from(array)))throw Error('legacy-invalid');
  }
  async boot(){
    this.#blocked=false;this.#publicDurable=false;this.#privateAvailable=false;this.#state='loading';
    try{await this.#locked(()=>{
      this.#codec.beginLoad();const publicRead=this.#read('public');if(!this.#feed('public',publicRead))throw Error('public-read');
      if(publicRead.values.every(v=>v===null))this.#legacy();
      if(!this.#codec.finishPublic())throw Error('public-invalid');
      this.#publicDurable=true;
      if(this.#codec.wantsPrivate()){const read=this.#read('private');if(!this.#feed('private',read))throw Error('private-read');}
      const state=this.#codec.finishBoot(this.#clock());this.#privateAvailable=state==='resume-available';this.#state=state;
    });}catch{this.#blocked=true;this.#state='read-unavailable';this.#codec.bootReadFailure();}
    return this.status();
  }
  #verifyBase(){const read=this.#read('public');if(read.io||read.values.includes('corrupt'))throw Error('public-read');try{if(!this.#codec.baseMatches(read.values))throw Error('public-conflict');}finally{for(const v of read.values)if(v instanceof Uint8Array)v.fill(0);}}
  #write(kind,slot,bytes){
    if(slot!==0&&slot!==1)throw Error('slot');const text=this.#encode(bytes);this.#storage.setItem(this.#keys[kind][slot],text);
    if(this.#storage.getItem(this.#keys[kind][slot])!==text)throw Error('readback');
  }
  async checkpoint({newSession=false}={}){
    if(this.#blocked||!this.#codec.needsCheckpoint())return this.status();let plan;
    try{await this.#locked(()=>{
      this.#verifyBase();plan=this.#codec.prepareCheckpoint(this.#nonce(),newSession);if(!plan)return;
      this.#state='saving-public';this.#publicDurable=false;
      this.#write('public',plan.slot,plan.bytes);
      if(!this.#codec.publicCommitted(plan.token,plan.bytes))throw Error('public-readback');
      this.#publicDurable=true;this.#privateAvailable=false;this.#state=this.#codec.wantsPrivate()?(this.#codec.capturePending()?'saving-checkpoint':'scores-saved-checkpoint-unavailable'):'saved';
    });}catch{this.#blocked=true;this.#publicDurable=false;this.#privateAvailable=false;this.#state='public-unsaved';this.#codec.publicFailed(plan?.token);}
    return this.status();
  }
  async retryCheckpoint(){
    if(this.#state!=='public-unsaved')return this.status();let result=0;
    try{await this.#locked(()=>{const read=this.#read('public');if(read.io)throw Error('read');try{result=this.#codec.recoverBase(read.values);}finally{for(const v of read.values)if(v instanceof Uint8Array)v.fill(0);}});
      if(result){this.#blocked=false;if(result===2){this.#publicDurable=true;this.#privateAvailable=false;this.#state=this.#codec.capturePending()?'saving-checkpoint':'saved';}else return this.checkpoint();}
    }catch{/* Keep the acknowledged frame/grade gate closed; no fallback or overwrite. */}
    return this.status();
  }
  async pump({budgetMs=3,maxSteps=64}={}){
    if(this.#blocked||this.#pumping||!this.#codec.capturePending())return this.status();this.#pumping=true;
    let plan;
    try{
      const start=this.#clock();for(let step=0;step<Math.min(maxSteps,256)&&this.#clock()-start<Math.max(.1,budgetMs);step++){
        const progress=this.#codec.captureStep(512);if(progress==='ready'){plan=this.#codec.privatePlan();break;}if(progress==='obsolete'||progress==='idle'){this.#privateAvailable=false;this.#state='scores-saved-checkpoint-unavailable';break;}
      }
      if(plan){
        // Detach the internal transport buffer before awaiting an origin lock.
        const bytes=plan.bytes.slice();try{await this.#locked(()=>{
          const publicRead=this.#read('public');const privateRead=this.#read('private');if(publicRead.io||privateRead.io)throw Error('private-read');
          let current;try{current=this.#codec.privateStillCurrent(plan.token,publicRead.values,privateRead.values);}finally{for(const v of [...publicRead.values,...privateRead.values])if(v instanceof Uint8Array)v.fill(0);}
          if(!current){this.#codec.privateObsolete(plan.token);this.#privateAvailable=false;this.#state='scores-saved-checkpoint-unavailable';return;}
          this.#write('private',plan.slot,bytes);if(!this.#codec.privateCommitted(plan.token,bytes))throw Error('private-readback');
          this.#privateAvailable=true;this.#state='saved';
        });}finally{bytes.fill(0);}
      }
    }catch{this.#privateAvailable=false;this.#state=this.#publicDurable?'scores-saved-checkpoint-unavailable':'public-unsaved';this.#codec.privateFailed(plan?.token);}
    finally{this.#pumping=false;}
    return this.status();
  }
  async command(command){
    if(command==='retry-read')return this.boot();
    if(command==='retry-checkpoint')return this.retryCheckpoint();
    if(command==='start-unsaved'){
      // A failed-storage recovery never mutates either journal. The fresh RAM
      // session discards pending private state and cannot write/resume this boot.
      const seed=this.#nonce();try{this.#codec.startUnsaved(seed);}finally{seed.fill(0);}
      this.#blocked=true;this.#privateAvailable=false;this.#state='playing-unsaved';return this.status();
    }
    if(command==='start-new'){if(this.#blocked)return this.command('start-unsaved');const seed=this.#nonce();try{this.#codec.startNew(seed);}finally{seed.fill(0);}return this.checkpoint({newSession:true});}
    if(command==='resume'){
      try{await this.#locked(()=>{const pub=this.#read('public'),priv=this.#read('private');if(pub.io||priv.io)throw Error('read');try{if(!this.#codec.restoreLatest(pub.values,priv.values,this.#clock()))throw Error('stale');}finally{for(const v of [...pub.values,...priv.values])if(v instanceof Uint8Array)v.fill(0);}this.#publicDurable=true;this.#privateAvailable=true;this.#state='resumed';});}
      catch{this.#blocked=true;this.#privateAvailable=false;this.#state='resume-refused';this.#codec.refuseResume();}
    }
    return this.status();
  }
}
