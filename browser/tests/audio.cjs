const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
(async()=>{
 const golden=require('./audio-golden.json'),root=path.resolve(__dirname,'..');
 const {instance:{exports:w}}=await WebAssembly.instantiate(fs.readFileSync(path.join(root,'stardust-rust.wasm')),{});
 const hash=pcm=>{let h=14695981039346656037n;for(const n of pcm)for(const b of [n&255,(n>>8)&255])h=BigInt.asUintN(64,(h^BigInt(b))*1099511628211n);return h;};
 let count=0;for(let cue=1;cue<=14;cue++){
  const [rate,samples,rawHash,scaledHash]=golden[cue-1].map(BigInt);assert.equal(BigInt(w.sb_audio_rate(cue)),rate);assert.equal(BigInt(w.sb_audio_samples(cue)),samples);
  const raw=new Int16Array(w.memory.buffer,w.sb_audio_data(cue),Number(samples));assert.equal(hash(raw),rawHash);
  const scaled=new Int16Array(Number(samples));for(let i=0;i<scaled.length;i++)scaled[i]=w.sb_audio_scaled_sample(cue,i);assert.equal(hash(scaled),scaledHash);count+=scaled.length;
  assert.equal(w.sb_audio_scaled_sample(cue,Number(samples)),0);
 }
 for(const cue of [0,15,0xffffffff])assert.equal(w.sb_audio_scaled_sample(cue,0),0);
 console.log(`PASS all14 raw/scaled PCM golden hashes, ${count} samples; original gain master120/channel255, unity WebAudio, bounded sample indices.`);
})().catch(e=>{console.error(e);process.exitCode=1;});
