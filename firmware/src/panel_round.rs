//! CO5300 QSPI driver. Native panel orientation, software CW90, RGB565 big-endian.
use crate::round_panel_layout::{PIXELS, W, wire_pixel};
use esp_hal::{
    Blocking,
    delay::Delay,
    dma::DmaTxBuf,
    gpio::Output,
    spi::master::{Address, Command, DataMode, SpiDma},
};
const ROWS: usize = 8;
const DMA_BYTES: usize = 8192;
pub struct Panel {
    spi: Option<SpiDma<'static, Blocking>>,
    pixels: Option<DmaTxBuf>,
    on: bool,
    brightness: u8,
}
impl Panel {
    pub fn new(
        spi: SpiDma<'static, Blocking>,
        pixels: DmaTxBuf,
        reset: &mut Output<'static>,
    ) -> Self {
        let mut p = Self {
            spi: Some(spi),
            pixels: Some(pixels),
            on: false,
            brightness: 0,
        };
        let d = Delay::new();
        reset.set_high();
        d.delay_millis(10);
        reset.set_low();
        d.delay_millis(200);
        reset.set_high();
        d.delay_millis(200);
        p.reg(0x11, &[]);
        d.delay_millis(120);
        for (reg, value) in [
            (0xfe, 0),
            (0xc4, 0x80),
            (0x3a, 0x55),
            (0x53, 0x20),
            (0x63, 0xff),
            (0x51, 0),
            (0x58, 0),
            (0x36, 0),
        ] {
            p.reg(reg, &[value]);
        }
        p.reg(0x20, &[]);
        p.reg(0x28, &[]);
        p
    }
    fn reg(&mut self, reg: u8, data: &[u8]) {
        self.spi
            .as_mut()
            .unwrap()
            .half_duplex_write(
                DataMode::SingleTwoDataLines,
                Command::_8Bit(0x02, DataMode::SingleTwoDataLines),
                Address::_24Bit(u32::from(reg) << 8, DataMode::SingleTwoDataLines),
                0,
                data,
            )
            .expect("CO5300 register write");
    }
    pub fn set_brightness(&mut self, value: u8) {
        self.brightness = value;
        if self.on {
            self.reg(0x51, &[value]);
        }
    }
    pub fn present(&mut self, fb: &[u16; PIXELS], flipped: bool, mut poll: impl FnMut()) {
        for y in (0..W).step_by(ROWS) {
            let h = ROWS.min(W - y);
            let xa = 6u16;
            let xb = (W + 5) as u16;
            let ya = y as u16;
            let yb = (y + h - 1) as u16;
            self.reg(
                0x2a,
                &[(xa >> 8) as u8, xa as u8, (xb >> 8) as u8, xb as u8],
            );
            self.reg(
                0x2b,
                &[(ya >> 8) as u8, ya as u8, (yb >> 8) as u8, yb as u8],
            );
            self.reg(0x2c, &[]);
            let mut buf = self.pixels.take().unwrap();
            buf.set_length(DMA_BYTES);
            let b = buf.as_mut_slice();
            let mut n = 0;
            for yy in y..y + h {
                for x in 0..W {
                    b[n..n + 2].copy_from_slice(&wire_pixel(fb, x, yy, flipped));
                    n += 2;
                }
                poll();
            }
            buf.set_length(n);
            let spi = self.spi.take().unwrap();
            let transfer = spi
                .half_duplex_write_buffer(
                    DataMode::Quad,
                    Command::_8Bit(0x32, DataMode::SingleTwoDataLines),
                    Address::_24Bit(0x003c00, DataMode::SingleTwoDataLines),
                    0,
                    n,
                    buf,
                )
                .unwrap_or_else(|(e, _, _)| panic!("CO5300 DMA {e:?}"));
            while !transfer.is_done() {
                poll();
            }
            let (spi, buf) = transfer.wait();
            self.spi = Some(spi);
            self.pixels = Some(buf);
            poll();
        }
        if !self.on {
            self.reg(0x29, &[]);
            self.on = true;
            self.reg(0x51, &[self.brightness]);
        }
    }
}
