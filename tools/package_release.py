#!/usr/bin/env python3
"""Seal a portable release while excluding device/private and generated local state."""
from pathlib import Path
import hashlib
import json
import tomllib
import zipfile

root = Path(__file__).resolve().parents[1]
version = tomllib.loads((root / 'Cargo.toml').read_text())['package']['version']
audit = json.loads((root / 'docs/requirements-audit.json').read_text())
if not audit.get('goal_complete') or audit.get('release') != version:
    raise SystemExit('Refusing to seal: final requirement/device audit is not complete for this version.')
output = root.parent / ('stardust-blackjack-rust-' + version + '.zip')
blocked = {'recovery', 'releases', '.git', '.vendor', '.cargo', 'cargo-home', '__pycache__', '.DS_Store'}
files = []
for p in root.rglob('*'):
    rel = p.relative_to(root)
    if not p.is_file() or p.is_symlink():
        continue
    if any(x in blocked or x.startswith('target') for x in rel.parts):
        continue
    if any(x.startswith('device-') for x in rel.parts) or p.suffix in {'.pyc', '.tmp'}:
        continue
    files.append((p, rel))
private_hashes = {hashlib.sha256(p.read_bytes()).hexdigest() for p in (root / 'recovery').rglob('*') if p.is_file()}
for p, rel in files:
    if hashlib.sha256(p.read_bytes()).hexdigest() in private_hashes:
        raise SystemExit('Refusing private recovery content under another name: ' + str(rel))
with zipfile.ZipFile(output, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
    for p, rel in sorted(files, key=lambda row: str(row[1])):
        archive.write(p, 'stardust-blackjack-rust-' + version + '/' + str(rel))
with zipfile.ZipFile(output) as archive:
    assert archive.testzip() is None
    for entry in archive.infolist():
        assert not any(x in blocked or x.startswith('target') or x.startswith('device-') for x in Path(entry.filename).parts)
        assert hashlib.sha256(archive.read(entry)).hexdigest() not in private_hashes
manifest = {'version': version, 'archive': output.name, 'bytes': output.stat().st_size, 'files': len(files), 'sha256': hashlib.sha256(output.read_bytes()).hexdigest(), 'private_recovery_excluded': True, 'device_evidence_excluded': True}
output.with_suffix('.json').write_text(json.dumps(manifest, indent=2) + '\n')
print(json.dumps(manifest, indent=2))
