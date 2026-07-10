// ── AudioCapture ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct AudioCapture {
    pub raw:          Vec<u8>,
    pub sample_rate:  u32,
    pub sample_width: u16,
}

impl AudioCapture {
    pub fn new(raw: Vec<u8>, sample_rate: u32, sample_width: u16) -> Self {
        Self { raw, sample_rate, sample_width }
    }
}
