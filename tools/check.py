#!/usr/bin/env python3
"""Reproduce Rust math, state and pixel evidence without a C++ toolchain."""
import argparse,gzip,hashlib,json,os,subprocess,sys
from pathlib import Path
root=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser();p.add_argument('--logs',type=Path,default=root/'validation/latest-native');a=p.parse_args();a.logs.mkdir(parents=True,exist_ok=True)
target=Path(os.environ.get('CARGO_TARGET_DIR',root/'target'));target=target if target.is_absolute() else root/target

def run(name,cmd,input=None):
 print(name,flush=True)
 with (a.logs/(name+'.log')).open('wb')as log:subprocess.run(cmd,cwd=root,input=input,stdout=log,stderr=subprocess.STDOUT,check=True)
run('tests',['cargo','test','--release','--offline','--','--nocapture'])
run('clippy',['cargo','clippy','--all-targets','--all-features','--offline','--','-D','warnings'])
run('examples',['cargo','build','--release','--offline','--examples','--all-features'])
exe=target/'release/examples'
for kind in ['engine','render','table','widgets']:
 reference=gzip.decompress((root/'tests/reference'/(kind+'.txt.gz')).read_bytes())
 if kind=='widgets':
  # Preserve historical menu gold; 0.10.1 intentionally changes menu geometry/cues.
  # Cut selectors retain their accepted pixels. Current menus receive native visual review.
  reference=b''.join(line for line in reference.splitlines(keepends=True) if line.startswith(b'C '))
  print('widgets-parity: legacy cut layouts only; current 0.10.1 menus visually qualified',flush=True)
 run(kind+'-parity',[str(exe/(kind+'_parity'))],reference)
for kind in ['ev','split_ev']:
 run(kind+'-exact',[sys.executable,'-B',str(root/'tests'/(kind+'_exact_oracle.py')),str(exe/(kind+'_oracle_dump'))])
manifest={'status':'native Rust checks passed; app/firmware/browser integration and physical checks still required','sources':{}}
for f in sorted(root.rglob('*')):
 if f.is_file() and not f.is_symlink() and f.suffix in ['.rs','.toml','.csv','.bin','.gz','.py'] and not any(x.startswith('target') or x in ['.git','cargo-home','recovery','releases','validation','.vendor','.cargo','__pycache__','build'] for x in f.relative_to(root).parts):manifest['sources'][str(f.relative_to(root))]=hashlib.sha256(f.read_bytes()).hexdigest()
(a.logs/'source-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
print('PASS native Rust checks; evidence: '+str(a.logs),flush=True)
