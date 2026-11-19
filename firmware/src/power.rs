//! Minimum StickS3 PM1 changes. Preserve charging and physical power-key configuration.
use esp_hal::{Blocking, delay::Delay, i2c::master::I2c};
pub struct Power {
    bus: I2c<'static, Blocking>,
    pub pmic: bool,
    pub codec: bool,
    pub led_off: bool,
    pub led_config: u8,
    pub led_drive: u8,
    pub amp_on: bool,
    pub amp_output: bool,
    pub amp_mode: u8,
    pub amp_drive: u8,
    pub amp_function: u8,
    pub amp_level: u8,
}
impl Power {
    pub fn new(bus: I2c<'static, Blocking>) -> Self {
        let mut p = Self {
            bus,
            pmic: false,
            codec: false,
            led_off: false,
            led_config: 0xff,
            led_drive: 0xff,
            amp_on: false,
            amp_output: false,
            amp_mode: 0xff,
            amp_drive: 0xff,
            amp_function: 0xff,
            amp_level: 0xff,
        };
        let mut id = [0; 1];
        p.pmic = p.bus.write_read(0x6eu8, &[0], &mut id).is_ok();
        if p.pmic {
            // LED1 is driven by Q3: LED_EN high is ON, low is OFF.
            // Apply this first, and verify it rather than ignoring a startup NACK.
            for _ in 0..3 {
                p.keep_status_led_off();
                if p.led_off {
                    break;
                }
                Delay::new().delay_millis(2);
            }
            let _ = p.bus.write(0x6eu8, &[0x09, 0]);
            for (reg, clear, set) in [
                (0x16, 0x30, 0),
                (0x10, 0, 4),
                (0x13, 4, 0),
                (0x11, 0, 4),
                (0x49, 1, 0),
                (0x11, 1, 0),
            ] {
                p.update(reg, clear, set);
            }
            // StickS3 amplifier EN is PM1 GPIO3, distinct from GPIO2/L3B.
            // Set the inactive latch before selecting push-pull output mode.
            for _ in 0..3 {
                if p.configure_amplifier() {
                    break;
                }
                Delay::new().delay_millis(2);
            }
        }
        let mut ok = true;
        for (reg, val) in [
            (0x00, 0x80),
            (0x01, 0xb5),
            (0x02, 0x18),
            (0x0d, 0x01),
            (0x12, 0x00),
            (0x13, 0x10),
            (0x32, 0xbf),
            (0x37, 0x08),
        ] {
            ok &= p.bus.write(0x18u8, &[reg, val]).is_ok();
        }
        p.codec = ok;
        p
    }
    pub fn keep_status_led_off(&mut self) {
        // PM1 owns reset/download indication. Reassert the inactive level during
        // normal game operation without changing charging or power-rail bits.
        let written = self.pmic && self.update(0x06, 0x10, 0);
        let mut cfg = [0xff];
        let read = self.bus.write_read(0x6eu8, &[0x06], &mut cfg).is_ok();
        self.led_config = cfg[0];
        self.led_off = written && read && cfg[0] & 0x10 == 0;
        let mut drive = [0xff];
        if self.bus.write_read(0x6eu8, &[0x13], &mut drive).is_ok() {
            self.led_drive = drive[0];
        }
    }
    fn update(&mut self, reg: u8, clear: u8, set: u8) -> bool {
        let mut b = [0];
        if self.bus.write_read(0x6eu8, &[reg], &mut b).is_err() {
            return false;
        }
        self.bus
            .write(0x6eu8, &[reg, (b[0] & !clear) | set])
            .is_ok()
    }
    pub fn amplifier(&mut self, on: bool) {
        if !self.pmic {
            self.amp_on = false;
            self.amp_output = false;
            return;
        }
        let on = on && self.pmic && self.codec;
        if !self.amp_output && !self.configure_amplifier() {
            return;
        }
        if on != self.amp_on {
            self.update(0x11, 8, if on { 8 } else { 0 });
            self.verify_amplifier();
        }
    }
    fn configure_amplifier(&mut self) -> bool {
        // GPIO_FUNC0 contains two bits per pin: GPIO3 occupies bits7:6.
        // Official M5Unified uses default amplifier mode: EN high, no pulses.
        let ok = self.update(0x11, 8, 0)
            && self.update(0x16, 0xc0, 0)
            && self.update(0x13, 8, 0)
            && self.update(0x10, 0, 8);
        self.verify_amplifier();
        ok && self.amp_output && !self.amp_on
    }
    pub fn verify_amplifier(&mut self) {
        let mut values = [0xff; 4];
        let mut ok = self.pmic;
        for (i, reg) in [0x10, 0x13, 0x16, 0x11].into_iter().enumerate() {
            ok &= self
                .bus
                .write_read(0x6eu8, &[reg], &mut values[i..i + 1])
                .is_ok();
        }
        self.amp_mode = values[0];
        self.amp_drive = values[1];
        self.amp_function = values[2];
        self.amp_level = values[3];
        self.amp_output = ok && values[0] & 8 != 0 && values[1] & 8 == 0 && values[2] & 0xc0 == 0;
        self.amp_on = self.amp_output && values[3] & 8 != 0;
    }
}
