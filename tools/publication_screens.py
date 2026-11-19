#!/usr/bin/env python3
"""Render synthetic native screens and generate review-only GitHub media.

No device, network, save, recovery payload, or production-score input is read.
Requires Pillow; all PNG previews are exact3x nearest-neighbor pixel replication.
"""
from pathlib import Path
import argparse,csv,hashlib,json,os,subprocess,re,tomllib,unicodedata
from publication_catalog import GROUPS,RECOVERY
from PIL import Image
ROOT=Path(__file__).resolve().parents[1]
VERSION=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version']
WORK=next((p/'work' for p in ROOT.parents if (p/'work').is_dir()),ROOT/'validation')

def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def runtime_guard():return {p.relative_to(ROOT).as_posix():digest(p)for base in ['src','assets']for p in sorted((ROOT/base).rglob('*'))if p.is_file()}
def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--raw-dir',type=Path,default=WORK/'rust-publication/frames');parser.add_argument('--existing',action='store_true',help='convert a previously rendered frames.tsv');a=parser.parse_args()
 before=runtime_guard()
 if not a.existing:
  env=dict(os.environ);env.setdefault('CARGO_TARGET_DIR',str(WORK/'rust-publication-target'))
  subprocess.run(['cargo','run','--offline','--locked','--release','--features','validation','--example','publication_screens','--',str(a.raw_dir.resolve())],cwd=ROOT,env=env,check=True)
 assert before==runtime_guard(),'runtime/source assets changed while rendering'
 rows=list(csv.DictReader((a.raw_dir/'frames.tsv').open(),delimiter='\t'))
 media=ROOT/'docs/media';native=media/'screens/native';preview=media/'screens';native.mkdir(parents=True,exist_ok=True)
 records=[]
 for row in rows:
  slug=row['slug'];assert re.fullmatch('[a-z0-9-]+',slug);im=Image.open(a.raw_dir/f'{slug}.ppm').convert('RGB');assert im.size==(240,135)
  im.save(native/f'{slug}.png',optimize=True)
  large=im.resize((720,405),Image.Resampling.NEAREST);large.save(preview/f'{slug}.png',optimize=True)
  raw=im.tobytes();expected=b''.join(b''.join(raw[i:i+3]*3 for i in range(y*720,(y+1)*720,3))*3 for y in range(135))
  assert Image.open(preview/f'{slug}.png').convert('RGB').tobytes()==expected
  assert Image.open(native/f'{slug}.png').convert('RGB').tobytes()==raw
  records.append({**row,'native':f'screens/native/{slug}.png','preview':f'screens/{slug}.png','rgb_sha256':hashlib.sha256(im.tobytes()).hexdigest(),'png_sha256':digest(native/f'{slug}.png'),'preview_sha256':digest(preview/f'{slug}.png')})
 for hero,slug in [('table','table-player'),('guide','table-guide')]:
  Image.open(preview/f'{slug}.png').save(media/f'{hero}.png',optimize=True)
 animations=[]
 for name,prefix,duration in [('deal','anim-deal-',130),('tablemates','anim-tablemates-',400)]:
  shots=[Image.open(preview/f"{r['slug']}.png").convert('RGB')for r in records if r['slug'].startswith(prefix)]
  # GIF's256-color limitation is explicit; PNG originals retain exact colors.
  strip=Image.new('RGB',(720,405*len(shots)))
  for i,im in enumerate(shots):strip.paste(im,(0,i*405))
  palette=strip.quantize(colors=256,method=Image.Quantize.MEDIANCUT,dither=Image.Dither.NONE)
  frames=[im.quantize(palette=palette,dither=Image.Dither.NONE)for im in shots]
  frames[0].save(media/f'{name}.gif',save_all=True,append_images=frames[1:],duration=duration,loop=0,disposal=2,optimize=False)
  encoded=Image.open(media/f'{name}.gif');actual_hashes={hashlib.sha256(f.convert('RGB').tobytes()).hexdigest()for f in frames}
  elapsed=0
  for frame_index in range(encoded.n_frames):
   encoded.seek(frame_index);elapsed+=encoded.info['duration'];assert hashlib.sha256(encoded.convert('RGB').tobytes()).hexdigest()in actual_hashes
  assert elapsed==len(frames)*duration
  animations.append({'file':f'{name}.gif','sha256':digest(media/f'{name}.gif'),'source_frames':len(frames),'encoded_frames':encoded.n_frames,'duration_ms_per_source_frame':duration,'source_prefix':prefix,'colors':'GIF global256-color quantization; exact PNG originals retained'})
 manifest={'version':VERSION,'status':'LOCAL REVIEW CANDIDATE; asset-rights review pending, not a publication/license grant','provenance':'Actual native Rust renderer, deterministic synthetic demo only; no device/save/private-recovery/saved scores input','native_size':[240,135],'preview_size':[720,405],'scaling':'nearest-neighbor3x','runtime_source_unchanged':True,'runtime_guard':before,'frames':records,'animations':animations}
 (media/'screen-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
 build_guide(records,manifest)
 audit_links()
 print(f'Generated{len(records)} native/preview pairs and{len(animations)} actual-frame GIFs;65/65 Views covered; runtime/assets unchanged')
def build_guide(records,manifest):
 app=(ROOT/'src/app.rs').read_text()
 variants=[v.strip().rstrip(',')for v in app.split('pub(crate) enum View {',1)[1].split('}',1)[0].splitlines()if v.strip()]
 names=dict(re.findall(r'Self::(\w+)\s*=>\s*"([^"]+)"',app.split('impl View {',1)[1].split('pub const MENU',1)[0]))
 represented={r['view']for r in records};catalog=[e[0]for _,entries in GROUPS for e in entries]
 assert set(catalog)==set(variants),(set(variants)-set(catalog),set(catalog)-set(variants))
 assert len(catalog)==len(set(catalog))==65
 assert all(names[v]in represented for v in variants)
 byslug={r['slug']:r for r in records}
 inventory={'version':VERSION,'enum_views':65,'coverage_complete':True,'views':[{'variant':v,'public_name':names[v],'frames':[r['slug']for r in records if r['view']==names[v]]}for v in variants],'additional_layouts':sorted(represented-set(names.values()))}
 (ROOT/'docs/media/screen-inventory.json').write_text(json.dumps(inventory,indent=2)+'\n')
 lines=['# Screen guide — Stardust Blackjack0.11.2','',
 '> **Local REVIEW candidate.** These images reproduce the approved renderer/assets. Asset-rights review remains pending; this guide is not a publication permission or license grant.','',
 'This tour assumes you already play blackjack. It explains this app’s readouts, navigation and training behavior, rather than teaching the underlying game. All cards, bankrolls and scores here are **synthetic demonstrations**, created with deterministic seeds and actual native Rust rendering. No connected device, player save, private checkpoint or installed score was used. Independent walkthrough branches intentionally use different demo histories; values across pictures are not one player’s continuous ledger.','',
 'Every one of the65 App screen types is represented, along with the shared recovery layouts and additional table states. The large previews are native240×135 frames enlarged exactly3× with nearest-neighbor scaling; each image links to its native PNG. Animated GIFs contain actual successive native frames and use a shared256-color palette; the PNGs preserve exact decoded RGB565 colors.','',
 'For the complete feature manual use [Feature guide](FEATURE_GUIDE.md), or [Playing and learning](PLAY.md) for a shorter tour.','',
 '## Contents','']
 for title,_ in GROUPS:lines.append(f'- [{title}](#{title.lower().replace(" ","-")})')
 lines += ['- [Save, resume and recovery](#save-resume-and-recovery)','- [Reproducing and auditing this gallery](#reproducing-and-auditing-this-gallery)','',
 '## First-hand flow','',
 '[Player cut](#player-cut) → [shoe-ready notice](#shoe-ready-notice) → [ordinary table](#ordinary-table-and-table-state-variants) → automatic dealer/result presentation → Blue for the next hand. Hold Grey for the unified menu whenever you need an app feature.','',
 '[![Actual native dealing sequence](media/deal.gif)](media/deal.gif)','',
 '*Synthetic example: Blue accepts a cut, Blue deals, and Fast mode presents the real card-reveal sequence. The animation loops; it is not a device recording.*','']
 extra_pages={'TrainingStats':'training-stats-','RoundReview':'round-review','ActionEv':'action-ev-','CountAnswer':'count-answer-','CountResult':'count-result-','CountStats':'count-stats-','WagerResult':'wager-result','WagerStats':'wager-stats-','DeviationQuestion':'deviation-','DeviationContext':'deviation-context-','DeviationReference':'deviation-reference-','DeviationStats':('deviation-stats-','deviation-replay-stats'),'DeviationResult':'deviation-replay-result','DeviationHistory':'deviation-history-','DeviationPolicy':'deviation-policy-','ProfileAmount':'profile-m','Rules':'rules-','Bankroll':'bankroll-','AnalysisSummary':'analysis-summary-','AnalysisRounds':'analysis-rounds-','AnalysisAccuracy':'analysis-accuracy-','AnalysisCosts':'analysis-costs-','AnalysisDecisions':'analysis-decisions-','AnalysisAssumptions':'analysis-assumptions-','Controls':'controls-','Sound':'sound-','Settings':'settings-'}
 for title,entries in GROUPS:
  lines += [f'## {title}','']
  for variant,heading,route,slug,readout,controls,purpose in entries:
   assert slug in byslug,slug
   assert byslug[slug]['view']==names[variant],(variant,slug)
   lines += [f'### {heading}','',f'**Open:** {route}.','',f'[![{heading} — synthetic native screen](media/screens/{slug}.png)](media/screens/native/{slug}.png)','',readout,'',f'**Buttons:** {controls}','',f'**Use:** {purpose}','']
   if variant=='Table':
    lines += ['<details>','<summary>Insurance, extra actions and distinct result presentations</summary>','']
    for image,caption in [('table-insurance','Ace-up dealer offer; the highlighted Yes/No choice is separate from the ordinary turn.'),('table-split','Split-hand presentation and current hand context.'),('table-double','The doubled commitment remains visible with the resulting draw.'),('table-surrender','Surrender settlement uses the same table presentation.'),('table-blackjack','Natural-blackjack result.'),('table-push','Push result.'),('table-win','Win result.'),('table-loss','Loss result.'),('table-bust','Bust result.')]:
     lines += [f'**{caption}**','',f'[![{caption}](media/screens/{image}.png)](media/screens/native/{image}.png)','']
    lines += ['</details>','']
   if variant=='Training':
    lines += ['**Modes:** Off keeps ordinary play quiet; Test reviews after the round; Coach pauses incorrect first choices before acting; Guide displays inline advice. Changing mode does not change the selected Basic/Hi-Lo policy or table rules.','','**Guide-mode table:** this separate real native frame shows the fresh Hi-Lo policy, advice and its Guide count display. Visible advice/counts are assistance; Off’s ordinary table remains quiet.','', '[![Hi-Lo Guide table — synthetic example](media/guide.png)](media/screens/native/table-guide.png)','']
   if variant=='TableSeats':
    lines += ['[![Actual native tablemate sequence](media/tablemates.gif)](media/tablemates.gif)','', '*A separate synthetic player-first branch shows timed tablemate play after the player action. Initial faces and later draws receive their own visible stages; their chips remain separate from the player’s results.*','']
   if variant=='Rules':lines += ['**Current-copy caveat:** the virtual-session page describes the default resumeOff behavior using pre-resume wording. Optional private resume can also preserve active hands/history; see [Save and resume](RESUME.md) for the current behavior.','']
   if variant=='DeviationQuestion':
    pages=[r for r in records if r['slug']in['deviation-insurance','deviation-three-cards','deviation-replay']]
   elif variant in extra_pages:pages=[r for r in records if r['slug'].startswith(extra_pages[variant])and r['view']==names[variant]]
   else:pages=[]
   pages=[r for r in pages if r['slug']!=slug]
   if pages:
    lines += ['<details>',f'<summary>Additional captured pages / variants ({len(pages)})</summary>','']
    for r in pages:lines.append(f'- [{r["slug"]}](media/screens/{r["slug"]}.png) · [native240×135](media/{r["native"]})')
    lines += ['','</details>','']
 lines += ['## Save, resume and recovery','',
 'These are the shared production draw-only recovery layouts, rendered with a fresh synthetic App. They do not read a checkpoint, induce a storage failure, or display anyone’s saved game. Resume is Off by default; only an explicit Resume selection restores a saved live mode. See [Resume](RESUME.md) for journal behavior and checkpoint limits.','']
 lines += ['**Current-layout note:** some long recovery-choice labels are clipped at native width in this build. The captions below spell out the full choices; the captures preserve the actual renderer unchanged.','']
 for heading,slug,description in RECOVERY:
  lines += [f'### {heading}','',f'[![{heading} — synthetic layout](media/screens/{slug}.png)](media/screens/native/{slug}.png)','',description,'']
 lines += ['## Reproducing and auditing this gallery','',
 'Run from the project root with a native Rust toolchain and Python/Pillow installed:','',
 '```sh','python3 tools/publication_screens.py','```','',
 'The helper runs `cargo run --offline --locked --release --features validation --example publication_screens`, stores intermediate PPM frames under `work/`, and writes this guide plus `docs/media`. It uses only public App controls/state and existing fixed public validation cases. Recovery examples use the existing draw-only helpers. It never opens USB, loads a save or imports private recovery data.','',
 'The generator checks all65 current `View` variants against the actual captured public view names and this caption catalogue. It verifies240×135 originals, exact3× PNG replication, GIF frames derived from the real captures and unchanged runtime/source assets during rendering.','',
 '- [Complete screen inventory](media/screen-inventory.json)','- [Frame provenance, synthetic seeds and image hashes](media/screen-manifest.json)','- [Native PNG directory](media/screens/native/)','- [Reproducible native example](../examples/publication_screens.rs)','- [Rendering / gallery helper](../tools/publication_screens.py)','- [Caption catalogue](../tools/publication_catalog.py)','',
 'Generated scores illustrate the interface only. They are not physical-device acceptance evidence, historical player results, a casino advantage claim or a forecast. This preparation creates local review files; it performs no GitHub upload or external publication.','']
 text='\n'.join(lines).replace('Stardust Blackjack0.11.2',f'Stardust Blackjack {VERSION}')
 # Keep ordinary typography readable without changing any renderer-generated text.
 for old,new in [('Blackjack0.11.2','Blackjack 0.11.2'),('the65','the 65'),('all65','all 65'),('native240','native 240'),('exactly3','exactly 3'),('shared256','shared 256'),('verified240','verified 240'),('verifies240','verifies 240'),('exact3','exact 3'),('65current','65 current')]:text=text.replace(old,new)
 (ROOT/'docs/SCREEN_GUIDE.md').write_text(text)
def audit_links():
 path=ROOT/'docs/SCREEN_GUIDE.md';text=path.read_text();headings=[];counts={}
 for heading in re.findall(r'^#{1,6}\s+(.+)$',text,re.M):
  base=re.sub(r'[^\w\- ]','',unicodedata.normalize('NFKC',heading).lower()).replace(' ','-')
  n=counts.get(base,0);counts[base]=n+1;headings.append(base+(f'-{n}'if n else''))
 links=re.findall(r'\]\(([^)]+)\)',text);broken=[]
 for target in links:
  if target.startswith(('http:','https:','mailto:')):continue
  file,_,fragment=target.partition('#')
  if file and not(path.parent/file).exists():broken.append(target)
  if not file and fragment not in headings:broken.append(target)
 assert not broken,broken
 manifest=json.loads((ROOT/'docs/media/screen-manifest.json').read_text())
 actual={p.stem for p in(ROOT/'docs/media/screens/native').glob('*.png')}
 expected={r['slug']for r in manifest['frames']}
 assert actual==expected,{'untracked_native':sorted(actual-expected),'missing_native':sorted(expected-actual)}
 audit={'version':VERSION,'status':'PASS LOCAL REVIEW candidate','views_covered':65,'native_frames':len(expected),'all_local_links_exist':True,'local_link_count':len(links),'heading_anchors':headings,'runtime_and_assets_unchanged_during_render':True,'all_pngs_exact_3x_verified':True,'gif_frames_from_quantized_actual_frames_verified':True,'no_device_save_or_private_recovery_input':True,'hero_sha256':{name:digest(ROOT/'docs/media'/name)for name in['table.png','guide.png','deal.gif']}}
 (ROOT/'docs/media/screen-audit.json').write_text(json.dumps(audit,indent=2)+'\n')
if __name__=='__main__':main()


