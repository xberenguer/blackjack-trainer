#!/usr/bin/env python3
"""Bounded read-only boot/status/frame verification of the authorized Stick."""
import argparse,json,re,time,tomllib
from pathlib import Path
import serial
p=argparse.ArgumentParser();p.add_argument('--port',required=True);p.add_argument('--out',type=Path,required=True);p.add_argument('--version',default=tomllib.loads((Path(__file__).resolve().parents[1]/'Cargo.toml').read_text())['package']['version']);p.add_argument('--min-decisions',type=int,default=0);a=p.parse_args();a.out.mkdir(parents=True,exist_ok=True)
s=serial.Serial();s.port=a.port;s.baudrate=115200;s.timeout=.15;s.write_timeout=2;s.dtr=False;s.rts=False;s.open()
lines=[];status='';pending=bytearray()
try:
 deadline=time.monotonic()+12;sent=0
 while time.monotonic()<deadline:
  if time.monotonic()-sent>.7:s.write(b's');sent=time.monotonic()
  pending.extend(s.readline())
  if not pending.endswith(b'\n'):continue
  line=pending.decode('utf8',errors='replace').strip();pending.clear()
  if line:lines.append(line)
  if line.startswith('STARDUST '+a.version+'-rust ') and 'language=Rust' in line and re.search(r' assisted=\d+(?: |$)',line):status=line;break
 (a.out/'boot-status.log').write_text('\n'.join(lines)+'\n')
 if not status:raise RuntimeError('No verified Rust status: '+repr(lines[-8:]))
 s.write(b'f');deadline=time.monotonic()+10
 while time.monotonic()<deadline:
  line=s.readline()
  if line.startswith(b'RGB565 '):break
 else:raise RuntimeError('No framebuffer header')
 if line.strip()!=b'RGB565 240 135 64800':raise RuntimeError('Unexpected frame '+repr(line))
 frame=bytearray()
 while len(frame)<64800 and time.monotonic()<deadline:
  frame.extend(s.read(64800-len(frame)))
 if len(frame)!=64800:raise RuntimeError('Truncated frame '+str(len(frame)))
 (a.out/'frame.rgb565').write_bytes(frame)
 fields=dict(re.findall(r'(\w+)=([^ ]+)',status));assert fields['language']=='Rust';assert fields['pmic']=='true' and fields['codec']=='true';assert fields['storage']=='true';assert int(fields['graded'])>=a.min_decisions;assert int(fields['graded'])==sum(int(fields[k]) for k in ('correct','errors','assisted'));assert fields['mode']=='Off'
 (a.out/'device-probe.json').write_text(json.dumps({'status':status,'fields':fields,'frame_bytes':len(frame),'scope':'Actual boot/PMIC/codec/storage/frame; no physical button presses or acoustic acceptance claimed'},indent=2)+'\n')
 print(status);print('PASS actual Rust boot, original scores retained, PMIC/codec/storage and64800-byte framebuffer')
finally:s.close()
