# Local esp-hal1.2.2 correction

Source copied into this project from the installed official esp-hal1.2.2 package. Original licenses and source retained. Cargo home and other projects remain untouched.

Two changes:

1. `ld/esp32s3/esp32s3.x`: `.rwdata_dummy` now reserves `ABSOLUTE(_rwtext_end) - ORIGIN(vectors_seg)` bytes. This measures the entire actual aliased IRAM span including vectors, alignment and RAM code. On the installed Xtensa GCC8.4 linker, the original sum of `SIZEOF` expressions produced only0x2660 bytes despite vectors adding another0x400. The resulting IRAM load40378000..4037aa60 aliases DRAM3fc88000..3fc8aa60, while `.data` began3fc8a660. The bootloader consequently overwrote the first1024 bytes of Rust data/switch tables with RAM instructions. Actual-device effects included invalid I2S WS widths and exception recursion while formatting an enum.
2. `src/i2s/master/mod.rs`: `TdmConfig::diagnostic_numbers` returns raw TX/RX format discriminants, data-bit getter results, channel counts and the metadata WS limit. Firmware logs only these primitive public hardware parameters before I2S initialization.

The corrected ELF has `.rwdata_dummy`0x2a60 and `.data`3fc8aa60. `tools/build.py` independently reads actual ELF sections and refuses packaging if any IRAM physical alias intersects actual DRAM data, BSS, noinit or stack. It separately rejects main frames≥32KB and requires CPU0 stack≥256KB. No stack guard is disabled.

See `build/memory-proof.json` for exact current ranges. Device validation is recorded separately by the installing agent.
