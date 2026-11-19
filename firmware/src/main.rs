#![no_std]
#![no_main]
mod benchmark;
mod flash_chunks;
mod legacy_nvs;
mod panel;
mod power;
mod resume;
mod speaker;
mod storage;
mod storage_format;
mod usb;

use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    gpio::{DriveMode, Input, InputConfig, Level, Output, OutputConfig, Pull},
    i2c::master::{Config as I2cConfig, I2c},
    i2s::master::{BitOrder, Channels, DataFormat, Endianness, I2s, Polarity, TdmConfig, WsWidth},
    ledc::{
        LSGlobalClkSource, Ledc, LowSpeed,
        channel::{self, ChannelIFace},
        timer::{self, TimerIFace},
    },
    psram::{Psram, PsramConfig, PsramMode, PsramSize, SpiRamFreq},
    rng::{Trng, TrngSource},
    spi::master::{Config as SpiConfig, Spi},
    time::{Instant, Rate},
    usb::usb_serial_jtag::UsbSerialJtag,
};
use stardust_blackjack::{
    app::App,
    canvas::{HEIGHT, WIDTH},
};
esp_bootloader_esp_idf::esp_app_desc!();
// Fully valid immutable object image in flash; copied directly to owned PSRAM.
#[used]
static APP_TEMPLATE: App = App::EMPTY;
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    esp_println::println!("STARDUST RUST PANIC: {info}");
    loop {
        core::hint::spin_loop()
    }
}
fn boot_stage(stage: &str) {
    esp_println::println!("STARDUST BOOT {stage}");
}
fn boot_result<T, E: core::fmt::Debug>(result: Result<T, E>, stage: &str) -> T {
    match result {
        Ok(value) => {
            boot_stage(stage);
            value
        }
        Err(error) => {
            let delay = Delay::new();
            loop {
                esp_println::println!("STARDUST BOOT FAILED stage={stage} error={error:?}");
                delay.delay_millis(500);
            }
        }
    }
}
fn ms() -> u32 {
    Instant::now().duration_since_epoch().as_millis() as u32
}
fn us() -> u64 {
    Instant::now().duration_since_epoch().as_micros()
}
struct Debounce {
    raw: [bool; 2],
    stable: [bool; 2],
    changed: [u32; 2],
}
impl Debounce {
    fn poll(&mut self, down: [bool; 2], now: u32) -> [bool; 2] {
        for i in 0..2 {
            if down[i] != self.raw[i] {
                self.raw[i] = down[i];
                self.changed[i] = now
            }
            if now.wrapping_sub(self.changed[i]) >= 30 {
                self.stable[i] = self.raw[i]
            }
        }
        self.stable
    }
}
#[esp_hal::main]
fn main() -> ! {
    let p = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    boot_stage("HAL initialized");
    let blue = Input::new(p.GPIO11, InputConfig::default().with_pull(Pull::Up));
    let grey = Input::new(p.GPIO12, InputConfig::default().with_pull(Pull::Up));
    let bus = boot_result(
        I2c::new(
            p.I2C1,
            I2cConfig::default().with_frequency(Rate::from_khz(100)),
        ),
        "I2C 100kHz",
    )
    .with_sda(p.GPIO47)
    .with_scl(p.GPIO48);
    let mut power = power::Power::new(bus);
    esp_println::println!(
        "STARDUST BOOT PM1={} ES8311={} status_led_off={} led_cfg={} led_drive={} amp_output={} amp_mode={} amp_drive={} amp_function={} amp_level={}",
        power.pmic,
        power.codec,
        power.led_off,
        power.led_config,
        power.led_drive,
        power.amp_output,
        power.amp_mode,
        power.amp_drive,
        power.amp_function,
        power.amp_level
    );
    boot_stage("PSRAM initialization");
    let memory = Psram::new(
        p.PSRAM,
        PsramConfig {
            mode: PsramMode::OctalSpi,
            size: PsramSize::Size(8 * 1024 * 1024),
            ram_frequency: SpiRamFreq::Freq80m,
            ..PsramConfig::default()
        },
    );
    let (ptr, len) = memory.raw_parts();
    esp_println::println!("STARDUST BOOT PSRAM bytes={len}");
    let app_bytes = core::mem::size_of::<App>();
    let stride = (app_bytes + 63) & !63;
    let snap_offset = stride * 3;
    let private_offset = (snap_offset + WIDTH * HEIGHT * 2 + 63) & !63;
    let bound_offset = (private_offset + stardust_blackjack::resume_private::MAX_BYTES + 63) & !63;
    let public_offset = (bound_offset + stardust_blackjack::resume_private::MAX_BYTES + 63) & !63;
    assert!(len >= public_offset + stardust_blackjack::persistence::SIZE);
    assert_eq!(ptr.align_offset(core::mem::align_of::<App>()), 0);
    assert!(!core::mem::needs_drop::<App>());
    let (boot_seed, boot_nonce) = entropy(p.RNG, p.ADC1);
    // SAFETY: all PSRAM ranges are aligned, initialized and disjoint. Immutable App::EMPTY images
    // construct active/frozen/candidate directly without large stack values. Resume exclusively owns
    // the latter two and opaque byte regions; USB owns only the separate RGB565 snapshot region.
    let (mut app, snapshot, regions) = unsafe {
        for offset in [0, stride, stride * 2] {
            core::ptr::copy_nonoverlapping(
                core::ptr::addr_of!(APP_TEMPLATE),
                ptr.add(offset).cast::<App>(),
                1,
            );
        }
        let app = &mut *ptr.cast::<App>();
        assert!(app.boot(boot_seed));
        let s = core::slice::from_raw_parts_mut(ptr.add(snap_offset).cast::<u16>(), WIDTH * HEIGHT);
        s.fill(0);
        let regions = resume::Regions {
            frozen: ptr.add(stride).cast(),
            candidate: ptr.add(stride * 2).cast(),
            private: ptr.add(private_offset),
            bound: ptr.add(bound_offset),
            public: ptr.add(public_offset),
            template: core::ptr::addr_of!(APP_TEMPLATE),
        };
        (app, s, regions)
    };
    boot_stage("App initialized in PSRAM");
    let mut store = storage::Storage::new(p.FLASH, boot_nonce);
    load_saved(app, &mut store);
    esp_println::println!(
        "STARDUST BOOT storage={} imported={}",
        store.enabled(),
        store.imported
    );
    let mut ledc = Ledc::new(p.LEDC);
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);
    let mut timer = ledc.timer::<LowSpeed>(timer::Number::Timer0);
    boot_result(
        timer.configure(timer::config::Config {
            duty: timer::config::Duty::Duty10Bit,
            clock_source: timer::LSClockSource::APBClk,
            frequency: Rate::from_hz(256),
        }),
        "backlight timer 256Hz/10bit",
    );
    let mut backlight = ledc.channel(channel::Number::Channel0, p.GPIO38);
    boot_result(
        backlight.configure(channel::config::Config {
            timer: &timer,
            duty_pct: 100,
            drive_mode: DriveMode::PushPull,
        }),
        "backlight channel",
    );
    let spi = boot_result(
        Spi::new(
            p.SPI3,
            SpiConfig::default().with_frequency(Rate::from_mhz(40)),
        ),
        "LCD SPI3 40MHz",
    )
    .with_sck(p.GPIO40)
    .with_mosi(p.GPIO39);
    let mut panel = panel::Panel::new(
        spi,
        Output::new(p.GPIO45, Level::High, OutputConfig::default()),
        Output::new(p.GPIO41, Level::High, OutputConfig::default()),
        Output::new(p.GPIO21, Level::High, OutputConfig::default()),
    );
    boot_stage("ST7789 initialized");
    let i2s_config = TdmConfig::new_tdm_philips()
        .with_sample_rate(Rate::from_hz(22050))
        .with_data_format(DataFormat::Data16Channel16)
        .with_channels(Channels::new(2, 0b11, None))
        .with_ws_width(WsWidth::Bits(16))
        .with_ws_polarity(Polarity::ActiveLow)
        .with_msb_shift(true)
        .with_endianness(Endianness::LittleEndian)
        .with_bit_order(BitOrder::MsbFirst);
    let n = i2s_config.diagnostic_numbers();
    esp_println::println!(
        "STARDUST BOOT I2S txformat={} rxformat={} txbits={} rxbits={} txchannels={} rxchannels={} wsbits=16 maxws={}",
        n[0],
        n[1],
        n[2],
        n[3],
        n[4],
        n[5],
        n[6]
    );
    let i2s = boot_result(
        I2s::new(p.I2S0, p.DMA_CH0, i2s_config),
        "I2S 22050Hz stereo WS16",
    )
    .with_mclk(p.GPIO18);
    let tx = i2s
        .i2s_tx
        .with_bclk(p.GPIO17)
        .with_ws(p.GPIO15)
        .with_dout(p.GPIO14)
        .build();
    let mut sound = speaker::Speaker::new(tx);
    boot_stage("audio DMA initialized");
    let mut debug = usb::Debug::new(UsbSerialJtag::new(p.USB_DEVICE));
    debug.pmic = power.pmic;
    debug.codec = power.codec;
    debug.led_off = power.led_off;
    debug.save = store.enabled();
    debug.imported = store.imported;
    // SAFETY: the PSRAM regions above are initialized, aligned, unique and live forever.
    let mut resume = unsafe { resume::Resume::new(regions, boot_nonce) };
    resume.boot(app, &mut store, ms());
    boot_stage("resume scan complete");
    boot_stage("running");
    let mut debounce = Debounce {
        raw: [false; 2],
        stable: [false; 2],
        changed: [0; 2],
    };
    let mut previous = [false; 2];
    let mut brightness = 0;
    let mut lastsave = ms();
    let mut last_led_check = 0u32;
    let delay = Delay::new();
    let mut muted_until = 0u32;
    let mut amplifier_ready_at = 0u32;
    let mut audio_policy = (app.sound_enabled, app.audio_mask);
    let mut shown_boot = None;
    let mut last_retry = 0u32;
    app.dirty = true;
    loop {
        let now = ms();
        if now.wrapping_sub(last_led_check) >= 1000 {
            power.keep_status_led_off();
            power.verify_amplifier();
            debug.led_off = power.led_off;
            last_led_check = now;
        }
        let down = debounce.poll([blue.is_low(), grey.is_low()], now);
        if down != previous {
            sound.interrupt();
            previous = down;
        }
        debug.unsaved = resume.ram_only;
        debug.resume_writes = resume.writes;
        debug.resume_errors = resume.errors;
        debug.resume_blocked = resume.blocked || resume.choosing();
        debug.resume_saved = resume.checkpoint_current(app);
        let external = debug.receive(
            app,
            now,
            snapshot,
            resume.outputs_allowed(app),
            resume.choosing(),
        );
        if !external.is_empty() {
            sound.interrupt();
        }
        if resume.choosing() {
            let before = resume.chooser.state;
            resume.chooser_events(&mut app, &mut store, down, &external, now);
            if resume.choosing()
                && (shown_boot != Some(resume.chooser.state) || before != resume.chooser.state)
            {
                if resume.blocked {
                    let row = match resume.chooser.state {
                        stardust_blackjack::resume_ui::BootState::ReadFailure { row } => row,
                        _ => 0,
                    };
                    if resume.saved_scores_unavailable() {
                        app.render_resume_read_error_choice(row);
                    } else {
                        app.render_resume_storage_error_choice(row);
                    }
                } else {
                    app.render_resume_chooser(&resume.chooser);
                }
                panel.present(&app.frame.pixels, app.flipped, || {
                    sound.pump(app.sound_enabled);
                    debug.pump(snapshot);
                });
                shown_boot = Some(resume.chooser.state);
            }
        } else if resume.accept_input(down) {
            app.buttons(down, now);
            app.tick(now);
            for event in external {
                app.event(event);
            }
        }
        if resume.blocked && now.wrapping_sub(last_retry) >= 250 {
            last_retry = now;
            // Failure choices are explicit; do not reset their focus or neutral gate automatically.
        }
        let next_audio_policy = (app.sound_enabled, app.audio_mask);
        if next_audio_policy != audio_policy {
            // Stop an active category immediately, while still allowing an
            // explicit Settings audition to bypass category filtering later.
            sound.cancel();
            audio_policy = next_audio_policy;
        }
        sound.set_volume(app.audio_volume);
        let audible = app.sound_enabled && app.audio_volume > 0;
        if !audible {
            sound.cancel();
            muted_until = now.wrapping_add(100);
        }
        let was_on = power.amp_on;
        power.amplifier(audible && (now.wrapping_sub(muted_until) as i32) >= 0);
        if power.amp_on && !was_on {
            // Prime silent DMA before the first cue after enabling the amplifier.
            // Nonblocking lead-in also covers codec/amp startup transients.
            amplifier_ready_at = now.wrapping_add(10);
        }
        sound.pump(audible);
        if app.brightness != brightness {
            brightness = app.brightness;
            backlight
                .set_duty(((brightness as u16 * 100) / 255).max(1) as u8)
                .unwrap();
        }
        if app.dirty && !resume.choosing() {
            let t = us();
            if resume.ram_only {
                app.resume_enabled = false;
            }
            app.render();
            debug.render_us = us() - t;
            if !resume.acknowledge(app, &mut store, now) {
                app.render_resume_storage_error();
            }
            if resume.ram_only {
                app.render_unsaved_banner();
            }
            let t = us();
            panel.present(&app.frame.pixels, app.flipped, || {
                sound.pump(app.sound_enabled);
                debug.pump(snapshot);
            });
            debug.push_us = us() - t;
            if !resume.blocked {
                app.checkpoint_output_clock(ms());
            }
            app.dirty = false;
            debug.frames = debug.frames.wrapping_add(1);
        }
        if resume.outputs_allowed(app)
            && !sound.busy()
            && (!audible || (power.amp_on && (now.wrapping_sub(amplifier_ready_at) as i32) >= 0))
        {
            if let Some(c) = app.pop_sound() {
                if audible {
                    sound.cue(c);
                }
            }
        }
        if debug.benchmark_requested {
            debug.benchmark_requested = false;
            if let Some(region) = resume.benchmark_region() {
                // SAFETY: coordinator grants exclusive inactive App region; live/frozen state stays separate.
                unsafe {
                    benchmark::run(region, core::ptr::addr_of!(APP_TEMPLATE), &mut sound);
                }
                app.checkpoint_output_clock(ms());
            }
        }
        sound.pump(app.sound_enabled);
        debug.pump(snapshot);
        resume.step(app, &mut store, now);
        if now.wrapping_sub(lastsave) >= 1000 {
            lastsave = now;
            if !resume.choosing() && !resume.blocked && !app.resume_enabled && !resume.ram_only {
                let save = app.save();
                if store.save(save) {
                    resume.record_public_saved(save);
                }
            }
            debug.save_writes = store.writes;
            debug.save_errors = store.errors;
        }
        debug.audio_samples = sound.samples;
        debug.audio_underruns = sound.underruns;
        debug.amp_on = power.amp_on;
        debug.amp_output = power.amp_output;
        debug.amp_mode = power.amp_mode;
        debug.amp_drive = power.amp_drive;
        debug.amp_function = power.amp_function;
        debug.amp_level = power.amp_level;
        if debug.frames == 1 && resume.outputs_allowed(app) {
            debug.status(app, now);
            debug.frames = 2;
        }
        delay.delay_micros(500);
    }
}

fn entropy(
    rng: esp_hal::peripherals::RNG<'static>,
    adc: esp_hal::peripherals::ADC1<'static>,
) -> (u64, [u8; 16]) {
    let source = TrngSource::new(rng, adc);
    let trng = Trng::try_new().unwrap();
    let mut b = [0; 24];
    trng.read(&mut b);
    drop(trng);
    drop(source);
    let n = u64::from_le_bytes(b[..8].try_into().unwrap());
    let nonce = b[8..24].try_into().unwrap();
    b.fill(0);
    (n, nonce)
}

#[inline(never)]
fn load_saved(app: &mut App, store: &mut storage::Storage) {
    let saved = store.load();
    if store.read_failed() {
        boot_stage("save read failed; offering retry or new unsaved session");
        return;
    }
    if let Some(saved) = saved {
        if !app.load(saved) {
            boot_stage("save rejected by App");
        }
    }
}
