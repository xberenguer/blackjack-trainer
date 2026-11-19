//! Waveshare1.75C audio only. Preserve AXP2101 rails, charging and power-key policy.
use esp_hal::{Blocking, delay::Delay, gpio::Output, i2c::master::I2c};
pub struct Power {
    bus: I2c<'static, Blocking>,
    amp: Output<'static>,
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
    pub fn new(bus: I2c<'static, Blocking>, mut amp: Output<'static>) -> Self {
        amp.set_low();
        let mut p = Self {
            bus,
            amp,
            pmic: false,
            codec: false,
            led_off: false,
            led_config: 0xff,
            led_drive: 0xff,
            amp_on: false,
            amp_output: true,
            amp_mode: 0,
            amp_drive: 0,
            amp_function: 0,
            amp_level: 0,
        };
        let mut id = [0u8];
        p.pmic = p.bus.write_read(0x34u8, &[3], &mut id).is_ok();
        // No touch bus address is probed. Only put the unused IMU/microphones to sleep.
        if p.bus.write_read(0x6bu8, &[0], &mut id).is_ok() && id[0] == 5 {
            for (reg, clear, set) in [(8, 3, 0), (2, 0, 2)] {
                if p.bus.write_read(0x6bu8, &[reg], &mut id).is_ok() {
                    let _ = p.bus.write(0x6bu8, &[reg, (id[0] & !clear) | set]);
                }
            }
        }
        if p.bus.write_read(0x40u8, &[1], &mut id).is_ok() {
            for reg in 0x47..=0x4c {
                let _ = p.bus.write(0x40u8, &[reg, 255]);
            }
            let _ = p.bus.write(0x40u8, &[1, 0x7f]);
            let _ = p.bus.write(0x40u8, &[6, 7]);
        }
        // Official ES8311 slave Philips16bit / MCLK256fs coefficient set for22050Hz.
        let mut ok = true;
        for (reg, value) in [(0, 0x1f), (0, 0), (0, 0x80)] {
            ok &= p.bus.write(0x18u8, &[reg, value]).is_ok();
            Delay::new().delay_millis(5);
        }
        for (reg, value) in [
            (1, 0x3f),
            (2, 0),
            (3, 0x10),
            (4, 0x10),
            (5, 0),
            (6, 3),
            (7, 0),
            (8, 0xff),
            (9, 0x0c),
            (10, 0x0c),
            (0x0d, 1),
            (0x0e, 2),
            (0x12, 0),
            (0x13, 0x10),
            (0x1c, 0x6a),
            (0x37, 8),
            (0x32, 0xff),
        ] {
            ok &= p.bus.write(0x18u8, &[reg, value]).is_ok();
        }
        // Master mode disabled; software PCM scaling supplies the requested volume.
        ok &= p.bus.write(0x18u8, &[0, 0x80]).is_ok();
        p.codec = ok;
        p
    }
    pub fn amplifier(&mut self, on: bool) {
        let on = on && self.codec;
        if on {
            self.amp.set_high()
        } else {
            self.amp.set_low()
        }
        self.amp_on = on;
        self.amp_level = u8::from(on);
    }
    pub fn keep_status_led_off(&mut self) {} // No M5PM1 on this board; never write its addresses.
    pub fn verify_amplifier(&mut self) {
        self.amp_level = u8::from(self.amp_on);
    }
}
