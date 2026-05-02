use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use prismatic_core::{SensoryEvent, GlobalContext};
use rtrb::RingBuffer;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use prismatic_core::temporal::LockFreeEventBus;

fn q_rsqrt(number: f32) -> f32 {
    let mut y = number;
    let mut i = y.to_bits();
    i = 0x5f3759df - (i >> 1);
    y = f32::from_bits(i);
    y * (1.5 - (number * 0.5 * y * y))
}

fn q_abs(x: f32) -> f32 {
    f32::from_bits(x.to_bits() & 0x7FFFFFFF)
}

fn q_sin(x: f32) -> f32 {
    let pi = std::f32::consts::PI;
    let period = 2.0 * pi;
    let mut x_mod = x - period * (x / period).floor();
    if x_mod > pi {
        x_mod -= period;
    }
    let mut sin_x = (4.0 / pi) * x_mod - (4.0 / (pi * pi)) * x_mod * q_abs(x_mod);
    sin_x = 0.225 * (sin_x * q_abs(sin_x) - sin_x) + sin_x;
    sin_x
}

fn q_cos(x: f32) -> f32 {
    q_sin(x + 1.57079632679)
}
/// Bi-directional Acoustic Sensory Organ
pub fn run_cpal_gradient_loop(bus: Arc<LockFreeEventBus>, state: Arc<GlobalContext>) {
    loop {
        if prismatic_core::SHUTDOWN.load(Ordering::Relaxed) { break; }
        tracing::info!("Attempting to bind bi-directional acoustic sensory organ...");
        let tx_clone = bus.clone();
        let state_clone = state.clone();

        let result = try_open_cpal_stream(tx_clone, state_clone);

        match result {
            Ok(_) => tracing::warn!("Acoustic sensory stream ended organically."),
            Err(e) => tracing::warn!("Failed to bind acoustic sensory: {}. Retrying in 5s.", e),
        }
        std::thread::sleep(Duration::from_secs(5));
    }
}

fn try_open_cpal_stream(bus: Arc<LockFreeEventBus>, state: Arc<GlobalContext>) -> anyhow::Result<()> {
    let host = cpal::default_host();
    let mic = host
        .default_input_device()
        .ok_or_else(|| anyhow::anyhow!("No microphone found"))?;
    let speaker = host
        .default_output_device()
        .ok_or_else(|| anyhow::anyhow!("No speaker found"))?;

    let in_config = mic.default_input_config()?;
    let out_config = speaker.default_output_config()?;

    let (mut producer, mut consumer) = RingBuffer::<f32>::new(4096);

    let err_fn = |err| tracing::error!("Cpal fault: {}", err);

    // 1. Microphone Input (The Cochlea)
    let in_config_clone = in_config.clone();
    let in_stream = match in_config.sample_format() {
        cpal::SampleFormat::F32 => mic.build_input_stream(
            &in_config_clone.into(),
            move |data: &[f32], _: &_| {
                data.iter().for_each(|&sample| {
                    let _ = producer.push(sample);
                });
            },
            err_fn,
            None,
        )?,
        _ => anyhow::bail!("Unsupported microphone sample format"),
    };

    // 2. Speaker Output (The Vocal Cords)
    let mut sample_clock = 0f32;
    let out_config_clone = out_config.clone();
    let sample_rate = out_config.sample_rate().0 as f32;
    
    // For Dynamic Biquad IIR Filter state
    let mut iir_z1 = 0.0;
    let mut iir_z2 = 0.0;

    let out_stream = match out_config.sample_format() {
        cpal::SampleFormat::F32 => speaker.build_output_stream(
            &out_config_clone.into(),
            move |data: &mut [f32], _: &_| {
                let target_hz = f32::from_bits(state.audio_oscillator_hz.load(Ordering::Relaxed));
                let gpu_thermal_celsius = f32::from_bits(state.gpu_thermal_celsius.load(Ordering::Relaxed));
                
                // Dynamic Biquad IIR Filter Coefficients (Low-Pass Filter)
                // Cutoff frequency drops as gpu_thermal_celsius rises to absorb thermal Re-entry spikes
                let cutoff = 2000.0 - (gpu_thermal_celsius * 10.0).clamp(0.0, 1900.0);
                let w0 = 2.0 * std::f32::consts::PI * cutoff / sample_rate;
                let alpha = q_sin(w0) / (2.0 * 0.707); // Q = 0.707 (Butterworth resonance)
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * q_cos(w0) / a0;
                let a2 = (1.0 - alpha) / a0;
                let b0 = ((1.0 - q_cos(w0)) / 2.0) / a0;
                let b1 = (1.0 - q_cos(w0)) / a0;
                let b2 = ((1.0 - q_cos(w0)) / 2.0) / a0;

                data.iter_mut().enumerate().for_each(|(_sample_idx, sample)| {
                    sample_clock = (sample_clock + 1.0) % sample_rate;
                    let t = sample_clock / sample_rate;

                    // SIMD Chebyshev Polynomial Exciter (Bosonic String Synthesis)
                    // Instead of loading wavetables from memory, compute Chebyshev T_n(x) natively
                    let x_phase = q_sin(t * target_hz * 2.0 * std::f32::consts::PI);
                    
                    // T_1(x) = x
                    // T_2(x) = 2x^2 - 1
                    // T_3(x) = 4x^3 - 3x
                    let t2 = 2.0 * x_phase * x_phase - 1.0;
                    let t3 = 4.0 * x_phase * x_phase * x_phase - 3.0 * x_phase;
                    
                    // Harmonic Blending
                    let mut drone = x_phase * 0.5 + t2 * 0.3 + t3 * 0.2;
                    
                    // Apply organic LFO breathing (4-second pulse)
                    let lfo = (q_sin(t * 0.25 * 2.0 * std::f32::consts::PI) * 0.5) + 0.5;
                    drone = drone * lfo * 0.05; // Base amplitude 5%
                    
                    // Apply Dynamic Biquad IIR Filter to absorb thermal spikes
                    let out = b0 * drone + b1 * iir_z1 + b2 * iir_z2 - a1 * iir_z1 - a2 * iir_z2;
                    iir_z2 = iir_z1;
                    iir_z1 = out;
                    
                    *sample = out;
                });
            },
            err_fn,
            None,
        )?,
        _ => anyhow::bail!("Unsupported speaker sample format"),
    };

    in_stream.play()?;
    out_stream.play()?;

    let mut empty_cycles = 0;
    loop {
        if prismatic_core::SHUTDOWN.load(Ordering::Relaxed) { break; }
        if let Ok(chunk) = consumer.read_chunk(1024) {
            empty_cycles = 0;
            let (slice1, slice2) = chunk.as_slices();

            // Hardware SIMD Dot Product (AVX2-256)
            #[inline(always)]
            fn simd_sum_sq(slice: &[f32]) -> f32 {
                let mut sum = 0.0;
                let mut i = 0;
                #[cfg(target_arch = "x86_64")]
                {
                    if std::is_x86_feature_detected!("avx2") {
                        unsafe {
                            use core::arch::x86_64::*;
                            let mut sum_vec = _mm256_setzero_ps();
                            while i + 8 <= slice.len() {
                                let data_vec = _mm256_loadu_ps(slice.as_ptr().add(i));
                                let squared = _mm256_mul_ps(data_vec, data_vec);
                                sum_vec = _mm256_add_ps(sum_vec, squared);
                                i += 8;
                            }
                            let mut sum_arr = [0.0f32; 8];
                            _mm256_storeu_ps(sum_arr.as_mut_ptr(), sum_vec);
                            sum += sum_arr.iter().sum::<f32>();
                        }
                    }
                }
                // Scalar Fallback / Safety Net Remainder
                while i < slice.len() {
                    sum += slice[i] * slice[i];
                    i += 1;
                }
                sum
            }
            
            let sum_sq: f32 = simd_sum_sq(slice1) + simd_sum_sq(slice2);
            let mean_sq = sum_sq / 1024.0;
            let rms = if mean_sq > 0.0 { mean_sq * q_rsqrt(mean_sq) } else { 0.0 };

            chunk.commit_all();

            bus.push(SensoryEvent::AudioAmplitude(rms));
        } else {
            if consumer.is_abandoned() {
                anyhow::bail!("Audio stream producer was abandoned.");
            }
            empty_cycles += 1;
            if empty_cycles > 200 {
                anyhow::bail!("Audio stream timed out.");
            }
            thread::sleep(Duration::from_millis(5));
        }
    }

    Ok(())
}
