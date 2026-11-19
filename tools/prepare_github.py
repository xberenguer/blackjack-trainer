#!/usr/bin/env python3
"""Prepare a private-data-free LOCAL REVIEW copy; never creates a remote or uploads."""
from pathlib import Path
import argparse
import hashlib
import json
import re
import shutil
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
TOP_DIRS = {'src', 'assets', 'browser', 'docs', 'examples', 'firmware', 'resume-journal', 'tests', 'tools', 'validation'}
BLOCKED_DIRS = {'.git', '.vendor', '.cargo', '.build', '.tools', '.venv', 'venv', '.pytest_cache', 'node_modules', '__pycache__', 'cargo-home', 'recovery', 'releases', 'build'}
CURATED_VALIDATION = {'release-0.10.0.json', 'release-0.10.2.json', 'release-0.11.0.json', 'release-0.11.1.json', 'release-0.11.2.json', 'rust-core-validation.json'}
BLOCKED_SUFFIXES = {'.pyc', '.tmp', '.log', '.elf', '.zip', '.rgb565', '.pem', '.key'}

def eligible(path):
    rel = path.relative_to(ROOT)
    if path.is_symlink() or not path.is_file():
        return False
    if any(part in BLOCKED_DIRS or part.startswith('target') or part.startswith('device-') for part in rel.parts[:-1]):
        return False
    if path.name == '.DS_Store' or path.suffix in BLOCKED_SUFFIXES or (path.name != '.env.example' and (path.name == '.env' or path.name.startswith('.env.'))):
        return False
    if len(rel.parts) == 1:
        return path.suffix in {'.md', '.toml', '.lock'} or path.name in {'.gitignore', 'LICENSE', 'NOTICE', 'NOTICE.md'}
    if rel.parts[0] not in TOP_DIRS:
        return False
    if 'validation' in rel.parts[:-1]:
        return rel.parts[0] == 'validation' and len(rel.parts) == 2 and path.name in CURATED_VALIDATION
    if path.suffix == '.bin':
        return rel.parts[0] == 'assets' or rel.parts[:2] == ('tests', 'data') or rel.as_posix() == 'firmware/tools/nvs-check/fixtures/save-v7-synthetic.bin'
    if path.suffix == '.wasm':
        return rel.as_posix() == 'browser/stardust-rust.wasm'
    return True

def collect():
    private_hashes = {hashlib.sha256(p.read_bytes()).hexdigest() for p in (ROOT / 'recovery').rglob('*') if p.is_file() and not p.is_symlink()}
    files = []
    for p in sorted(ROOT.rglob('*')):
        if not eligible(p):
            continue
        raw = p.read_bytes()
        sha = hashlib.sha256(raw).hexdigest()
        if sha in private_hashes:
            raise SystemExit('Refusing renamed recovery content: ' + str(p.relative_to(ROOT)))
        if p.suffix in {'.rs', '.py', '.cjs', '.js', '.md', '.toml', '.json', '.yaml', '.yml', '.html'} and re.search(rb'/(?:Users|home)/[A-Za-z0-9._-]+/', raw):
            raise SystemExit('Author-specific absolute path must be removed before export: ' + str(p.relative_to(ROOT)))
        files.append((p, {'path': p.relative_to(ROOT).as_posix(), 'bytes': len(raw), 'sha256': sha}))
    return files

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--dest', type=Path, default=ROOT.parent / 'blackjack-trainer')
    parser.add_argument('--dry-run', action='store_true')
    args = parser.parse_args()
    dest = args.dest.expanduser().resolve()
    if dest == ROOT or ROOT in dest.parents:
        raise SystemExit('Choose a new destination outside the source project.')
    if dest.exists() and not args.dry_run:
        raise SystemExit('Destination exists; choose a new review directory. Existing files are never replaced.')
    files = collect()
    version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
    manifest = {
        'repository_name': 'blackjack-trainer', 'version': version,
        'kind': 'LOCAL REVIEW CANDIDATE', 'ready_for_public_upload': False,
        'asset_rights_cleared': False,
        'review_required': ['Inherited graphics, sound and font-data rights/notices: docs/PUBLISHING.md and docs/THIRD_PARTY.md', 'A separate first-install firmware bundle requires qualification: docs/INSTALL.md'],
        'private_recovery_excluded': True, 'device_sessions_excluded': True,
        'build_caches_and_host_logs_excluded': True,
        'includes_review_assets_and_browser': True,
        'files': [entry for _, entry in files],
    }
    if args.dry_run:
        print(json.dumps({k:v for k,v in manifest.items() if k != 'files'}, indent=2))
        print('Review files:', len(files))
        return
    dest.parent.mkdir(parents=True, exist_ok=True)
    temporary = Path(tempfile.mkdtemp(prefix=dest.name + '-stage-', dir=dest.parent))
    try:
        for source, entry in files:
            target = temporary / entry['path']
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)
            if hashlib.sha256(target.read_bytes()).hexdigest() != entry['sha256']:
                raise RuntimeError('Copy hash mismatch: ' + entry['path'])
        (temporary / 'publication-manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
        temporary.rename(dest)
    finally:
        if temporary.exists():
            shutil.rmtree(temporary)
    print('Prepared local review candidate:', dest)
    print('Files:', len(files))
    print('Nothing was uploaded. Review the recorded asset and first-install status before public release.')

if __name__ == '__main__':
    main()
