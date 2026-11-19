#!/usr/bin/env python3
"""Build and qualify exactly one frozen standalone browser/core source set."""
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import hashlib, json, os, tomllib
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from threading import Thread
from pathlib import Path
import shutil, subprocess, sys

BROWSER=Path(__file__).resolve().parents[1]
CORE=BROWSER.parent
OUT=BROWSER/'validation';OUT.mkdir(exist_ok=True)
NODE=os.environ.get('STARDUST_NODE') or shutil.which('node')
if not NODE:raise SystemExit('Node.js is required; install it on PATH or set STARDUST_NODE.')
ENV=os.environ.copy()
BUILD=Path(ENV.get('STARDUST_BUILD_DIR',CORE/'.build')).expanduser().resolve()
for key,default in [('CARGO_HOME',BUILD/'cargo'),('CARGO_TARGET_DIR',BUILD/'browser-target')]:
    ENV[key]=str(Path(ENV.get(key,default)).expanduser().resolve())
toolchain=ENV.get('STARDUST_TOOLCHAIN')
if not toolchain:
    names={line.split()[0] for line in subprocess.check_output(['rustup','toolchain','list'],text=True).splitlines() if line.strip()}
    toolchain=next((name for name in ('roulette-esp','esp') if name in names),None)
if not toolchain:raise SystemExit('Set STARDUST_TOOLCHAIN; see docs/BUILD.md.')
ENV['STARDUST_TOOLCHAIN']=toolchain
sysroot=Path(subprocess.check_output(['rustup','run',toolchain,'rustc','--print','sysroot'],text=True).strip())
vendor=sysroot/'lib/rustlib/src/rust/library/vendor'
if not vendor.is_dir():raise SystemExit('The selected toolchain needs vendored rust-src; see docs/BUILD.md.')
CARGO_SOURCE=['--config','source.crates-io.replace-with="rust-sysroot-vendor"','--config','source.rust-sysroot-vendor.directory='+json.dumps(str(vendor))]

def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def guard():
    paths=sorted([CORE/'Cargo.toml',CORE/'Cargo.lock',*CORE.glob('src/**/*.rs'),*(p for p in (CORE/'assets').rglob('*') if p.is_file())])
    digest=hashlib.sha256()
    for p in paths:digest.update(p.relative_to(CORE).as_posix().encode()+b'\0'+p.read_bytes()+b'\0')
    return digest.hexdigest(),{p.relative_to(CORE).as_posix():sha(p) for p in paths}
def run(item):
    name,cmd=item
    result=subprocess.run(cmd,cwd=BROWSER,env=ENV,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
    (OUT/f'{name}.log').write_text(result.stdout)
    return name,{'passed':result.returncode==0,'exit_code':result.returncode,'log':f'{name}.log','summary':result.stdout.strip().splitlines()[-1:]}

before,files=guard()
name,build=run(('build',[sys.executable,str(BROWSER/'build.py')]))
if not build['passed']:print((OUT/'build.log').read_text());sys.exit(1)
checks=[
 ('wasm',[NODE,str(BROWSER/'tests/wasm.cjs')]),
 ('platform',[NODE,str(BROWSER/'tests/platform.cjs')]),
 ('audio',[NODE,str(BROWSER/'tests/audio.cjs')]),
 ('chrome',[NODE,str(BROWSER/'tests/chrome.cjs')]),
 ('journal',['cargo','test','--manifest-path',str(BROWSER/'journal/Cargo.toml'),'--offline']),
 ('journal-clippy',['cargo','clippy','--manifest-path',str(BROWSER/'journal/Cargo.toml'),'--all-targets','--offline','--','-D','warnings']),
 ('bridge-clippy',['cargo','+'+toolchain,'clippy','--manifest-path',str(BROWSER/'bridge/Cargo.toml'),'-Zbuild-std=core','--target','wasm32v1-none','--release','--offline',*CARGO_SOURCE,'--','-D','warnings']),
]
# Actual DOM tests get a private ephemeral localhost origin; no external server required.
class QuietHandler(SimpleHTTPRequestHandler):
    def log_message(self,*args):pass
server=ThreadingHTTPServer(('127.0.0.1',0),partial(QuietHandler,directory=str(BROWSER)))
ENV['STARDUST_BROWSER_URL']=f'http://127.0.0.1:{server.server_port}'
thread=Thread(target=server.serve_forever,daemon=True);thread.start()
try:
    with ThreadPoolExecutor(max_workers=5) as pool:results=dict(pool.map(run,checks))
finally:
    server.shutdown();server.server_close();thread.join()
after,_=guard()
wasm=BROWSER/'stardust-rust.wasm'
source={p.relative_to(BROWSER).as_posix():sha(p) for p in sorted(BROWSER.rglob('*')) if p.is_file() and not any(part in {'validation','.build','target','node_modules','__pycache__'} for part in p.relative_to(BROWSER).parts) and p!=wasm}
manifest={'version':tomllib.loads((CORE/'Cargo.toml').read_text())['package']['version'],'utc':datetime.now(timezone.utc).isoformat(),'core_guard_sha256':after,'core_sources_unchanged':before==after,'core_files':files,'browser_files':source,'wasm':{'file':wasm.name,'bytes':wasm.stat().st_size,'sha256':sha(wasm),'memory_bytes':16777216,'stack_bytes':2097152,'runtime_imports':0},'checks':{'build':build,**results},'scope':'Software acceptance only; no device flash performed by browser agent.','integration':{'copy':'Whole browser/ tree, including journal/, tests/ and validation/.','core_dependency':'Set bridge/Cargo.toml stardust-blackjack path to ../.. in ordinary core/browser layout.','retained':'Dynamic version title/banner derives sb_version from core VERSION.','excluded':'No production root docs or firmware were modified.'}}
manifest['passed']=before==after and all(x['passed'] for x in manifest['checks'].values())
(OUT/'browser-acceptance.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(json.dumps({'passed':manifest['passed'],'core_guard_sha256':after,'wasm':manifest['wasm'],'checks':{k:v['passed'] for k,v in manifest['checks'].items()}},indent=2))
if not manifest['passed']:
    for k,v in manifest['checks'].items():
        if not v['passed']:print(k,(OUT/v['log']).read_text())
    sys.exit(1)
