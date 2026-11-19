#!/usr/bin/env python3
"""Build/package Rust firmware. Never opens serial, flashes, or erases any device."""
from pathlib import Path
import argparse, hashlib, json, os, re, shutil, subprocess, struct, sys, tarfile, tomllib, urllib.request

fw = Path(__file__).resolve().parents[1]
project = fw.parent
builder_input_hash = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()

def source_inputs():
    """Deterministic project-relative fingerprints; omit generated/cache state."""
    required = [
        project / 'Cargo.toml', project / 'Cargo.lock',
        fw / 'Cargo.toml', fw / 'Cargo.lock', fw / 'build.rs', fw / 'partitions.csv',
        fw / 'hal-patch/Cargo.toml', fw / 'hal-patch/Cargo.lock',
        fw / 'hal-patch/build.rs', fw / 'hal-patch/esp_config.yml',
        project / 'resume-journal/Cargo.toml', project / 'resume-journal/Cargo.lock',
    ]
    trees = [
        project / 'src', project / 'assets', fw / 'src', fw / 'tools',
        fw / 'hal-patch/src', fw / 'hal-patch/ld', project / 'resume-journal/src',
    ]
    # Support project-local linker files as well as the patched HAL's scripts.
    if (fw / 'ld').is_dir():
        trees.append(fw / 'ld')
    paths = set(required)
    for tree in trees:
        if not tree.is_dir() or tree.is_symlink():
            raise SystemExit(f'Missing or linked source directory: {tree.relative_to(project)}')
        for path in tree.rglob('*'):
            relative = path.relative_to(project)
            if any(part in {'__pycache__', '.git', '.build', 'build', '.vendor', '.cargo'}
                   or part.startswith('target') for part in relative.parts):
                continue
            if path.is_symlink():
                raise SystemExit(f'Source input must not be a symlink: {relative.as_posix()}')
            if path.is_file() and path.suffix not in {'.pyc', '.pyo', '.tmp', '.log'}:
                paths.add(path)
    result = {}
    for path in sorted(paths, key=lambda item: item.relative_to(project).as_posix()):
        if not path.is_file() or path.is_symlink():
            raise SystemExit(f'Missing or linked source input: {path.relative_to(project).as_posix()}')
        result[path.relative_to(project).as_posix()] = hashlib.sha256(path.read_bytes()).hexdigest()
    return result

def assert_source_inputs_unchanged(expected):
    actual = source_inputs()
    changed = sorted(name for name in expected.keys() | actual.keys()
                     if expected.get(name) != actual.get(name))
    if changed:
        raise SystemExit('Source inputs changed during firmware build: ' + ', '.join(changed))

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--download', action='store_true', help='fetch missing exact Cargo.lock crates after SHA256 verification')
p.add_argument('--print-paths', action='store_true', help='print selected build/tool paths without writing or building')
a = p.parse_args()
env = dict(os.environ)
build_root = Path(env.get('STARDUST_BUILD_DIR', project / '.build')).expanduser().resolve()
for key, default in [('CARGO_HOME', build_root / 'cargo'), ('CARGO_TARGET_DIR', build_root / 'firmware-target')]:
    env[key] = str(Path(env.get(key, default)).expanduser().resolve())
toolchain = env.get('STARDUST_TOOLCHAIN')
if not toolchain:
    installed = subprocess.check_output(['rustup', 'toolchain', 'list'], text=True)
    names = {line.split()[0] for line in installed.splitlines() if line.strip()}
    toolchain = next((name for name in ('roulette-esp', 'esp') if name in names), None)
if not toolchain:
    raise SystemExit('An Xtensa-capable Rust toolchain is required; set STARDUST_TOOLCHAIN. See docs/BUILD.md.')
sysroot = Path(subprocess.check_output(['rustup', 'run', toolchain, 'rustc', '--print', 'sysroot'], text=True).strip())
sysroot_vendor = sysroot / 'lib/rustlib/src/rust/library/vendor'
if not sysroot_vendor.is_dir():
    raise SystemExit(f'Missing vendored rust-src at {sysroot_vendor}; see docs/BUILD.md for the tested toolchain requirements.')
vendor = build_root / 'firmware-vendor'
gcc = env.get('STARDUST_XTENSA_GCC') or shutil.which('xtensa-esp32s3-elf-gcc') or str(Path.home() / '.platformio/packages/toolchain-xtensa-esp32s3/bin/xtensa-esp32s3-elf-gcc')
gcc = str(Path(gcc).expanduser().resolve())
if not Path(gcc).is_file():
    raise SystemExit('Set STARDUST_XTENSA_GCC to the installed Xtensa ESP32S3 GCC')
# Keep a virtual environment's interpreter path: resolving its executable
# symlink selects the base interpreter and loses that environment's packages.
python = str(Path(env.get('STARDUST_ESPTOOL_PYTHON', sys.executable)).expanduser().absolute())
parttool = Path(env.get('STARDUST_PARTITION_TOOL') or shutil.which('gen_esp32part.py') or Path.home() / '.platformio/packages/framework-arduinoespressif32/tools/gen_esp32part.py').expanduser().resolve()
if not parttool.is_file():
    raise SystemExit('Set STARDUST_PARTITION_TOOL to Espressif gen_esp32part.py')
if a.print_paths:
    print(json.dumps({'project': str(project), 'build_dir': str(build_root), 'cargo_home': env['CARGO_HOME'], 'target_dir': env['CARGO_TARGET_DIR'], 'toolchain': toolchain, 'sysroot_vendor': str(sysroot_vendor), 'vendor': str(vendor), 'gcc': gcc, 'esptool_python': python, 'partition_tool': str(parttool), 'output': str(fw / 'build')}, indent=2))
    raise SystemExit(0)
if subprocess.run([python, '-c', 'import esptool'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode:
    raise SystemExit('Set STARDUST_ESPTOOL_PYTHON to a Python with esptool installed; see docs/BUILD.md.')
core_manifest_bytes = (project / 'Cargo.toml').read_bytes()
core_version = tomllib.loads(core_manifest_bytes.decode())['package']['version']
firmware_manifest = fw / 'Cargo.toml'
manifest_text = firmware_manifest.read_text()
manifest_text = re.sub(r'(?m)^version\s*=\s*"[^"]+"', lambda m: 'version = ' + json.dumps(core_version), manifest_text, count=1)
if firmware_manifest.read_text() != manifest_text:
    firmware_manifest.write_text(manifest_text)
inputs_before = source_inputs()
if inputs_before['firmware/tools/build.py'] != builder_input_hash:
    raise SystemExit('Firmware builder changed while preparing the build')
if inputs_before['Cargo.toml'] != hashlib.sha256(core_manifest_bytes).hexdigest():
    raise SystemExit('Core package manifest changed while preparing the build')
registry = Path(env['CARGO_HOME']) / 'registry'
sources = registry / 'src/index.crates.io-1949cf8c6b5b557f'
cache = registry / 'cache/index.crates.io-1949cf8c6b5b557f'
vendor.mkdir(parents=True, exist_ok=True)
for pkg in tomllib.loads((fw / 'Cargo.lock').read_text())['package']:
    if 'source' not in pkg:
        continue
    name = pkg['name'] + '-' + pkg['version']; dest = vendor / name; sha = pkg.get('checksum')
    if not dest.exists():
        source = sources / name
        if source.exists():
            dest.mkdir()
            for file in source.iterdir():
                if file.name != '.cargo-checksum.json':
                    (dest / file.name).symlink_to(file)
        else:
            crate = cache / (name + '.crate')
            if not crate.exists():
                crate = vendor / (name + '.crate')
            if not crate.exists():
                if not a.download:
                    raise SystemExit(f'Missing {name}; rerun with --download to fetch its pinned archive, or explicitly set CARGO_HOME to a populated cache')
                url = f'https://static.crates.io/crates/{pkg["name"]}/{name}.crate'
                crate.write_bytes(urllib.request.urlopen(url, timeout=60).read())
            if hashlib.sha256(crate.read_bytes()).hexdigest() != sha:
                raise SystemExit(f'Checksum failure: {crate}')
            with tarfile.open(crate) as archive:
                archive.extractall(vendor, filter='data')
    (dest / '.cargo-checksum.json').write_text(json.dumps({'files': {}, 'package': sha}))
for source in sysroot_vendor.iterdir():
    if not source.is_dir():
        continue
    dest = vendor / source.name
    if not dest.exists():
        dest.mkdir()
    for file in source.iterdir():
        if file.name != '.cargo-checksum.json' and not (dest / file.name).exists():
            (dest / file.name).symlink_to(file)
    if not (dest / '.cargo-checksum.json').exists():
        (dest / '.cargo-checksum.json').write_text((source / '.cargo-checksum.json').read_text())
config = f'''[build]\ntarget = "xtensa-esp32s3-none-elf"\n[target.xtensa-esp32s3-none-elf]\nlinker = {json.dumps(gcc)}\nrustflags = ["-C", "link-arg=-nostartfiles", "-C", "link-arg=-Tlinkall.x"]\n[unstable]\nbuild-std = ["core"]\n[source.crates-io]\nreplace-with = "local-vendor"\n[source.local-vendor]\ndirectory = {json.dumps(str(vendor))}\n[env]\nESP_HAL_CONFIG_ENSURE_MAIN_STACK_MINIMUM = "262144"\n'''
(fw / '.cargo').mkdir(exist_ok=True)
(fw / '.cargo/config.toml').write_text(config)
subprocess.run(['cargo', '+' + toolchain, 'build', '--release', '--offline'], cwd=fw, env=env, check=True)
elf = Path(env['CARGO_TARGET_DIR']) / 'xtensa-esp32s3-none-elf/release/stardust-sticks3'
# Verify physical SRAM aliases in the actual ELF, independent of linker intent.
elfbytes=elf.read_bytes();assert elfbytes[:5]==b'\x7fELF\x01' and elfbytes[5]==1
shoff=struct.unpack_from('<I',elfbytes,32)[0];shsize,shnum,shstr=struct.unpack_from('<HHH',elfbytes,46)
sections=[struct.unpack_from('<10I',elfbytes,shoff+i*shsize) for i in range(shnum)]
strings=sections[shstr];names=elfbytes[strings[4]:strings[4]+strings[5]]
named=[]
for sec in sections:
    name=names[sec[0]:].split(b'\0',1)[0].decode();named.append((name,sec[3],sec[5],sec[1],sec[2]))
iram=[(name,addr-0x6f0000,addr+size-0x6f0000) for name,addr,size,ty,flags in named if size and flags&2 and 0x40378000<=addr<0x403e0000]
dram=[(name,addr,addr+size) for name,addr,size,ty,flags in named if size and flags&2 and name!='.rwdata_dummy' and 0x3fc88000<=addr<0x3fcf0000]
alias_proof={'iram_physical_aliases':[{'section':n,'start':hex(a),'end':hex(z)} for n,a,z in iram],'dram_loaded_sections':[{'section':n,'start':hex(a),'end':hex(z)}for n,a,z in dram],'overlap_bytes':0,'previous_failure':{'iram_alias':['0x3fc88000','0x3fc8aa60'],'data_start':'0x3fc8a660','overlap_bytes':1024}}
for ni,ai,zi in iram:
    for nd,ad,zd in dram:
        overlap=max(0,min(zi,zd)-max(ai,ad));alias_proof['overlap_bytes']+=overlap
assert alias_proof['overlap_bytes']==0,f'IRAM/DRAM physical alias overlap: {alias_proof}'
out=fw/'build';out.mkdir(exist_ok=True);(out/'memory-proof.json').write_text(json.dumps(alias_proof,indent=2)+'\n');shutil.copyfile(elf,out/'stardust-sticks3.elf')
subprocess.run([str(python),'-m','esptool','--chip','esp32s3','elf2image','--flash_mode','qio','--flash_freq','80m','--flash_size','8MB','--elf-sha256-offset','0xb0','-o',str(out/'stardust-sticks3.bin'),str(elf)],check=True)
subprocess.run([str(python),str(parttool),str(fw/'partitions.csv'),str(out/'partitions.bin')],check=True)
assert (out/'stardust-sticks3.bin').stat().st_size<=0x600000
nm=Path(gcc).with_name('xtensa-esp32s3-elf-nm')
symbols=subprocess.check_output([str(nm),'-n',str(elf)],text=True)
stack={line.split()[2]:int(line.split()[0],16) for line in symbols.splitlines() if len(line.split())==3 and line.split()[2]in['_stack_end','_stack_start']}
stack_bytes=stack['_stack_start']-stack['_stack_end'];assert stack_bytes>=262144
objdump=Path(gcc).with_name('xtensa-esp32s3-elf-objdump')
disasm=subprocess.check_output([str(objdump),'-d','--disassemble=main',str(elf)],text=True)
prologue=disasm.split('<main>:',1)[1].splitlines()[:10]
entry=re.search(r'entry\s+a1,\s+(0x[0-9a-f]+|[0-9]+)', '\n'.join(prologue))
assert entry and not any('movsp' in line for line in prologue),'unexpected large/dynamic main prologue; inspect initialization stack copies'
main_frame_bytes=int(entry.group(1),0);assert main_frame_bytes<32768, 'main frame exceeds32KB; keep App initialization in PSRAM'
(out/'stack-proof.txt').write_text('ELF SHA256 '+hashlib.sha256(elf.read_bytes()).hexdigest()+'\n'+f'CPU0 stack {stack_bytes} bytes; main frame {main_frame_bytes} bytes\n'+ '\n'.join(disasm.splitlines()[:28])+'\n')
from xtensa_frames import audit_elf
frame_proof=audit_elf(elf,objdump=objdump,allow_existing=True)
(out/'linked-stack-proof.json').write_text(json.dumps(frame_proof,indent=2)+'\n')
assert frame_proof['passed'], 'linked frame audit failed; inspect linked-stack-proof.json (entry + literal/movsp, not entry alone)'
decoder_frames=[r['frame_bytes']for r in frame_proof['functions']if r['hint']=='App::resume_read']
assert decoder_frames, 'missing linked App::resume_read decoder proof'
assert_source_inputs_unchanged(inputs_before)
manifest={'linked_stack_audit':{'functions_audited':frame_proof['functions_audited'],'maximum_frame_bytes':max(r['frame_bytes']for r in frame_proof['functions']),'app_decode_frame_bytes':max(decoder_frames),'ordinary_function_cap':32768,'decoder_cap':8192,'explicit_existing_path_cap':65536,'unknown_sp_mutations':sum(len(r['unresolved_sp'])for r in frame_proof['functions'])},'version':core_version,'language':'Rust','target':'xtensa-esp32s3-none-elf','stack_bytes':stack_bytes,'main_frame_bytes':main_frame_bytes,'iram_dram_overlap_bytes':alias_proof['overlap_bytes'],'app_offset':'0x10000','partition_table_offset':'0x8000','preserve':['bootloader','nvs','otadata','reserved predecessor filesystem except declared rustsave and rustresume partitions'], 'private_resume':{'partition_offset':'0x770000','partition_bytes':524288,'slot_bytes':262144,'max_payload_bytes':262000,'default_enabled':False,'erase_policy':'fixed footer sector first, then actual payload sectors only'}, 'artifacts':{f.name:{'bytes':f.stat().st_size,'sha256':hashlib.sha256(f.read_bytes()).hexdigest()} for f in[out/'stardust-sticks3.elf',out/'stardust-sticks3.bin',out/'partitions.bin']}}
manifest['source_inputs'] = inputs_before
(out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n');print(json.dumps(manifest,indent=2))
