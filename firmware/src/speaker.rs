//! Continuous I2S DMA, PCM mono duplicated into stereo; no allocator or busy wait.
use esp_hal::{
    Blocking,
    dma::DmaTxStreamBuf,
    dma_tx_stream_buffer,
    i2s::master::{I2sTx, I2sTxDmaTransfer},
};
use stardust_blackjack::{assets::Clip, audio::Cue};
pub struct Speaker {
    transfer: Option<I2sTxDmaTransfer<'static, Blocking, DmaTxStreamBuf>>,
    clip: Option<Clip>,
    at: usize,
    cue: Option<Cue>,
    volume_percent: u8,
    pub samples: u64,
    pub underruns: u32,
}
impl Speaker {
    pub fn new(tx: I2sTx<'static, Blocking>) -> Self {
        let mut b = dma_tx_stream_buffer!(8192, 1024);
        while b.push_with(|s| {
            s.fill(0);
            s.len()
        }) > 0
        {}
        let transfer = tx.write(b).unwrap_or_else(|_| panic!("I2S DMA"));
        Self {
            transfer: Some(transfer),
            clip: None,
            at: 0,
            cue: None,
            volume_percent: 50,
            samples: 0,
            underruns: 0,
        }
    }
    pub fn busy(&self) -> bool {
        self.clip.is_some()
    }
    pub fn set_volume(&mut self, percent: u8) {
        self.volume_percent = percent.min(100);
        if self.volume_percent == 0 {
            self.cancel();
        }
    }
    pub fn cue(&mut self, c: Cue) {
        self.clip = c.clip();
        self.cue = Some(c);
        self.at = 0;
    }
    pub fn cancel(&mut self) {
        if self.cue.is_some() {
            self.restart();
        }
        self.clip = None;
        self.cue = None;
        self.at = 0;
    }
    fn restart(&mut self) {
        let old = self.transfer.take().expect("audio transfer");
        let (tx, b) = old.stop();
        let (d, b) = b.split();
        let mut b = DmaTxStreamBuf::new(d, b).unwrap();
        b.push_with(|s| {
            s.fill(0);
            s.len()
        });
        self.transfer = Some(tx.write(b).unwrap_or_else(|_| panic!("I2S restart")));
    }
    pub fn interrupt(&mut self) {
        if self.cue.is_some_and(Cue::interrupts_on_input) {
            self.cancel();
        }
    }
    pub fn pump(&mut self, enabled: bool) {
        if !enabled || self.volume_percent == 0 {
            self.cancel()
        }
        if self.transfer.as_ref().is_some_and(|t| t.is_done()) {
            self.underruns = self.underruns.saturating_add(1);
            self.restart();
        }
        let Self {
            transfer,
            clip,
            at,
            samples,
            volume_percent,
            ..
        } = self;
        let transfer = transfer.as_mut().expect("audio transfer");
        // Bound one pass to the available DMA ring. The callback never allocates.
        for _ in 0..8 {
            let n = transfer.push_with(|out| {
                let len = out.len() / 4 * 4;
                for frame in out[..len].chunks_exact_mut(4) {
                    let v = if let Some(c) = *clip {
                        if let Some(s) = c.sample(*at) {
                            *at += 1;
                            *samples += 1;
                            stardust_blackjack::audio_gain::playback_sample(s, *volume_percent)
                        } else {
                            *clip = None;
                            0
                        }
                    } else {
                        0
                    };
                    let b = v.to_le_bytes();
                    frame.copy_from_slice(&[b[0], b[1], b[0], b[1]]);
                }
                len
            });
            if n == 0 {
                break;
            }
        }
    }
}
