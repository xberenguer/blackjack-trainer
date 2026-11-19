#!/usr/bin/env python3
"""Package source-bound StickS3 and button-only round firmware for the signed flasher."""
import argparse
import hashlib
import json
import struct
import subprocess
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BOOT_SHA = '1776e4dd896a69d0a5c2e79957b0e2a88aa4129b1381d6478683515a1f6af343'
OTA_SHA = 'f94c5d786a7a8fab06ac5d10e33bf37711a6697636dc037559ea19cc410a17f0'
WAVESHARE_MAX_APP = 0x570000
WAVESHARE_PARTITIONS = [dict(type=t, subtype=s, offset=o, size=n, label=l, flags=0) for t, s, o, n, l in [
    (1, 2, 0x9000, 0x32000, 'nvsfactory'), (1, 2, 0x3b000, 0xd2000, 'nvs'),
    (1, 0, 0x10d000, 0x2000, 'otadata'), (1, 1, 0x10f000, 0x1000, 'phy_init'),
    (1, 130, 0x110000, 0xf0000, 'model'), (0, 0, 0x200000, 0x600000, 'factory'),
    (1, 130, 0x800000, 0x800000, 'storage')]]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def partition_rows(data):
    rows = []
    for at in range(0, len(data), 32):
        if data[at:at + 2] != b'\xaaP':
            break
        rows.append({'type': data[at + 2], 'subtype': data[at + 3],
                     'offset': int.from_bytes(data[at + 4:at + 8], 'little'),
                     'size': int.from_bytes(data[at + 8:at + 12], 'little'),
                     'label': data[at + 12:at + 28].split(b'\0')[0].decode('ascii'),
                     'flags': int.from_bytes(data[at + 28:at + 32], 'little')})
    return rows


def check_build(root, directory, version, board, max_bytes, flash_code):
    """Bind the exact binary to its final source and linked stack audit."""
    proof = json.loads((directory / 'manifest.json').read_text())
    assert proof['version'] == version, 'Build version differs from core.'
    name = 'stardust-' + board + '.bin'
    app = (directory / name).read_bytes()
    assert sha(app) == proof['artifacts'][name]['sha256'] and len(app) == proof['artifacts'][name]['bytes']
    audit = proof['linked_stack_audit']
    assert audit['unknown_sp_mutations'] == 0 and audit['app_decode_frame_bytes'] <= audit['decoder_cap']
    assert audit['maximum_frame_bytes'] <= audit['explicit_existing_path_cap']
    source_inputs = proof.get('source_inputs')
    assert isinstance(source_inputs, dict) and len(source_inputs) > 30, 'Rebuild with source fingerprint support.'
    for relative, digest in source_inputs.items():
        source = root / relative
        assert not source.is_symlink() and source.is_file() and root in source.resolve().parents
        assert sha(source.read_bytes()) == digest, 'Build source changed: ' + relative
    assert len(app) > 256 and len(app) <= max_bytes, 'Application exceeds its release write boundary.'
    assert app[0] == 0xe9 and struct.unpack_from('<H', app, 12)[0] == 9 and app[3] >> 4 == flash_code
    assert app[32:36] == b'2T\xcd\xab', 'Missing standard ESP application descriptor.'
    assert app[48:80].split(b'\0')[0].decode() == version, 'Embedded app version does not match release.'
    assert proof['iram_dram_overlap_bytes'] == 0 and proof['main_frame_bytes'] < 32768 and proof['stack_bytes'] >= 262144
    return app, proof


def waveshare_release(root, directory, version):
    app, proof = check_build(root, directory, version, 'waveshare175c', WAVESHARE_MAX_APP, 5)
    assert proof['board'] == 'waveshare175c' and proof['application_offset'] == 0x200000
    assert proof['flash_bytes'] == 0x2000000 and proof['display_orientation_degrees'] == 270
    assert proof['touch_enabled'] is False, 'Round release must use buttons only.'
    assert proof.get('flash_partition_table') is False, 'Factory map must not be flashed.'
    name = 'blackjack-waveshare175c-app.bin'
    row = {'device': 'waveshare175c', 'version': version, 'contract': 'blackjack-waveshare175c-v1',
           'files': [{'id': name, 'url': name, 'bytes': len(app), 'sha256': sha(app)}],
           'parts': [{'file': name, 'sourceOffset': 0, 'length': len(app), 'address': 0x200000, 'kind': 'application'}],
           'partitions': WAVESHARE_PARTITIONS}
    return row, {name: app}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--bootloader', type=Path, required=True)
    p.add_argument('--ota', type=Path, required=True)
    p.add_argument('--waveshare-build', type=Path, default=ROOT / 'firmware/build-waveshare175c')
    p.add_argument('--out', type=Path, required=True)
    a = p.parse_args()
    version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
    tag = 'v' + version
    dirty = subprocess.check_output(['git', '-C', str(ROOT), 'status', '--porcelain'], text=True)
    assert not dirty, 'Commit the reviewed source before packaging a source-bound release.'
    commit = subprocess.check_output(['git', '-C', str(ROOT), 'rev-parse', 'HEAD'], text=True).strip()
    app, proof = check_build(ROOT, ROOT / 'firmware/build', version, 'sticks3', 0x600000, 3)
    partitions = (ROOT / 'firmware/build/partitions.bin').read_bytes()
    boot, ota = a.bootloader.read_bytes(), a.ota.read_bytes()
    assert len(boot) == 15104 and sha(boot) == BOOT_SHA and len(ota) == 8192 and sha(ota) == OTA_SHA, 'Boot/OTA must be reviewed public-build material, never a device dump.'
    assert sha(partitions) == proof['artifacts']['partitions.bin']['sha256'] and len(partitions) == 3072
    assert sha(partitions) == '65b87532f84d5ddec377c550f8bef2fcc846a3aea884cf8c0f25f1b12d3f421a', 'Unexpected Blackjack partition contract.'
    install = bytearray(b'\xff' * (0x10000 + len(app)))
    install[:len(boot)] = boot
    install[0x8000:0x8000 + len(partitions)] = partitions
    install[0xe000:0x10000] = ota
    install[0x10000:] = app
    assert all(x == 255 for x in install[0x9000:0xe000])
    names = ['blackjack-sticks3-app.bin', 'blackjack-sticks3-install.bin']
    payloads = dict(zip(names, [app, bytes(install)]))
    parts = [{'file': names[1], 'sourceOffset': o, 'length': n, 'address': o, 'kind': k} for o, n, k in [
        (0, 0x8000, 'bootloader'), (0x8000, 0x1000, 'partitions'), (0xe000, 0x2000, 'ota')]]
    parts += [{'file': names[0], 'sourceOffset': 0, 'length': len(app), 'address': 0x10000, 'kind': 'application'}]
    rows = [{'device': 'sticks3', 'version': version, 'contract': 'blackjack-sticks3-v1',
             'files': [{'id': n, 'url': n, 'bytes': len(b), 'sha256': sha(b)} for n, b in payloads.items()],
             'parts': parts, 'partitions': partition_rows(partitions)}]
    if tuple(map(int, version.split('.'))) >= (0, 12, 0):
        row, data = waveshare_release(ROOT, a.waveshare_build.resolve(), version)
        rows.append(row)
        payloads.update(data)
    manifest = {'schemaVersion': 1, 'project': 'blackjack', 'repository': 'xberenguer/blackjack-trainer',
                'tag': tag, 'commit': commit, 'releases': rows}
    out = a.out.resolve()
    assert out != ROOT and ROOT not in out.parents, 'Release artifacts must stay outside the source checkout.'
    assert not out.exists(), 'Choose a new release staging directory.'
    out.mkdir(parents=True)
    for name, data in payloads.items():
        (out / name).write_bytes(data)
    raw = (json.dumps(manifest, indent=2) + '\n').encode()
    (out / 'flasher-release.json').write_bytes(raw)
    payloads['flasher-release.json'] = raw
    (out / 'SHA256SUMS').write_text(''.join(sha(data) + '  ' + name + '\n' for name, data in payloads.items()))
    print(json.dumps({'version': version, 'tag': tag, 'commit': commit, 'output': str(out),
                      'devices': [row['device'] for row in rows],
                      'files': {n: {'bytes': len(b), 'sha256': sha(b)} for n, b in payloads.items()},
                      'public_boot_source': BOOT_SHA, 'public_OTA_source': OTA_SHA}, indent=2))


if __name__ == '__main__':
    main()
