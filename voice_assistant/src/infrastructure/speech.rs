//! Audio denoising pipeline (spectral subtraction via FFT).

use rustfft::{FftPlanner, num_complex::Complex};

/// Stationary noise reduction: treats the signal's own spectrum as the noise
/// profile and applies spectral subtraction (`prop_decrease` fraction removed).
pub fn denoise(samples: &[i16], _sample_rate: u32, prop_decrease: f32) -> Vec<i16> {
    spectral_subtract(samples, samples, prop_decrease)
}

/// Echo cancellation: subtract the reference signal's spectrum from the mic signal.
pub fn cancel_echo(mic: &[i16], reference: &[i16], prop_decrease: f32) -> Vec<i16> {
    spectral_subtract(mic, reference, prop_decrease)
}

// ── internals ─────────────────────────────────────────────────────────────────

fn spectral_subtract(signal: &[i16], noise_profile: &[i16], prop_decrease: f32) -> Vec<i16> {
    let n = signal.len();
    if n == 0 {
        return Vec::new();
    }

    let mut planner  = FftPlanner::<f32>::new();
    let fft          = planner.plan_fft_forward(n);
    let ifft         = planner.plan_fft_inverse(n);

    let mut sig_buf: Vec<Complex<f32>> = signal
        .iter()
        .map(|&s| Complex::new(s as f32, 0.0))
        .collect();

    let mut noise_buf: Vec<Complex<f32>> = noise_profile
        .iter()
        .take(n)
        .map(|&s| Complex::new(s as f32, 0.0))
        .collect();
    noise_buf.resize(n, Complex::new(0.0, 0.0));

    fft.process(&mut sig_buf);
    fft.process(&mut noise_buf);

    for (s, nb) in sig_buf.iter_mut().zip(noise_buf.iter()) {
        let sig_mag   = s.norm();
        let noise_mag = nb.norm();
        let new_mag   = (sig_mag - noise_mag * prop_decrease).max(0.0);
        let phase     = s.arg();
        *s = Complex::from_polar(new_mag, phase);
    }

    ifft.process(&mut sig_buf);

    let scale = 1.0 / n as f32;
    sig_buf
        .iter()
        .map(|c| (c.re * scale).clamp(-32_768.0, 32_767.0) as i16)
        .collect()
}
