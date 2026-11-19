//! Run with `rustc --edition=2024 --test power_registers.rs -o /tmp/stardust-power-test && /tmp/stardust-power-test`.
//! Uses the actual driver with a register-file I2C stand-in; no device access.
extern crate self as esp_hal;
pub struct Blocking;
pub mod delay {
    pub struct Delay;
    impl Delay {
        pub fn new() -> Self {
            Self
        }
        pub fn delay_millis(&self, _: u32) {}
    }
}
pub mod i2c {
    pub mod master {
        use std::{cell::RefCell, marker::PhantomData, rc::Rc};
        pub struct Registers {
            pub bytes: [u8; 256],
            pub fail: bool,
        }
        pub struct I2c<'a, T> {
            pub state: Rc<RefCell<Registers>>,
            pub marker: PhantomData<&'a T>,
        }
        impl<T> I2c<'_, T> {
            pub fn write_read(&mut self, _: u8, data: &[u8], out: &mut [u8]) -> Result<(), ()> {
                let state = self.state.borrow();
                if state.fail {
                    return Err(());
                }
                out[0] = state.bytes[usize::from(data[0])];
                Ok(())
            }
            pub fn write(&mut self, address: u8, data: &[u8]) -> Result<(), ()> {
                let mut state = self.state.borrow_mut();
                if state.fail {
                    return Err(());
                }
                if address == 0x6e {
                    state.bytes[usize::from(data[0])] = data[1];
                }
                Ok(())
            }
        }
    }
}
#[path = "../src/power.rs"]
mod power;
#[test]
fn amplifier_configures_gpio3_and_verifies_both_levels() {
    use i2c::master::{I2c, Registers};
    use std::{cell::RefCell, marker::PhantomData, rc::Rc};
    let state = Rc::new(RefCell::new(Registers {
        bytes: [0xff; 256],
        fail: false,
    }));
    let mut p = power::Power::new(I2c {
        state: state.clone(),
        marker: PhantomData,
    });
    assert!(p.pmic && p.codec && p.led_off && p.amp_output && !p.amp_on);
    {
        let bytes = &state.borrow().bytes;
        assert_eq!(
            bytes[0x10], 0xff,
            "GPIO3 becomes output; unrelated GPIO modes preserved"
        );
        assert_eq!(
            bytes[0x13], 0xf3,
            "GPIO2/3 push-pull; unrelated drive bits preserved"
        );
        assert_eq!(bytes[0x16], 0x0f, "GPIO2/3 two-bit mux fields cleared only");
        assert_eq!(bytes[0x11] & 8, 0, "amp inactive after boot");
    }
    p.amplifier(true);
    assert!(p.amp_on && p.amp_output);
    assert_eq!(state.borrow().bytes[0x11] & 8, 8);
    p.amplifier(false);
    assert!(!p.amp_on && p.amp_output);
    state.borrow_mut().fail = true;
    p.verify_amplifier();
    assert!(
        !p.amp_on && !p.amp_output,
        "failed readback must never claim enabled"
    );
    p.amplifier(true);
    assert!(!p.amp_on, "failed configuration must not claim enabled");
    state.borrow_mut().fail = false;
    p.amplifier(true);
    assert!(
        p.amp_on && p.amp_output,
        "successful retry enables amplifier"
    );
}
#[test]
fn amplifier_defaults_from_input_to_push_pull_output() {
    use i2c::master::{I2c, Registers};
    use std::{cell::RefCell, marker::PhantomData, rc::Rc};
    let state = Rc::new(RefCell::new(Registers {
        bytes: [0; 256],
        fail: false,
    }));
    let mut p = power::Power::new(I2c {
        state: state.clone(),
        marker: PhantomData,
    });
    assert!(p.amp_output && !p.amp_on);
    assert_eq!(
        state.borrow().bytes[0x10],
        0x0c,
        "both rail GPIO2 and amp GPIO3 are outputs"
    );
    p.amplifier(true);
    assert_eq!(state.borrow().bytes[0x11] & 0x0c, 0x0c);
    assert!(p.amp_on);
}
