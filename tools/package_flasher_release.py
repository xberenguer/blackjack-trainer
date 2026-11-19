#!/usr/bin/env python3
"""Package reviewed public build outputs for the signed StickS3 flasher contract."""
import argparse,hashlib,json,subprocess,struct,tomllib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
BOOT_SHA='1776e4dd896a69d0a5c2e79957b0e2a88aa4129b1381d6478683515a1f6af343'
OTA_SHA='f94c5d786a7a8fab06ac5d10e33bf37711a6697636dc037559ea19cc410a17f0'
def sha(data):return hashlib.sha256(data).hexdigest()
def partition_rows(data):
 rows=[]
 for at in range(0,len(data),32):
  if data[at:at+2]!=b'\xaaP':break
  rows.append({'type':data[at+2],'subtype':data[at+3],'offset':int.from_bytes(data[at+4:at+8],'little'),'size':int.from_bytes(data[at+8:at+12],'little'),'label':data[at+12:at+28].split(b'\0')[0].decode('ascii'),'flags':int.from_bytes(data[at+28:at+32],'little')})
 return rows

def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--bootloader',type=Path,required=True);p.add_argument('--ota',type=Path,required=True);p.add_argument('--out',type=Path,required=True);a=p.parse_args()
 version=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version'];tag='v'+version
 dirty=subprocess.check_output(['git','-C',str(ROOT),'status','--porcelain'],text=True);assert not dirty,'Commit the reviewed source before packaging a source-bound release.'
 commit=subprocess.check_output(['git','-C',str(ROOT),'rev-parse','HEAD'],text=True).strip();proof=json.loads((ROOT/'firmware/build/manifest.json').read_text());assert proof['version']==version
 app=(ROOT/'firmware/build/stardust-sticks3.bin').read_bytes();partitions=(ROOT/'firmware/build/partitions.bin').read_bytes();boot=a.bootloader.read_bytes();ota=a.ota.read_bytes()
 assert len(boot)==15104 and sha(boot)==BOOT_SHA and len(ota)==8192 and sha(ota)==OTA_SHA,'Boot/OTA must be the reviewed public-build material, never a device dump.'
 assert sha(app)==proof['artifacts']['stardust-sticks3.bin']['sha256'] and len(app)==proof['artifacts']['stardust-sticks3.bin']['bytes']
 assert sha(partitions)==proof['artifacts']['partitions.bin']['sha256'] and len(partitions)==3072
 assert sha(partitions)=='65b87532f84d5ddec377c550f8bef2fcc846a3aea884cf8c0f25f1b12d3f421a','Unexpected Blackjack partition contract.'
 audit=proof['linked_stack_audit'];assert audit['unknown_sp_mutations']==0 and audit['app_decode_frame_bytes']<=audit['decoder_cap'] and audit['maximum_frame_bytes']<=audit['explicit_existing_path_cap']
 source_inputs=proof.get('source_inputs');assert isinstance(source_inputs,dict) and len(source_inputs)>30,'Rebuild with source fingerprint support.'
 for name,digest in source_inputs.items():
  source=ROOT/name;assert not source.is_symlink() and source.is_file() and ROOT in source.resolve().parents and sha(source.read_bytes())==digest,'Build source changed: '+name
 assert app[0]==0xe9 and struct.unpack_from('<H',app,12)[0]==9 and app[3]>>4==3
 assert app[32:36]==b'2T\xcd\xab','Missing standard ESP application descriptor.'
 assert app[48:80].split(b'\0')[0].decode()==version,'Embedded app version does not match release.'
 assert len(app)<=0x600000 and proof['iram_dram_overlap_bytes']==0 and proof['main_frame_bytes']<32768 and proof['stack_bytes']>=262144
 install=bytearray(b'\xff'*(0x10000+len(app)));install[:len(boot)]=boot;install[0x8000:0x8000+len(partitions)]=partitions;install[0xe000:0x10000]=ota;install[0x10000:]=app
 assert all(x==255 for x in install[0x9000:0xe000])
 names=['blackjack-sticks3-app.bin','blackjack-sticks3-install.bin'];payloads=[app,bytes(install)]
 parts=[{'file':names[1],'sourceOffset':o,'length':n,'address':o,'kind':k}for o,n,k in[(0,0x8000,'bootloader'),(0x8000,0x1000,'partitions'),(0xe000,0x2000,'ota')]]+[{'file':names[0],'sourceOffset':0,'length':len(app),'address':0x10000,'kind':'application'}]
 manifest={'schemaVersion':1,'project':'blackjack','repository':'xberenguer/blackjack-trainer','tag':tag,'commit':commit,'releases':[{'device':'sticks3','version':version,'contract':'blackjack-sticks3-v1','files':[{'id':name,'url':name,'bytes':len(data),'sha256':sha(data)}for name,data in zip(names,payloads)],'parts':parts,'partitions':partition_rows(partitions)}]}
 out=a.out.resolve();assert out!=ROOT and ROOT not in out.parents,'Release artifacts must stay outside the source checkout.';assert not out.exists(),'Choose a new release staging directory.';out.mkdir(parents=True)
 for name,data in zip(names,payloads):(out/name).write_bytes(data)
 raw=(json.dumps(manifest,indent=2)+'\n').encode();(out/'flasher-release.json').write_bytes(raw)
 (out/'SHA256SUMS').write_text(''.join(sha(data)+'  '+name+'\n'for name,data in zip(names+['flasher-release.json'],payloads+[raw])))
 print(json.dumps({'version':version,'tag':tag,'commit':commit,'output':str(out),'app_bytes':len(app),'app_sha256':sha(app),'install_bytes':len(install),'install_sha256':sha(install),'public_boot_source':BOOT_SHA,'public_OTA_source':OTA_SHA},indent=2))
if __name__=='__main__':main()
