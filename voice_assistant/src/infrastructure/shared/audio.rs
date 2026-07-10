//! Shared audio utilities: sample conversion, resampling, denoising,
//! and echo cancellation.

use rustfft::{FftPlanner, num_complex::Complex};

// ── Sample conversion ──────────────────────────────────────────────────────

pub fn bytes_to_i16(bytes: &[u8]) -> Vec<i16> {
    bytes
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]))
        .collect()
}

pub fn i16_to_bytes(samples: &[i16]) -> Vec<u8> {
    samples.iter().flat_map(|s| s.to_le_bytes()).collect()
}

/// Linear resampling via interpolation.
pub fn resample(samples: &[i16], from_rate: u32, to_rate: u32) -> Vec<i16> {
    let n_out = (samples.len() as u64 * to_rate as u64 / from_rate as u64) as usize;
    (0..n_out)
        .map(|i| {
            let src = i as f64 * from_rate as f64 / to_rate as f64;
            let lo  = src.floor() as usize;
            let hi  = (lo + 1).min(samples.len().saturating_sub(1));
            let t   = src.fract() as f32;
            (samples[lo] as f32 * (1.0 - t) + samples[hi] as f32 * t) as i16
        })
        .collect()
}

/// RMS amplitude of an i16 signal, normalized to 0.0–1.0.
pub fn rms_amplitude(samples: &[i16]) -> f64 {
    if samples.is_empty() { return 0.0; }
    let sum_sq: f64 = samples.iter().map(|&s| (s as f64).powi(2)).sum();
    (sum_sq / samples.len() as f64).sqrt() / 32768.0
}

// ── Denoising / echo cancellation ──────────────────────────────────────────

/// Stationary noise reduction: treats the signal's own spectrum as the noise
/// profile and applies spectral subtraction (`prop_decrease` fraction removed).
pub fn denoise(samples: &[i16], _sample_rate: u32, prop_decrease: f32) -> Vec<i16> {
    spectral_subtract(samples, samples, prop_decrease)
}

/// Echo cancellation: subtract the reference signal's spectrum from the mic signal.
pub fn cancel_echo(mic: &[i16], reference: &[i16], prop_decrease: f32) -> Vec<i16> {
    spectral_subtract(mic, reference, prop_decrease)
}

// ── internals ──────────────────────────────────────────────────────────────

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
