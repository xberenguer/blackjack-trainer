#!/usr/bin/env python3
"""One-time data migration. Rust builds read committed binary assets, never C++.

Preserves the approved RGB565/alpha sprites, font masks/metrics, and PCM clips.
No game or renderer implementation is translated or included.
"""
import argparse, hashlib, json, re, struct
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('source',type=Path,help='Directory containing generated_art/audio/venetian.h');a=p.parse_args()
root=Path(__file__).resolve().parents[1];dest=root/'assets';dest.mkdir(exist_ok=True)
texts={name:(a.source/name).read_text() for name in ['generated_art.h','generated_venetian.h','generated_audio.h']}
def array(s,name):
 m=re.search(r'\b'+re.escape(name)+r'\[\]\s*=\s*\{(.*?)\};',s,re.S)
 if not m:raise ValueError('Missing data array '+name)
 return m.group(1)
def values(s,name):return [int(v) for v in re.findall(r'-?\d+',array(s,name))]
def rows(s,name):return [v.split(',') for v in re.findall(r'\{([^{}]+)\}',array(s,name))]
def fixed(value):
 n=float(value)*64
 if not n.is_integer():raise ValueError('Non-exact 26.6 font metric: '+value)
 return int(n)
def bank(magic,records):
 data=bytearray(magic+struct.pack('<HH',1,len(records))+bytes(len(records)*4));offsets={}
 for i,record in enumerate(records):
  key=bytes(record)
  if key not in offsets:offsets[key]=len(data);data.extend(record)
  struct.pack_into('<I',data,8+i*4,offsets[key])
 return bytes(data)
art=texts['generated_art.h'];chips=texts['generated_venetian.h'];audio=texts['generated_audio.h']
all_sprites={}
for text in [art,chips]:
 for name,body in re.findall(r'inline const Sprite (\w+)\s*=\s*\{([^}]+)\};',text):
  w,h,pixels,alpha=body.split(',');w,h=int(w),int(h);rgb=values(text,pixels);mask=values(text,alpha)
  assert len(rgb)==len(mask)==w*h and all(0<=v<=65535 for v in rgb) and all(0<=v<=255 for v in mask)
  all_sprites[name]=struct.pack('<HH',w,h)+struct.pack('<'+'H'*len(rgb),*rgb)+bytes(mask)
names=[f'card_big_{i}' for i in range(52)]+[f'card_small_{i}' for i in range(52)]+['back_big','back_small','button_gold','button_dim','stardust_logo']
chip_entries=[]
for cents,diameter,name in rows(chips,'entries'):
 name=name.strip().lstrip('&');chip_entries.append({'cents':int(cents),'diameter':int(diameter),'sprite':len(names)});names.append(name)
(dest/'sprites.bin').write_bytes(bank(b'SART',[all_sprites[n] for n in names]))
fonts=[];font_names=[]
for bold in [False,True]:
 for size in range(7,25):
  name=f'font_{"bold" if bold else "regular"}_{size}'
  m=re.search(r'inline const Font '+name+r'=\{([^}]+)\};',art);g,mask,kern,kcount,actual_size,actual_bold=m.group(1).split(',');assert int(actual_size)==size and actual_bold==str(bold).lower()
  glyphs=rows(art,g);assert len(glyphs)==95;alpha=bytes(values(art,mask));kerning=rows(art,kern)[:int(kcount)]
  record=bytearray(struct.pack('<BBHI',size,bold,len(kerning),len(alpha)))
  for off,w,h,bx,by,advance in glyphs:
   off,w,h,bx,by=map(int,[off,w,h,bx,by]);assert off+w*h<=len(alpha);record.extend(struct.pack('<IHHhhi',off,w,h,bx,by,fixed(advance)))
  for pair,amount in kerning:record.extend(struct.pack('<Hi',int(pair),fixed(amount)))
  record.extend(alpha);fonts.append(record);font_names.append(name)
(dest/'fonts.bin').write_bytes(bank(b'SFNT',fonts))
cue_names=['button','hold1r','carddealt','card5r','bet1r','whandr','tpawrd1r','button','loser1','srtgme1r','crdtlp2r','mxbet2r','cate1r','lstone2r']
clips=[]
for name in cue_names:
 m=re.search(r'inline const Clip '+name+r'=\{([^}]+)\};',audio);samples,count,rate,source=m.group(1).split(',');pcm=values(audio,samples);assert len(pcm)==int(count) and all(-32768<=v<=32767 for v in pcm)
 clips.append(struct.pack('<II',int(rate),len(pcm))+struct.pack('<'+'h'*len(pcm),*pcm))
(dest/'audio.bin').write_bytes(bank(b'SPCM',clips))
metadata={'schema':1,'source_headers':{n:hashlib.sha256(t.encode()).hexdigest() for n,t in texts.items()},'sprites':names,'chip_entries':chip_entries,'fonts':font_names,'audio_cues':cue_names,'banks':{n:{'bytes':(dest/n).stat().st_size,'sha256':hashlib.sha256((dest/n).read_bytes()).hexdigest()} for n in ['sprites.bin','fonts.bin','audio.bin']}}
(dest/'manifest.json').write_text(json.dumps(metadata,indent=2)+'\n')
print(json.dumps(metadata['banks'],indent=2))
