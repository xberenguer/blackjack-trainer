# Optional private session resume

Resume defaults OFF in Settings. Normal boot starts a fresh Off game at$1000 and the table minimum, retaining saved public preferences/lifetime accuracy. With Resume ON and a matching valid checkpoint, boot offers Resume saved session or Start a new session. Button tails are consumed until neutral. Resume restores the exact live mode, unlike ordinary preference loading.

Native checkpoints coalesce after 1200 ms of stable state; the browser captures cooperatively after public acknowledgment using its frame budget. They are not a promise that every tap is privately durable. A new meaningful event/automatic stage/aid exposure first commits a public epoch, even if its public statistics bytes are unchanged. Its frame, status and grading cue wait for verification. Private capture/write follows in bounded steps while the live application can continue. New input aborts a stale job. Power loss between public and private commit refuses the old private checkpoint and starts fresh with verified public progress.

## Formats and invariants

- Public SBTR V13:2636 bytes, default-off resume boolean at583; byte202 stores volume0–100 and byte203 stores Menus/Cards/Results bits0–2 plus Fast bit3. V1–V9 initialize volume20%, all categories enabled and Fast speed while retaining exact existing scores/preferences. Public SBRJ V2 wraps exact raw SBTR bytes with generation64, checkpoint epoch64 and independent128-bit transaction nonce. Normal boot's Off override occurs after reading these raw binding bytes.
- Private SBRS V1/schema`0x000a0001`:48-byte header with declared length, sequence/epoch, saved clock and header/payload CRC32; required TLVs contain explicit typed private fields and the matching public packet. Integers/floats use fixed little-endian wire encoding; options/enums are checked. No pointers, Rust padding, framebuffer or allocator state is serialized.
- SBTX V1:64-byte transaction header followed by exact public bytes and SBRS. Restore requires equality with the latest trusted public packet, generation, epoch and nonce. An old private packet cannot roll back a grade, assistance exposure, reset or scoreless Off move. Private-ahead-public is refused too.
- Private RSMJ V1: two256KB slots at`0x770000`;40-byte header and fixed32-byte footer. Footer-sector erase invalidates the old target marker first; only used payload sectors are erased. One4KB erase or at most512-byte hash/program/verify per step. Footer commit is last; CRC covers declared unpadded bytes. Internal-RAM staging pads only the final physical word with erased bits.

The maximum opaque bound packet is262000 bytes. Fully populated128-entry training/count/wager/deviation/first-answer rings, a full128-round ring, pending helped count/wager questions, pending deviation replay and exact analytic moments/Kahan sums measured139626 SBRS bytes/142326 bound bytes. All histories, private generators/origins, physical shoe pools/order/RNG, hands/funding, exposed-count deduplication, immutable grading/help, pending faces and clock-relative state are retained. All owning-field codecs use exhaustive destructuring so new fields require review.

Restore recursively decodes each field/array element in place into a separate EMPTY PSRAM candidate and validates structural/model/shoe/count invariants before replacing the active reference. Failed restore never mutates the active game or adds totals. Controls/audio reset and frame regeneration are intentional. Current initial/queued faces, dealer/NPC/Burn animation and result review receive fresh readable dwell after restore or an output gate delay; this is the clock-safety exception to exact remaining-time continuation. An ordinary successful output ACK preserves held-control and queued-cue state.

## Failure behavior

Read I/O never means empty. Valid old Rust journals precede NVS. Recognizable committed public corruption and newer unfinished valid public headers block private restore, retain corroborated public progress and require Retry or explicit Fresh. Generations advance above validated hints; Fresh uses an independent nonce and invalidates/tombstones old private state after public commit.

Permanent save/read failure offers Retry or Start fresh without saving. That deliberate RAM fork aborts all jobs and discards current pending question/outcome/RNG/history. It loads only last acknowledged public preferences/totals when known, starts Off/$1000/minimum, displays UNSAVED and latches all persistence/resume off until reboot. If scores could not be read, they are explicitly unavailable and the RAM session uses defaults. It does not write/delete either journal, including any uncertain public record. Reboot may offer the last valid durable checkpoint.

Private payloads cross only opaque internal flash/browser persistence boundaries. They are absent from public Save export, USB status/frame commands, general runtime APIs, help and downloads. Digital audio gain parity and DMA timing do not claim acoustic or physical-button acceptance.

## V9 private checkpoint compatibility

The private SBRS field order and schema remain unchanged. Audio volume/category
and speed preferences are carried only by its existing mirrored public Save TLV;
restore applies those preferences before comparing the reconstructed public Save.
New view tags append after the previous tags. A V9 public commit and its V9 private
checkpoint can therefore be validated and offered without an eager rewrite. After
explicit restore, the next acknowledged public checkpoint migrates canonically to
V13 before any new private capture can bind. Waiting at the chooser never performs
that migration, and a stale unanswered question cannot replace newer public progress.

<a id="round-display-preferences-and-older-saves"></a>

## Display preferences and older saves

V11 added the round background in byte 10 bits 3–4. V12 added the card-back choice in bits 5–6. V13 uses bit 7 for the compact display’s Black background setting. Earlier compact saves default to Blue. Waveshare backgrounds, saved card backs, other preferences and lifetime scores carry over. All versions keep their original reserved-bit checks. The packet remains 2,636 bytes.

Appearance choices travel in the existing mirrored public Save TLV. The private body schema stays the same and the preview view has a new tag. Older private checkpoints are verified against their original public bytes and migrate to V13 after explicit resume and acknowledgment. The Display menu keeps the saved Back selection IDs for each device.
