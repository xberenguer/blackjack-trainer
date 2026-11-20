//! Bounded native USB debug. Frame is a frozen logical RGB565 snapshot, never live pixels.
use core::fmt::Write;
use esp_hal::{Blocking, usb::usb_serial_jtag::UsbSerialJtag};
use heapless::{Deque, String, Vec};
use stardust_blackjack::{
    app::App,
    controls::{Event, Key},
    round::{HEIGHT, WIDTH},
};
pub struct Debug {
    ack_status: String<1800>,
    pub benchmark_requested: bool,
    pub unsaved: bool,
    pub resume_writes: u32,
    pub resume_errors: u32,
    pub resume_blocked: bool,
    pub resume_saved: bool,
    usb: UsbSerialJtag<'static, Blocking>,
    tx: Deque<u8, 4096>,
    frame: bool,
    at: usize,
    pub frames: u32,
    pub max_poll_ms: u32,
    last_poll: u32,
    pub render_us: u64,
    pub push_us: u64,
    pub pmic: bool,
    pub codec: bool,
    pub led_off: bool,
    pub amp_on: bool,
    pub amp_output: bool,
    pub amp_mode: u8,
    pub amp_drive: u8,
    pub amp_function: u8,
    pub amp_level: u8,
    pub save: bool,
    pub imported: bool,
    pub audio_samples: u64,
    pub audio_underruns: u32,
    pub save_writes: u32,
    pub save_errors: u32,
}
impl Debug {
    pub fn new(usb: UsbSerialJtag<'static, Blocking>) -> Self {
        Self {
            ack_status: String::new(),
            benchmark_requested: false,
            unsaved: false,
            resume_writes: 0,
            resume_errors: 0,
            resume_blocked: false,
            resume_saved: false,
            usb,
            tx: Deque::new(),
            frame: false,
            at: 0,
            frames: 0,
            max_poll_ms: 0,
            last_poll: 0,
            render_us: 0,
            push_us: 0,
            pmic: false,
            codec: false,
            led_off: false,
            amp_on: false,
            amp_output: false,
            amp_mode: 0xff,
            amp_drive: 0xff,
            amp_function: 0xff,
            amp_level: 0xff,
            save: false,
            imported: false,
            audio_samples: 0,
            audio_underruns: 0,
            save_writes: 0,
            save_errors: 0,
        }
    }
    fn queue(&mut self, b: &str) {
        if self.tx.capacity() - self.tx.len() >= b.len() {
            for c in b.bytes() {
                let _ = self.tx.push_back(c);
            }
        }
    }
    pub fn pump(&mut self, snapshot: &[u16]) {
        for _ in 0..512 {
            let queued = self.tx.front().copied();
            let b = queued.or_else(|| {
                if self.frame && self.at < WIDTH * HEIGHT * 2 {
                    Some(snapshot[self.at / 2].to_le_bytes()[self.at % 2])
                } else {
                    None
                }
            });
            let Some(b) = b else { break };
            if self.usb.write_byte_nb(b).is_err() {
                break;
            }
            if queued.is_some() {
                self.tx.pop_front();
            } else {
                self.at += 1;
                if self.at == WIDTH * HEIGHT * 2 {
                    self.frame = false;
                    self.queue("\nEND_FRAME\n");
                }
            }
        }
        let _ = self.usb.flush_tx_nb();
    }
    pub fn status(&mut self, a: &App, now: u32) {
        let mut s = String::<1800>::new();
        let mut menu_key = String::<96>::new();
        for c in a.menu_item().chars() {
            let _ = menu_key.push(if c.is_ascii_alphanumeric() { c } else { '_' });
        }
        let stats = a.training.stats().all;
        let _ = writeln!(
            s,
            "STARDUST {}-rust phase={:?} chips={} rounds={} shoe={} frame={} view={} focus={} brightness={} sound={} flipped={} ms={} pmic={} codec={} storage={} imported={} save_writes={} save_errors={} render_us={} push_us={} max_poll_ms={} audio_samples={} audio_underruns={} app_bytes={} device=waveshare175c display=466x466 orientation=270 touch=disabled language=Rust entropy=SAR-ADC-TRNG radios=off mode={:?} graded={} correct={} errors={} assisted={} seats={} player_seat={} wager_enabled={} wager_graded={} save_size={} unsaved={} resume_enabled={} resume_writes={} resume_errors={} resume_blocked={} resume_saved={} led_off={} volume={} audio_mask={} fast_mode={} amp_on={} amp_output={} amp_mode={} amp_drive={} amp_function={} amp_level={} background={} card_back={} menu_key={}",
            env!("STARDUST_CORE_VERSION"),
            a.game.phase,
            a.game.bankroll,
            a.game.rounds,
            a.game.shoe_number,
            self.frames,
            a.view_name(),
            a.focus(),
            a.brightness,
            a.sound_enabled,
            a.flipped,
            now,
            self.pmic,
            self.codec,
            self.save,
            self.imported,
            self.save_writes,
            self.save_errors,
            self.render_us,
            self.push_us,
            self.max_poll_ms,
            self.audio_samples,
            self.audio_underruns,
            core::mem::size_of::<App>(),
            a.save().mode,
            stats.decisions,
            stats.correct,
            stats.errors,
            stats.assisted,
            a.save().extra_seats,
            a.save().player_seat,
            a.save().wager_enabled,
            a.wager_practice.stats().all.decisions,
            stardust_blackjack::persistence::SIZE,
            self.unsaved,
            a.resume_enabled,
            self.resume_writes,
            self.resume_errors,
            self.resume_blocked,
            self.resume_saved,
            self.led_off,
            a.audio_volume,
            a.audio_mask,
            a.fast_mode,
            self.amp_on,
            self.amp_output,
            self.amp_mode,
            self.amp_drive,
            self.amp_function,
            self.amp_level,
            a.background,
            a.card_back,
            menu_key
        );
        self.ack_status = s.clone();
        self.queue(&s);
    }
    pub fn receive(
        &mut self,
        a: &App,
        now: u32,
        snapshot: &mut [u16],
        round_pixels: &[u16],
        allow_outputs: bool,
        safe_chooser: bool,
    ) -> Vec<Event, 32> {
        if self.last_poll != 0 {
            self.max_poll_ms = self.max_poll_ms.max(now.wrapping_sub(self.last_poll));
        }
        self.last_poll = now;
        let mut events = Vec::new();
        for _ in 0..32 {
            let Ok(c) = self.usb.read_byte() else { break };
            if self.frame {
                continue;
            }
            let event = match c {
                b'p' => {
                    self.benchmark_requested = true;
                    None
                }
                b's' | b'S' => {
                    if allow_outputs {
                        self.status(a, now)
                    } else if self.ack_status.is_empty() {
                        self.queue("STARDUST status=awaiting_committed_session\n")
                    } else {
                        let cache = self.ack_status.clone();
                        self.queue(&cache)
                    }
                    None
                }
                b'b' => Some(Event::Tap(Key::Blue)),
                b'g' => Some(Event::Tap(Key::Grey)),
                b'm' => Some(Event::Hold(Key::Grey)),
                b'h' => Some(Event::Hold(Key::Blue)),
                b'x' => Some(Event::SubmitCount),
                b'f' if allow_outputs || safe_chooser => {
                    snapshot.copy_from_slice(round_pixels);
                    self.queue("RGB565 466 466 434312\n");
                    self.at = 0;
                    self.frame = true;
                    None
                }
                _ => None,
            };
            if let Some(event) = event {
                let _ = events.push(event);
            }
        }
        self.pump(snapshot);
        events
    }
}
