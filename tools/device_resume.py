#!/usr/bin/env python3
"""Two-stage, announced USB resume check. Never submits a graded user answer."""
import argparse
import hashlib
import json
import re
import time
import tomllib
from pathlib import Path
import serial
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('stage',choices=['prepare','restore-and-disable'])
p.add_argument('--port',required=True)
p.add_argument('--out',type=Path,required=True)
p.add_argument('--version',default=tomllib.loads((Path(__file__).resolve().parents[1]/'Cargo.toml').read_text())['package']['version'])
a=p.parse_args();a.out.mkdir(parents=True,exist_ok=True)
s=serial.Serial();s.port=a.port;s.baudrate=115200;s.timeout=.2;s.write_timeout=2;s.dtr=s.rts=False;s.open()
evidence=[]

def status():
    s.write(b's'); deadline=time.monotonic()+8; pending=bytearray(); found=None; last=0
    while time.monotonic()<deadline:
        chunk=s.readline()
        if not chunk and found is not None and not pending and time.monotonic()-last>.12:
            evidence.append(found)
            assert found['save_errors']==found['resume_errors']=='0',found
            assert found['unsaved']=='false' and found['resume_blocked']=='false',found
            return found
        pending.extend(chunk)
        if not pending.endswith(b'\n'):continue
        line=pending.decode(errors='replace').strip();pending.clear()
        if line.startswith('STARDUST '+a.version+'-rust ') and 'assisted=' in line:
            found=dict(re.findall(r'(\w+)=([^ ]+)',line));last=time.monotonic()
        if 'PANIC' in line or 'BOOT FAILED' in line:raise RuntimeError(line)
    raise RuntimeError('No responsive acknowledged game status')

def send(keys,delay=.25):
    s.write(keys);time.sleep(delay);return status()

def frame(name):
    deadline=time.monotonic()+15;sent=0;lines=[]
    while time.monotonic()<deadline:
        if time.monotonic()-sent>1.5:s.write(b'f');sent=time.monotonic()
        line=s.readline().strip()
        if line==b'RGB565 240 135 64800':break
        if line:lines.append(line.decode(errors='replace'))
    else:
        (a.out/(name+'-startup.log')).write_text('\n'.join(lines)+'\n')
        raise RuntimeError('No safe framebuffer: '+repr(lines[-4:]))
    deadline=time.monotonic()+10
    data=bytearray()
    while len(data)<64800 and time.monotonic()<deadline:data.extend(s.read(64800-len(data)))
    assert len(data)==64800
    (a.out/(name+'.rgb565')).write_bytes(data)
    return hashlib.sha256(data).hexdigest()

def menu(item):
    d=status()
    if d['view']!='Menu':d=send(b'm')
    assert d['view']=='Menu',d
    for _ in range(14):
        if d['focus']=='0':break
        d=send(b'g',.12)
    assert d['focus']=='0',d
    count=0
    while True:
        d=send(b'g',.12);count+=1
        assert d['view']=='Menu' and count<=14,d
        if d['focus']=='0':break
    extra=count-11;assert 0<=extra<=3
    if extra+item:d=send(b'g'*(extra+item))
    assert int(d['focus'])==extra+item,d
    return send(b'b')

def scores(d):return {k:d[k] for k in ('graded','correct','errors','assisted','wager_graded')}

try:
    if a.stage=='prepare':
        start=status();assert start['phase']=='Cut' and start['mode']=='Off' and start['resume_enabled']=='false',start
        d=menu(3);assert d['view']=='Settings'
        d=send(b'ggggg');assert d['focus']=='5'
        d=send(b'b');assert d['resume_enabled']=='true'
        d=menu(0);assert d['view']=='Cut',d
        d=send(b'b',.2);assert d['view']=='Burn'
        d=send(b'b',1.6)
        d=menu(2);assert d['view']=='Training'
        d=send(b'ggb');assert d['view']=='CountPractice'
        d=send(b'b');assert d['view']=='CountAnswer'
        digest=frame('pending-question')
        deadline=time.monotonic()+35
        while d['resume_saved']!='true' and time.monotonic()<deadline:
            time.sleep(.3);d=status()
        assert d['resume_saved']=='true' and int(d['resume_writes'])>=1,d
        assert scores(d)==scores(start)
        report={'scope':'Actual device unsubmitted RC checkpoint, verified private idle commit; no answer graded','start':start,'pending':d,'question_frame_sha256':digest,'evidence':evidence}
        (a.out/'prepared.json').write_text(json.dumps(report,indent=2)+'\n')
        print('PASS pending RC checkpoint committed; no user score changed. Ready for announced reboot.')
    else:
        prior=json.loads((a.out/'prepared.json').read_text())
        chooser_digest=frame('resume-chooser')
        d=send(b'b',.8);assert d['view']=='CountAnswer',d
        digest=frame('restored-question')
        assert digest==prior['question_frame_sha256'],(digest,prior['question_frame_sha256'])
        for k in ('phase','chips','rounds','mode'):assert d[k]==prior['pending'][k],(k,d,prior['pending'])
        assert scores(d)==scores(prior['start'])
        restored=d
        d=menu(3);assert d['view']=='Settings'
        d=send(b'ggggg');assert d['focus']=='5'
        d=send(b'b');assert d['resume_enabled']=='false'
        time.sleep(2);d=status()
        assert scores(d)==scores(prior['start'])
        frame('resume-disabled')
        report={'passed':True,'scope':'Actual boot chooser and restored pending RC frame/state; resume then disabled; lifetime scores unchanged. No user answer submitted.','chooser_frame_sha256':chooser_digest,'restored':restored,'after_disable':d,'question_frame_sha256':digest,'evidence':evidence}
        (a.out/'restore.json').write_text(json.dumps(report,indent=2)+'\n')
        print('PASS exact visible pending question restored, scores unchanged, resume disabled. Ready for final fresh reboot.')
finally:
    (a.out/(a.stage+'-status.json')).write_text(json.dumps(evidence,indent=2)+'\n')
    s.close()
