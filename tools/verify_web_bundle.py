#!/usr/bin/env python3
"""Bind the shipped browser module to its current source and runtime files."""
import argparse,hashlib,json,tomllib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def inputs():
 files={ROOT/'Cargo.toml',ROOT/'Cargo.lock'}
 for folder in ['src','assets','resume-journal','browser/bridge/src','browser/journal/src']:
  files.update(p for p in (ROOT/folder).rglob('*') if p.is_file() and (p.suffix in {'.rs','.toml','.bin','.json'}))
 for folder in ['browser/bridge','browser/journal']:
  for name in ['Cargo.toml','Cargo.lock']:
   p=ROOT/folder/name
   if p.is_file():files.add(p)
 files.update(ROOT/'browser'/p for p in ['index.html','app.js','style.css','journal-host.js','journal-codec.js','build.py'])
 assert all(p.is_file() and not p.is_symlink() for p in files)
 return {p.relative_to(ROOT).as_posix():sha(p) for p in sorted(files)}
def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--write',action='store_true');args=parser.parse_args();wasm=ROOT/'browser/stardust-rust.wasm';manifest=ROOT/'browser/bundle-manifest.json'
 expected={'schema':1,'version':tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version'],'wasm':{'file':'stardust-rust.wasm','bytes':wasm.stat().st_size,'sha256':sha(wasm)},'inputs':inputs()}
 if args.write:manifest.write_text(json.dumps(expected,indent=2)+'\n')
 else:
  actual=json.loads(manifest.read_text());assert actual==expected,'Browser bundle is stale: rebuild browser/build.py, verify it, then regenerate the source binding.'
 print('PASS browser bundle matches version, current sources, assets and web host files.')
if __name__=='__main__':main()
