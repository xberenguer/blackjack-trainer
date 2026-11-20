# Install Blackjack Trainer

Use the [same web flasher](https://xn--sg8hhb.ws/flasher/) for both devices. Select your exact board, then **Blackjack Trainer**. GitHub releases include separate firmware downloads for [StickS3](#sticks3) and [Waveshare 1.75C](#waveshare-175c-round-device). The [feature guide](../README.md) explains play and the [menu guide](MENUS.md) shows both displays.

## Browser

**[Play web version](https://xberenguer.github.io/blackjack-trainer/)** — the same Rust game and native screen renderer, without installing firmware or developer tools.

Use the Grey/Blue on-screen buttons or Left/Right Arrow keys. Hold an arrow for the same shortcut as holding a device button. Sound starts after a button/key gesture if the browser requires one. Settings → Sound → Test sounds includes Deal, Win and Menu auditions. Fresh preferences are 20% volume, Fast speed and Hi-Lo strategy; saved choices take precedence. Training still boots Off.

Saves belong to that browser profile and site. A different browser, a private window or clearing site data can give you a separate or empty save. They do not automatically synchronize with the Stick. Third-party notices remain in [publication notes](PUBLISHING.md).

For developer builds or local hosting, see [Build and validation](BUILD.md#browser-build-and-local-play).

## StickS3

The firmware target is **M5Stack StickS3 / ESP32-S3,8MB flash and8MB OPI PSRAM**. StickC/StickCPlus/StickCPlus2 are different hardware targets.

### Web flasher

**[Open the firmware flasher](https://xn--sg8hhb.ws/flasher/)** in desktop Chrome or Edge. Choose **StickS3**, then **Blackjack Trainer**. The catalogue follows the latest stable GitHub release; a newly published release normally appears within 15 minutes.

The default flow is **Connect → Flash → Verify**. Device checks, signed downloads and write verification run automatically. Recovery backup and restore are optional under **Advanced**; a layout change, a different recognized project or unknown firmware gets a prominent backup offer without blocking Flash. Choosing a backup also enables best-effort automatic rollback while the tab remains open. An existing matching trainer layout receives an app-only update, preserving saves. First installations use the reviewed boot/partition/application bundle and initialize only the new trainer journal regions. **StickS3 browser installation is marked Preview**: the migration/recovery logic has software coverage, while the complete browser USB installation path still needs physical qualification.

### Manual installation paths

| Your device | Status and path |
|---|---|
| Already running this Rust trainer with the matching partition table | App-only updates are tested. Preserve a backup, then update only the application at0x10000. |
| Factory firmware, the earlier C++ trainer, another app, or unknown partitions | This is a first installation/partition migration. The app-only update is not a complete first-install bundle. Use the maintainer's board-specific migration instructions after a full backup. |

The current release evidence validates app updates on the existing trainer layout. It does not establish that an arbitrary factory installation is ready for the same update command. The release also includes a complete bundle for the Preview web flasher above; its migration path has separate safeguards and qualification limits. The application binary alone is not a universal full-device image.

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

The device should boot to the player-cut screen, unless you have opted into a valid saved-session chooser. Hold Grey to check the small version in the menu. The current version is shown in small grey type. Your saved strategy remains selectable. New setups start at 20% volume and Fast speed.

Press Blue twice into a hand, check Grey/Blue play, then use Settings → Sound → Test sounds → Deal or Win if desired. The [release notes](RELEASE-0.12.2.md) link the current verification results.

### If something goes wrong

If USB is missing, try a known data cable/direct port and the board's documented download mode. If the firmware is incompatible, stop and compare hardware/partitions/version rather than erasing the whole device. Keep your original backup for board-specific recovery. The[firmware reference](../firmware/README.md) explains partitions, USB diagnostics and private resume; the[build guide](BUILD.md) covers developer setup.

## No account or real money

The app uses virtual chips and local saves. It does not connect the device to a casino account. The firmware does not start Wi-Fi/Bluetooth for play.


## Waveshare 1.75C round device

Use the [Waveshare installation and controls guide](WAVESHARE.md). Select Waveshare 1.75C in the same device flasher. This build uses two physical buttons, a native round layout and 270° enclosure orientation, with touch disabled. Installation writes only its application at `0x200000`; retain the factory partition map. The Stick instructions above do not apply to this board.
