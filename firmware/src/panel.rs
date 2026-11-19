//! StickS3 ST7789V2. Wire RGB565 big endian; logical framebuffer remains host endian.
use esp_hal::{Blocking, delay::Delay, gpio::Output, spi::master::Spi};
use stardust_blackjack::canvas::{HEIGHT, WIDTH};
pub struct Panel {
    spi: Spi<'static, Blocking>,
    dc: Output<'static>,
    cs: Output<'static>,
    flipped: bool,
}
impl Panel {
    pub fn new(
        spi: Spi<'static, Blocking>,
        dc: Output<'static>,
        cs: Output<'static>,
        mut reset: Output<'static>,
    ) -> Self {
        let d = Delay::new();
        reset.set_low();
        d.delay_millis(10);
        reset.set_high();
        d.delay_millis(120);
        let mut s = Self {
            spi,
            dc,
            cs,
            flipped: false,
        };
        s.cmd(0x01, &[]);
        d.delay_millis(150);
        s.cmd(0x11, &[]);
        d.delay_millis(120);
        // Datasheet/M5 ST7789 initialization: 16-bit RGB, inversion, normal mode.
        s.cmd(0xb7, &[0x35]);
        s.cmd(0xbb, &[0x28]);
        s.cmd(0xc0, &[0x0c]);
        s.cmd(0xc2, &[0x01, 0xff]);
        s.cmd(0xc3, &[0x10]);
        s.cmd(0xc4, &[0x20]);
        s.cmd(0xd0, &[0xa4, 0xa1]);
        s.cmd(0xb0, &[0x00, 0xc0]);
        s.cmd(
            0xe0,
            &[
                0xd0, 0x00, 0x02, 0x07, 0x0a, 0x28, 0x32, 0x44, 0x42, 0x06, 0x0e, 0x12, 0x14, 0x17,
            ],
        );
        s.cmd(
            0xe1,
            &[
                0xd0, 0x00, 0x02, 0x07, 0x0a, 0x28, 0x31, 0x54, 0x47, 0x0e, 0x1c, 0x17, 0x1b, 0x1e,
            ],
        );
        s.cmd(0x3a, &[0x55]);
        s.cmd(0x21, &[]);
        s.rotation(false);
        s.cmd(0x13, &[]);
        s.cmd(0x29, &[]);
        d.delay_millis(20);
        s
    }
    fn cmd(&mut self, c: u8, b: &[u8]) {
        self.cs.set_low();
        self.dc.set_low();
        self.spi.write(&[c]).expect("ST7789 SPI transfer");
        if !b.is_empty() {
            self.dc.set_high();
            self.spi.write(b).expect("ST7789 SPI transfer");
        }
        self.cs.set_high();
    }
    fn rotation(&mut self, flipped: bool) {
        self.flipped = flipped;
        self.cmd(0x36, &[if flipped { 0xb8 } else { 0x6c }]);
    }
    pub fn present(
        &mut self,
        pixels: &[u16; WIDTH * HEIGHT],
        flipped: bool,
        mut service: impl FnMut(),
    ) {
        if flipped != self.flipped {
            self.rotation(flipped)
        }
        // Memory offsets rotate with MADCTL. 240 rows begin at40; narrow axis at52/53.
        let y = if flipped { 52u16 } else { 53u16 };
        self.cmd(0x2a, &[0, 40, 1, 23]);
        let end = y + HEIGHT as u16 - 1;
        self.cmd(0x2b, &[0, y as u8, (end >> 8) as u8, end as u8]);
        self.cs.set_low();
        self.dc.set_low();
        self.spi.write(&[0x2c]).expect("ST7789 SPI transfer");
        self.dc.set_high();
        let mut row = [0u8; WIDTH * 2];
        for r in pixels.chunks_exact(WIDTH) {
            for (p, b) in r.iter().zip(row.chunks_exact_mut(2)) {
                b.copy_from_slice(&p.to_be_bytes());
            }
            self.spi.write(&row).expect("ST7789 SPI transfer");
            service();
        }
        self.cs.set_high();
    }
}
