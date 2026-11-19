#!/usr/bin/env python3
"""Build the zero-dependency Rust browser; no Emscripten or C++ tools."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess

HERE = Path(__file__).resolve().parent
PROJECT = HERE.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--print-paths', action='store_true', help='print resolved build paths without writing or building')
args = parser.parse_args()
env = os.environ.copy()
build_dir = Path(env.get('STARDUST_BUILD_DIR', PROJECT / '.build')).expanduser().resolve()
for key, default in [('CARGO_HOME', build_dir / 'cargo'), ('CARGO_TARGET_DIR', build_dir / 'browser-target')]:
    env[key] = str(Path(env.get(key, default)).expanduser().resolve())
toolchain = env.get('STARDUST_TOOLCHAIN')
if not toolchain:
    installed = subprocess.check_output(['rustup', 'toolchain', 'list'], text=True)
    names = {line.split()[0] for line in installed.splitlines() if line.strip()}
    toolchain = next((name for name in ('roulette-esp', 'esp') if name in names), None)
if not toolchain:
    raise SystemExit('Set STARDUST_TOOLCHAIN to a nightly/build-std-capable toolchain; see docs/BUILD.md.')
sysroot = Path(subprocess.check_output(['rustup', 'run', toolchain, 'rustc', '--print', 'sysroot'], text=True).strip())
vendor = sysroot / 'lib/rustlib/src/rust/library/vendor'
if not vendor.is_dir():
    raise SystemExit(f'Missing vendored rust-src at {vendor}; see docs/BUILD.md for the tested requirements.')
if args.print_paths:
    print(json.dumps({'project': str(PROJECT), 'build_dir': str(build_dir), 'cargo_home': env['CARGO_HOME'], 'target_dir': env['CARGO_TARGET_DIR'], 'toolchain': toolchain, 'sysroot_vendor': str(vendor), 'output': str(HERE / 'stardust-rust.wasm')}, indent=2))
    raise SystemExit(0)
# Fixed memory/stack limits are ABI requirements, rather than caller tuning.
env['RUSTFLAGS'] = '-C link-arg=-zstack-size=2097152 -C link-arg=--initial-memory=16777216 -C link-arg=--max-memory=16777216'
subprocess.run(['cargo', '+' + toolchain, 'build', '-Zbuild-std=core',
    '--target', 'wasm32v1-none', '--release', '--offline',
    '--config', 'source.crates-io.replace-with="rust-sysroot-vendor"',
    '--config', 'source.rust-sysroot-vendor.directory=' + json.dumps(str(vendor))],
    cwd=HERE / 'bridge', env=env, check=True)
artifact = Path(env['CARGO_TARGET_DIR']) / 'wasm32v1-none/release/stardust_browser.wasm'
shutil.copyfile(artifact, HERE / 'stardust-rust.wasm')
print(f'Built {HERE / "stardust-rust.wasm"} ({artifact.stat().st_size:,} bytes)')
