#!/usr/bin/env python3
"""Exercise fixed public training fixtures in spare PSRAM, never the live game."""
import argparse
import json
import re
import time
import tomllib
from pathlib import Path
import serial

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--port', required=True)
p.add_argument('--out', type=Path, required=True)
p.add_argument('--version', default=tomllib.loads((Path(__file__).resolve().parents[1] / 'Cargo.toml').read_text())['package']['version'])
a = p.parse_args()
a.out.mkdir(parents=True, exist_ok=True)
lines = []
s = serial.Serial()
s.port, s.baudrate, s.timeout, s.write_timeout = a.port, 115200, .2, 2
s.dtr = s.rts = False
s.open()

def fields(line):
    return dict(re.findall(r'(\w+)=([^ ]+)', line))

def read_status():
    s.write(b's')
    deadline = time.monotonic() + 8
    pending = bytearray()
    while time.monotonic() < deadline:
        pending.extend(s.readline())
        if not pending.endswith(b'\n'):
            continue
        line = pending.decode(errors='replace').strip()
        pending.clear()
        if line:
            lines.append(line)
        if line.startswith('STARDUST ' + a.version + '-rust ') and 'assisted=' in line:
            return fields(line)
        if 'PANIC' in line or 'BOOT FAILED' in line:
            raise RuntimeError(line)
    raise RuntimeError('No complete versioned status')

try:
    before = read_status()
    time.sleep(.2)
    s.read_all()  # Drain the completed status before direct fixture diagnostics.
    s.write(b'p')
    cases = []
    complete = None
    pending = bytearray()
    deadline = time.monotonic() + 55
    while time.monotonic() < deadline:
        pending.extend(s.readline())
        if not pending.endswith(b'\n'):
            continue
        line = pending.decode(errors='replace').strip()
        pending.clear()
        if line:
            lines.append(line)
        if 'STARDUST BENCH case=' in line:
            line = line[line.index('STARDUST BENCH case='):]
            case = fields(line)
            cases.append(case)
            print(line, flush=True)
        if line.startswith('STARDUST BENCH COMPLETE '):
            complete = fields(line)
            print(line, flush=True)
            break
        if 'PANIC' in line or 'BOOT FAILED' in line:
            raise RuntimeError(line)
    assert complete, 'Fixed-fixture benchmark did not finish'
    after = read_status()
    assert len(cases) == 18
    assert {(c['mode'], int(c['case'])) for c in cases} == {(m, n) for m in ('Guide', 'Test', 'Coach') for n in range(6)}
    assert all(c['graded'] == '1' for c in cases)
    assert all(c['audio_underruns'] == '0' for c in cases), cases
    assert complete['audio_underruns'] == '0', complete
    assert complete['live_state_unchanged'] == complete['fixed_cases'] == 'true'
    keys = ('phase', 'chips', 'rounds', 'shoe', 'view', 'focus', 'mode', 'graded', 'correct', 'errors', 'assisted', 'wager_graded')
    assert {k: before[k] for k in keys} == {k: after[k] for k in keys}, 'Live public state changed'
    result = {'passed': True, 'scope': 'Actual device fixed-fixture training/EV event timing and DMA; live public game/score fields unchanged. Not acoustic acceptance.', 'before': before, 'cases': cases, 'complete': complete, 'after': after}
    (a.out / 'benchmark.json').write_text(json.dumps(result, indent=2) + '\n')
    print('PASS 18 fixed training fixtures, no DMA underruns, live public game and scores unchanged')
finally:
    (a.out / 'benchmark.log').write_text('\n'.join(lines) + '\n')
    s.close()
