# Run in the browser or on StickS3

This guide covers running the app; the [quickstart](PLAY.md) covers its controls and the [screen guide](SCREEN_GUIDE.md) shows every view. No blackjack rules lesson is required.

## Browser

The browser uses the same Rust game and native screen renderer. You do not need a Stick or a Rust toolchain to run an included prebuilt browser.

1. Unzip the complete review/release folder and open a terminal in its top-level folder.
2. With Python3 installed, run:

   ```sh
   python3 -m http.server 8000 --bind 127.0.0.1 --directory browser
   ```

3. Open [http://127.0.0.1:8000](http://127.0.0.1:8000).
4. Use the Grey/Blue on-screen buttons or Left/Right Arrow keys. Hold an arrow for the same shortcut as holding a device button.

Keep the terminal open while playing. Use Ctrl+C there to stop the server. Opening index.html directly is insufficient because the browser must fetch the WASM and module files.

Sound starts after a button/key gesture if the browser requires one. Start at Settings → Sound if you want to audition the cues. Fresh preferences are20% volume, Fast speed and Hi-Lo strategy; saved choices take precedence. Training still boots Off.

Saves belong to that browser profile and origin. Changing the port/browser, using a private window or clearing site data can give you a separate or empty save. They do not automatically synchronize with the Stick. The supplied browser is for local review until the asset/publication review in[PUBLISHING](PUBLISHING.md) is complete.

## StickS3

The firmware target is **M5Stack StickS3 / ESP32-S3,8MB flash and8MB OPI PSRAM**. StickC/StickCPlus/StickCPlus2 are different hardware targets.

### Choose the correct installation path

| Your device | Status and path |
|---|---|
| Already running this Rust trainer with the matching partition table | App-only updates are tested. Preserve a backup, then update only the application at0x10000. |
| Factory firmware, the earlier C++ trainer, another app, or unknown partitions | This is a first installation/partition migration. The app-only update is not a complete first-install bundle. Use the maintainer's board-specific migration instructions after a full backup. |

The current release evidence validates app updates on the existing trainer layout. It does not establish that an arbitrary factory installation is ready for the same update command. A public one-step first-install bundle still needs separate qualification; do not describe the app binary as a universal full-device image.

The expected partition-table SHA256 for this build is:

```text
65b87532f84d5ddec377c550f8bef2fcc846a3aea884cf8c0f25f1b12d3f421a
```

Its source is[firmware/partitions.csv](../firmware/partitions.csv). It reserves public-save and optional private-resume areas. Replacing a partition table is a migration step, not part of an ordinary update.

### Prepare an app-only update

Use a data-capable USB cable, close any serial monitor, and identify the device port. On macOS/Linux it is commonly a /dev/cu.usbmodem… or /dev/ttyACM… path; on Windows it is a COM port. Substitute your actual port for PORT below.

The tested command syntax is esptool4.9.0:

```sh
python3 -m venv .tools
.tools/bin/python -m pip install esptool==4.9.0
```

On Windows, use .tools\Scripts\python.exe for that interpreter. If the Stick is already in blinking download mode, leave it there for the operation. Otherwise esptool may reset it into download mode automatically.

### Back up before writing

Choose an empty backup filename outside the repository. Create a sibling folder named private-blackjack-backups first (for example, with mkdir ../private-blackjack-backups). The following complete8MB backup writes into that sibling folder:

```sh
.tools/bin/python -m esptool --chip esp32s3 --port PORT --baud 921600 --after no_reset read_flash 0x0 0x800000 ../private-blackjack-backups/before-blackjack.bin
```

Keep this private: it can contain the previous app, preferences and saved session data. A normal app update should preserve those areas. Do not attach your flash dump to an issue or commit it to GitHub.

### Write the application

Use the application file belonging to the reviewed version and verify its SHA256 against that version's manifest. The source builder puts the file at firmware/build/stardust-sticks3.bin; a release may supply it separately.

```sh
.tools/bin/python -m esptool --chip esp32s3 --port PORT --baud 921600 --before default_reset --after watchdog_reset write_flash 0x10000 firmware/build/stardust-sticks3.bin
```

This command writes the application only. It does not request a full erase or write a bootloader/partition table. Esptool verifies the write. See Espressif's[write/read-flash documentation](https://docs.espressif.com/projects/esptool/en/release-v4/esp32s3/esptool/basic-commands.html) for the command format.

### Check the result

The device should boot to the player-cut screen, unless you have opted into a valid saved-session chooser. Hold Grey to check the small version in the menu. For0.11.2, verify Hi-Lo under Training → Strategy; your saved choice remains selectable. New preferences are20% volume/Fast speed.

Press Blue twice into a hand, check Grey/Blue play, then use Settings → Sound → Test deal/Test win if desired. The version's[validation record](../validation/release-0.11.2.json) distinguishes automated checks, actual-device results and physical audio observations.

### If something goes wrong

If USB is missing, try a known data cable/direct port and the board's documented download mode. If the firmware is incompatible, stop and compare hardware/partitions/version rather than erasing the whole device. Keep your original backup for board-specific recovery. The[firmware reference](../firmware/README.md) explains partitions, USB diagnostics and private resume; the[build guide](BUILD.md) covers developer setup.

## No account or real money

The app uses virtual chips and local saves. It does not connect the device to a casino account. The firmware does not start Wi-Fi/Bluetooth for play.
