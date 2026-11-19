# Linked Xtensa frame audit

This WORK-only helper reads a fully linked ELF32 Xtensa image and its local objdump output. It never builds firmware, opens a device, reads a checkpoint, or changes runtime source.

```sh
python3 xtensa_frames.py firmware.elf --allow-existing --json frames.json
python3 -m unittest discover -s . -p 'test_*.py'
```

Importable API: `audit_elf(path, objdump=None, limit=32768, decoder_limit=8192, allow_existing=False) -> dict`. The `passed` field and CLI exit status cover ordinary per-function caps, known stack allocations, and unresolved ordinary SP mutations. A missing symbol table / empty sized-function audit is rejected. The default strict32KiB policy rejects the two known larger non-decoder frames; production uses the explicitly approved `allow_existing=True` profile.

All `entry a1,N` allocations are included. Large-frame prologues load the linked32-bit allocation literal, subtract/add it to the already-adjusted SP, then issue `movsp`; their true frame is ENTRY plus the additional allocation. Literals are read from loaded ELF sections, not inferred from a nearest-symbol label. Decoding follows register constants/affine SP values; unknown or body SP mutations fail ordinary-function auditing.

Instruction ranges come from actual executable STT_FUNC symbols and sizes. A whole-section objdump can decode inline literal pools as fictitious `movsp` instructions; those outside function ranges are excluded. Zero-size naked assembly aliases are grouped into inferred ranges and reported for manual bootstrap/interrupt/context review, rather than silently certified as normal Rust functions.

Production policy:

- Ordinary sized functions, including main: at most32768bytes.
- All57 actual `Wire::get`/`Wire::read_into` monomorphizations plus `App.resume_read`: at most8192bytes, including generic symbol suffixes.
- Only `App.load` and `wager_practice::Model.resume_valid` may use the named existing exception, each at most65536bytes. Both remain listed.

`firmware/build/linked-stack-proof.json` audits the packaged final ELF SHA256 `4e6ae4a9a0515167b622cd23f79413393bf0172fc4b4b2a99eb2088fc17ec67c`:

-600 sized functions pass; no unresolved ordinary SP mutation.
-57 decoders, largest1104bytes; App.resume_read432bytes.
-Only larger frames are App.load39632bytes and wager validator41312bytes (approved exceptions).
-Actual linked CPU stack305176bytes; main20576bytes.
-Resolved App decoding chain3216bytes; full restore_bound chain64752bytes, including the conservative validator/panic-format path.
-Conservative visible prefix main20576 +Resume.boot2720 +Resume.scan160 +restore_bound64752 =88208bytes, leaving216968bytes before interrupt/ROM allowances.
-28 inferred naked assembly ranges require manual context/bootstrap review; largest recognized local allocation256bytes.

These resolved chains are not a whole-program stack proof. JSON retains unresolved function-pointer calls, external/ROM sites and call-chain caveats. Naked context restoration and ROM code cannot be assigned complete stack bounds from this ELF alone. Interrupt/nesting/ROM reserve remains an explicit platform assumption.

`baseline-frames.json` rejects the installed predecessor: App.resume_read231056bytes plus nested wager decoder121456bytes yields a resolved353616-byte decode path, exceeding the305176-byte stack before any interrupt reserve. These totals include the32-byte ENTRY that an immediate-only prologue check misses.

The five synthetic tests independently check ENTRY+literal subtraction, negative-literal addition, movi/neg/add allocation, register/input unknowns, and late body stack mutations. Actual old-ELF rejection and patched-ELF acceptance are additional integration regressions.
