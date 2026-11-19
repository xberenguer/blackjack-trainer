#!/usr/bin/env python3
"""Bounded serial-path play check. Does not claim physical/acoustic acceptance."""
import argparse
import hashlib
import json
import re
import time
import tomllib
from pathlib import Path

import serial

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--port', required=True)
p.add_argument('--board',choices=['sticks3','waveshare175c'],default='sticks3')
p.add_argument('--out', type=Path, required=True)
p.add_argument('--version', default=tomllib.loads((Path(__file__).resolve().parents[1]/'Cargo.toml').read_text())['package']['version'])
a = p.parse_args()
width,height=(466,466)if a.board=='waveshare175c'else(240,135)
frame_bytes=width*height*2
a.out.mkdir(parents=True, exist_ok=True)
s = serial.Serial()
s.port, s.baudrate, s.timeout, s.write_timeout = a.port, 115200, .15, 2
s.dtr = s.rts = False
s.open()
evidence = []

def status():
    deadline = time.monotonic() + 6
    pending = bytearray()
    found = None
    last = 0
    s.write(b's')
    while time.monotonic() < deadline:
        chunk = s.readline()
        if not chunk and found is not None and not pending and time.monotonic() - last > .12:
            assert found['language'] == 'Rust'
            if a.board=='waveshare175c':
                assert found['device']=='waveshare175c' and found['display']=='466x466'
                assert found['orientation']=='270' and found['touch']=='disabled'
            assert found['pmic'] == found['codec'] == found['storage'] == 'true'
            assert found['save_errors'] == '0'
            evidence.append(found)
            return found
        pending.extend(chunk)
        if not pending.endswith(b'\n'):
            continue
        raw = pending.decode('utf8', errors='replace').strip()
        pending.clear()
        if raw.startswith('STARDUST ' + a.version + '-rust ') and 'language=Rust' in raw and re.search(r' assisted=\d+(?: |$)', raw):
            found = dict(re.findall(r'(\w+)=([^ ]+)', raw))
            last = time.monotonic()
        if 'PANIC' in raw or 'BOOT FAILED' in raw:
            raise RuntimeError(raw)
    raise RuntimeError('No responsive Rust status')

def frame(name):
    s.write(b'f')
    deadline = time.monotonic() + (30 if a.board=='waveshare175c'else 8)
    while time.monotonic() < deadline:
        if s.readline().strip() == f'RGB565 {width} {height} {frame_bytes}'.encode():
            break
    else:
        raise RuntimeError('No framebuffer header')
    data = bytearray()
    while len(data) < frame_bytes and time.monotonic() < deadline:
        data.extend(s.read(frame_bytes - len(data)))
    assert len(data) == frame_bytes
    (a.out / (name + '.rgb565')).write_bytes(data)
    evidence.append({'frame': name, 'sha256': hashlib.sha256(data).hexdigest()})

def send(keys, delay=.25):
    s.write(keys)
    time.sleep(delay)
    return status()

try:
    start = status()
    initial_scores = [start[k] for k in ('graded', 'correct', 'errors', 'assisted')]
    assert start['mode'] == 'Off'
    frame('startup')
    if start['phase'] == 'Cut':
        value = send(b'b', .15)
        assert value['view'] == 'Burn'
        value = send(b'b', 1.5)
    else:
        value = send(b'b', 1.5)
    assert value['phase'] in ('Player', 'Insurance', 'Result')
    frame('initial-hand')
    if value['phase'] == 'Insurance':
        value = send(b'b', .4)  # Default no insurance.
    if value['phase'] == 'Player':
        before = value
        value = send(b'g', .4)
        frame('after-hit')
        assert value['frame'] != before['frame']
    if value['phase'] == 'Player':
        value = send(b'b', .4)
    deadline = time.monotonic() + 18
    while value['phase'] == 'Dealer' and time.monotonic() < deadline:
        time.sleep(.35)
        value = status()
    assert value['phase'] == 'Result', value
    assert int(value['rounds']) >= 1
    frame('settled-round')
    value = send(b'm')
    assert value['view'] == 'Menu'
    frame('menu')
    value = send(b'b')  # Result menu begins at Return to game.
    assert value['view'] == 'Table'
    value = send(b'hhh', 1.3)
    assert value['mode'] == 'Guide'
    value = send(b'b', 1.5)
    if value['view'] == 'Cut':
        send(b'b', .15)
        value = send(b'b', 1.5)
    frame('guide')
    value = send(b'h', 1.3)
    assert value['mode'] == 'Off'
    value = send(b'm')
    frame('in-hand-menu')
    # Leave the same hand on screen, preserving the original grading counters.
    value = send(b'm')
    assert value['view'] == 'Table'
    frame('final-table')
    final = status()
    assert [final[k] for k in ('graded', 'correct', 'errors', 'assisted')] == initial_scores
    assert int(final['save_writes']) >= 1
    assert final['audio_underruns'] == '0', final
    (a.out / 'serial-play-check.json').write_text(json.dumps({
        'result': 'pass',
        'scope': 'Actual firmware serial input, native frames, peripheral status, dealer progression and score preservation. Physical buttons and acoustic quality require separate human observation.',
        'evidence': evidence,
    }, indent=2) + '\n')
    print('PASS actual Rust cut/deal/hit/stand/dealer/result/menu/Guide/Off; original scores unchanged')
    print(json.dumps(final))
finally:
    s.close()
