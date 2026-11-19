#!/usr/bin/env python3
"""Audit LINKED ELF32 Xtensa stack frames, including literal/sub/movsp frames.

No build, serial or device access. Never treats disassembled literal pools outside
STT_FUNC ranges as code. Per-function bounds are not a whole-program stack proof;
resolved call chains, unresolved call sites and cycles are reported separately.
"""
from __future__ import annotations
import argparse,bisect,hashlib,json,re,shutil,struct,subprocess
from pathlib import Path

DEFAULT_OBJDUMP=Path.home()/'.platformio/packages/toolchain-xtensa-esp32s3/bin/xtensa-esp32s3-elf-objdump'
class Elf:
 def __init__(self,path):
  self.path=Path(path);self.data=self.path.read_bytes();b=self.data
  if b[:6]!=b'\x7fELF\x01\x01' or struct.unpack_from('<H',b,18)[0]!=94:raise ValueError('requires little-endian ELF32 Xtensa')
  if struct.unpack_from('<H',b,16)[0]!=2:raise ValueError('requires fully linked ET_EXEC ELF')
  off=struct.unpack_from('<I',b,32)[0];size,n,shstr=struct.unpack_from('<HHH',b,46)
  self.sections=[struct.unpack_from('<10I',b,off+i*size)for i in range(n)]
  self.symbols=[];self.absolute={}
  for sec in self.sections:
   if sec[1]!=2:continue
   st=self.sections[sec[6]];names=b[st[4]:st[4]+st[5]]
   for at in range(sec[4],sec[4]+sec[5],sec[9]):
    name,value,size,info,other,idx=struct.unpack_from('<IIIBBH',b,at)
    name=names[name:].split(b'\0',1)[0].decode(errors='replace')
    self.absolute[name]=value
    if info&15==2 and idx<len(self.sections) and self.sections[idx][2]&4:
     self.symbols.append({'name':name,'address':value,'size':size,'section':idx})
 def word(self,address):
  for s in self.sections:
   if s[1]!=8 and s[2]&2 and s[3]<=address and address+4<=s[3]+s[5]:return struct.unpack_from('<I',self.data,s[4]+address-s[3])[0]
  raise ValueError(f'literal0x{address:x} is not in loaded ELF bytes')

def instructions(text):
 result=[]
 for line in text.splitlines():
  m=re.match(r'^\s*([0-9a-f]+):\s+([0-9a-f]{4,8})\s+([a-z0-9_.]+)\s*(.*?)\s*$',line)
  if m:result.append({'address':int(m[1],16),'bytes':m[2],'op':m[3],'args':m[4],'text':line.strip()})
 return result

def hint(name):
 suffixes=['resume_read','read_into','load','resume_valid','main']
 if name=='main':return name
 for suffix in suffixes:
  if name.endswith(str(len(suffix))+suffix):
   prefix='App'if '3App' in name else next((m+'::Model'for m in ['wager_practice','deviation_drills','training','count_practice','session_metrics','session_analysis']if m in name),'')
   return prefix+'::'+suffix
 if name.endswith('4Wire3get'):
  return next((m+'::Wire::get'for m in ['wager_practice','deviation_drills','training','count_practice']if m in name),'Wire::get')
 return name

def signed(n):return n-(1<<32)if n&(1<<31)else n
# Abstract value: (coefficient of function-entry SP, signed32 constant).
def constant(n):return (0,signed(n&0xffffffff))
def combine(a,b,subtract=False):
 if a is None or b is None:return None
 c=a[0]+(-b[0]if subtract else b[0]);v=a[1]+(-b[1]if subtract else b[1])
 return (c,signed(v&0xffffffff))if c in(0,1)else None

def analyze_function(elf,symbol,code):
 regs={'a1':(1,0)};peak=0;entry=0;events=[];unresolved=[];prologue=True;calls=[];indirect=[]
 for ins in code:
  op=ins['op'];args=[x.strip()for x in ins['args'].split(',')];dest=args[0]if args else''
  oldsp=regs.get('a1');value=None;writes=False
  try:
   if op=='entry':
    n=int(args[1],0);entry+=n;value=combine(regs.get('a1'),constant(n),True);writes=True
   elif op in('movi','movi.n'):value=constant(int(args[1],0));writes=True
   elif op=='l32r':
    literal=int(re.match(r'[0-9a-f]+',args[1])[0],16);word=elf.word(literal);value=constant(word);writes=True
    ins={**ins,'literal_address':hex(literal),'literal_u32':word}
   elif op in('mov','mov.n','movsp'):value=regs.get(args[1]);writes=True
   elif op=='neg':
    source=regs.get(args[1]);value=constant(-source[1])if source and source[0]==0 else None;writes=True
   elif op in('addi','addi.n','addmi'):value=combine(regs.get(args[1]),constant(int(args[2],0)));writes=True
   elif op in('add','add.n','sub'):value=combine(regs.get(args[1]),regs.get(args[2]),op=='sub');writes=True
   elif op in('l32i','l32i.n'):
    base=combine(regs.get(args[1]),constant(int(args[2],0)));value=constant(elf.word(base[1]&0xffffffff))if base and base[0]==0 else None;writes=True
   elif op in('addx2','addx4','addx8'):
    a=regs.get(args[1]);b=regs.get(args[2]);factor=int(op[-1]);value=combine((a[0]*factor,a[1]*factor)if a else None,b);writes=True
   elif re.match(r'^(?:l[0-9]|mov|add|sub|and|or|xor|sll|srl|sra|extui|neg|abs|mul|quos|quou|rem|clamp|min|max)',op):writes=True
  except (ValueError,IndexError,AttributeError) as e:
   value=None;writes=True
   if dest=='a1':unresolved.append({'address':hex(ins['address']),'reason':str(e),'instruction':ins['text']})
  if writes:
   if dest=='a1':
    if not prologue and op!='entry':unresolved.append({'address':hex(ins['address']),'reason':'SP mutation outside straight-line function prologue','instruction':ins['text']})
    if value is None or value[0]!=1:unresolved.append({'address':hex(ins['address']),'reason':'unresolved/non-affine SP allocation','instruction':ins['text']})
    else:peak=max(peak,-value[1]);events.append({'address':hex(ins['address']),'instruction':ins['text'],'sp_below_entry_bytes':-value[1]})
   regs[dest]=value
  if op.startswith('call'):
   if op.startswith('callx'):
    value=regs.get(dest)
    if value and value[0]==0:calls.append({'address':hex(ins['address']),'target':value[1]&0xffffffff,'kind':'literal-resolved-indirect'})
    else:indirect.append({'address':hex(ins['address']),'instruction':ins['text']})
   else:
    m=re.match(r'[0-9a-f]+',dest)
    if m:calls.append({'address':hex(ins['address']),'target':int(m[0],16),'kind':'direct'})
   prologue=False;regs={'a1':oldsp if oldsp else regs.get('a1')}
  elif op.startswith('b')or op in('j','jx','ret','ret.n','retw','retw.n','rfi','rfde','rfe'):
   prologue=False;regs={'a1':regs.get('a1')}
 return {'name':symbol['name'],'hint':hint(symbol['name']),'address':hex(symbol['address']),'size':symbol['size'],'frame_bytes':peak,'entry_bytes':entry,'additional_bytes':peak-entry,'sp_events':events,'unresolved_sp':unresolved,'calls':calls,'unresolved_calls':indirect,'prologue':code[:min(10,len(code))]}

def audit_elf(path,objdump=None,limit=32768,decoder_limit=8192,allow_existing=False):
 elf=Elf(path);tool=str(objdump or DEFAULT_OBJDUMP)
 text=subprocess.check_output([tool,'-d',str(path)],text=True)
 code=sorted(instructions(text),key=lambda i:i['address']);addresses=[i['address']for i in code];results=[];zero=[]
 for sym in elf.symbols:
  if not sym['size']:zero.append(sym);continue
  subset=code[bisect.bisect_left(addresses,sym['address']):bisect.bisect_left(addresses,sym['address']+sym['size'])]
  result=analyze_function(elf,sym,subset)
  if not subset:result['unresolved_sp'].append({'reason':'no instructions found for defined function symbol'})
  results.append(result)
 if not results:raise ValueError('No sized executable function symbols; refusing vacuous/stripped ELF audit')
 # Known existing large, bounded paths remain visible and are never exempt from64KiB.
 for r in results:
  r['is_decoder']=r['hint']=='App::resume_read'or'4Wire'in r['name']and('9read_into'in r['name']or'3get'in r['name'])
  r['cap_bytes']=decoder_limit if r['is_decoder']else limit
  r['exception']=None
  if allow_existing and(r['hint']=='App::load'or r['hint']=='wager_practice::Model::resume_valid'):
   r['cap_bytes']=65536;r['exception']='Explicit existing non-decoder bounded path; <=64KiB, still listed'
  r['passed']=r['frame_bytes']<=r['cap_bytes']and not r['unresolved_sp']
 byaddr={int(r['address'],16):r for r in results}
 for r in results:
  for c in r['calls']:c['target_name']=byaddr.get(c['target'],{}).get('hint','external/ROM/interior target')
 roots=[r for r in results if r['hint']=='App::resume_read'or r['name'].endswith('17restore_candidate')or r['name'].endswith('13restore_bound')or r['hint']=='App::resume_valid']
 cache={}
 def chain(r,seen):
  key=r['address']
  if key in cache:return cache[key]
  if key in seen:return {'bytes':0,'path':[],'cycles':[r['hint']]}
  best={'bytes':0,'path':[],'cycles':[]}
  for address in {c['target'] for c in r['calls']}:
   target=byaddr.get(address)
   if target:
    candidate=chain(target,seen|{key})
    if candidate['bytes']>best['bytes']:best=candidate
  result={'bytes':r['frame_bytes']+best['bytes'],'path':[r['hint']]+best['path'],'cycles':best['cycles']}
  cache[key]=result
  return result
 def reach(root):
  seen=set();todo=[root];unknown=[];external=[]
  while todo:
   r=todo.pop();key=r['address']
   if key in seen:continue
   seen.add(key)
   unknown.extend({'caller':r['hint'],**c}for c in r['unresolved_calls'])
   for c in r['calls']:
    target=byaddr.get(c['target'])
    if target:todo.append(target)
    else:external.append({'caller':r['hint'],**c})
  return {'reachable_functions':len(seen),'unresolved_calls':unknown,'external_or_rom_calls':external}
 chains=[{'root':r['hint'],**chain(r,set()),**reach(r)}for r in roots]
 stack=elf.absolute.get('_stack_start',0)-elf.absolute.get('_stack_end',0)
 naked=[]
  # Naked assembly aliases have zero symbol sizes. Infer only their code extents;
  # absolute SP bootstrap/context restore is reported separately, never certified.
 for sym in zero:
  if sym['address']in byaddr:continue
  if any(r['address']==hex(sym['address'])for r in naked):continue
  sec=elf.sections[sym['section']];start=sym['address']
  boundaries=[other['address']for other in elf.symbols if other['section']==sym['section']and other['address']>start]
  end=min(boundaries,default=sec[3]+sec[5]);subset=code[bisect.bisect_left(addresses,start):bisect.bisect_left(addresses,end)]
  r=analyze_function(elf,{**sym,'size':end-start},subset);r['scope']='zero-size naked assembly, inferred symbol range; bootstrap/context SP writes require manual review';naked.append(r)
 genuine=[e for r in results for e in r['sp_events']if 'movsp' in e['instruction']]
 return {'elf':str(Path(path).resolve()),'elf_sha256':hashlib.sha256(elf.data).hexdigest(),'stack_bytes':stack,'limit_bytes':limit,'decoder_limit_bytes':decoder_limit,'allow_existing':allow_existing,'functions_audited':len(results),'decoder_functions_audited':sum(r['is_decoder']for r in results),'max_decoder_frame_bytes':max((r['frame_bytes']for r in results if r['is_decoder']),default=0),'naked_assembly_ranges_manual_review':naked,'zero_size_function_symbols_not_audited':zero,'objdump_movsp_count':sum(i['op']=='movsp'for i in code),'genuine_movsp_count':len(genuine),'ignored_movsp_outside_function_symbols':sum(i['op']=='movsp'for i in code)-len(genuine),'passed':all(r['passed']for r in results)and all(r['frame_bytes']<=limit for r in naked),'failures':[r for r in results if not r['passed']],'large_frames':[r for r in results if r['frame_bytes']>limit],'functions':sorted(results,key=lambda r:r['frame_bytes'],reverse=True),'decode_resolved_call_chains':chains,'call_chain_caveat':'Resolved edges only. Inspect unresolved call sites/cycles before asserting complete runtime stack bound. No recursion/interrupt bound is inferred.'}

def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('elf');p.add_argument('--objdump');p.add_argument('--limit',type=int,default=32768);p.add_argument('--decoder-limit',type=int,default=8192);p.add_argument('--allow-existing',action='store_true');p.add_argument('--json',type=Path);a=p.parse_args()
 report=audit_elf(a.elf,a.objdump,a.limit,a.decoder_limit,a.allow_existing)
 if a.json:a.json.write_text(json.dumps(report,indent=2)+'\n')
 print(f"ELF {report['elf_sha256']} stack={report['stack_bytes']} audited={report['functions_audited']} passed={report['passed']}")
 for r in report['functions'][:18]:print(f"{r['frame_bytes']:>8} B entry={r['entry_bytes']:>5} additional={r['additional_bytes']:>8} cap={r['cap_bytes']:>5} {'OK'if r['passed']else'FAIL'} {r['hint']}")
 for r in report['failures']:
  for issue in r['unresolved_sp']:print('UNRESOLVED',r['hint'],issue)
 for c in report['decode_resolved_call_chains']:print('RESOLVED CHAIN',c['root'],c['bytes'],c['path'],'unresolved_calls',len(c['unresolved_calls']),'external/ROM',len(c['external_or_rom_calls']))
 return 0 if report['passed']else 1
if __name__=='__main__':raise SystemExit(main())
