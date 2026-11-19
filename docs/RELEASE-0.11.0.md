# 0.11.0 — sound, settings and speed

Installed on the connected Stick. The amplifier enable pin is now configured as an output before enabling it; the user confirmed deal and win sounds are audible at50%.

Under **hold Grey → Settings**:

- **Sound:** master mute,0–100% volume, separate Menus/Cards/Results switches and deal/win/menu tests. Volume uses one-point steps below10%, then five-point steps. Grey lowers, Blue raises, hold Blue tests, hold Grey returns. Tests respect master mute and zero volume.
- **Display:** percentage brightness and Flip screen.
- **Speed:** Normal(default) or Fast. Fast shortens automatic dealing/dealer pacing while preserving every exposed card. Questions and reviews remain untimed; hold thresholds are unchanged.

The approved table and B menu remain. Preferences use the same-sized public V10 save. V1–V9 receive50%/all audio/Normal defaults without changing scores. Old private checkpoint binding is preserved until explicit restore and verified migration.

All205 native tests, strict Clippy, current WASM/Chrome and linked memory/stack checks passed. Actual device checks covered sound tests, amplifier on/off readbacks, volume, categories, speed, brightness and parent navigation:12 verified saves, zero save errors and zero DMA underruns. Saved scores are unchanged. Inputs stopped at Cut/Off with50% volume and Normal speed. See[scoped validation](../validation/release-0.11.0.json).
