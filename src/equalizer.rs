//! Nine octave-spaced peaking filters, processed by the decoder worker.
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

pub const FREQUENCIES: [f64; 9] = [
    62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];
pub const LABELS: [&str; 9] = ["62", "125", "250", "500", "1k", "2k", "4k", "8k", "16k"];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct EqualizerSettings {
    pub enabled: bool,
    pub gains: [i8; 9],
}

impl EqualizerSettings {
    pub fn normalized(mut self) -> Self {
        self.gains
            .iter_mut()
            .for_each(|gain| *gain = (*gain).clamp(-12, 12));
        self
    }
}

#[derive(Clone, Copy)]
struct Coefficients {
    b: [f64; 3],
    a: [f64; 2],
}

impl Coefficients {
    const IDENTITY: Self = Self {
        b: [1.0, 0.0, 0.0],
        a: [0.0; 2],
    };

    fn peak(frequency: f64, gain: f64, rate: f64) -> Self {
        // Frequencies at/above Nyquist have no representable band.
        if frequency >= rate / 2.0 || gain == 0.0 {
            return Self::IDENTITY;
        }
        let amplitude = 10.0_f64.powf(gain / 40.0);
        let omega = 2.0 * PI * frequency / rate;
        // Q = sqrt(2): approximately one octave between the half-gain points.
        let alpha = omega.sin() / (2.0 * 2.0_f64.sqrt());
        let a0 = 1.0 + alpha / amplitude;
        Self {
            b: [
                (1.0 + alpha * amplitude) / a0,
                -2.0 * omega.cos() / a0,
                (1.0 - alpha * amplitude) / a0,
            ],
            a: [-2.0 * omega.cos() / a0, (1.0 - alpha / amplitude) / a0],
        }
    }

    fn magnitude(self, omega: f64) -> f64 {
        let power = |values: [f64; 3]| {
            let real = values[0] + values[1] * omega.cos() + values[2] * (2.0 * omega).cos();
            let imaginary = values[1] * omega.sin() + values[2] * (2.0 * omega).sin();
            real * real + imaginary * imaginary
        };
        (power(self.b) / power([1.0, self.a[0], self.a[1]])).sqrt()
    }
}

pub(crate) struct Processor {
    rate: f64,
    channels: usize,
    settings: Option<EqualizerSettings>,
    coefficients: [Coefficients; 9],
    target_coefficients: [Coefficients; 9],
    ramp_frames: usize,
    target_preamp: f64,
    // Independent delay lines for every channel, including multichannel files.
    states: Vec<[[f64; 2]; 9]>,
    wet: f64,
    target_wet: f64,
    preamp: f64,
}

impl Processor {
    pub fn new(rate: u32, channels: usize) -> Self {
        Self {
            rate: f64::from(rate),
            channels,
            settings: None,
            coefficients: [Coefficients::IDENTITY; 9],
            target_coefficients: [Coefficients::IDENTITY; 9],
            ramp_frames: 0,
            target_preamp: 1.0,
            states: vec![[[0.0; 2]; 9]; channels],
            wet: 0.0,
            target_wet: 0.0,
            preamp: 1.0,
        }
    }

    pub fn process(&mut self, samples: &mut [f32], settings: EqualizerSettings) {
        let settings = settings.normalized();
        if self.settings != Some(settings) {
            self.target_coefficients = std::array::from_fn(|index| {
                Coefficients::peak(
                    FREQUENCIES[index],
                    f64::from(settings.gains[index]),
                    self.rate,
                )
            });
            // Measure the combined response, including the exact band centers.
            // This avoids multiplying nine independent headroom allowances.
            let peak = (0..=1024)
                .map(|index| PI * index as f64 / 1024.0)
                .chain(
                    FREQUENCIES
                        .map(|frequency| 2.0 * PI * frequency / self.rate)
                        .into_iter()
                        .filter(|omega| *omega < PI),
                )
                .map(|omega| {
                    self.target_coefficients
                        .iter()
                        .map(|filter| filter.magnitude(omega))
                        .product::<f64>()
                })
                .fold(1.0_f64, f64::max);
            self.target_preamp = if peak > 1.0001 {
                1.0 / (peak * 10.0_f64.powf(0.5 / 20.0))
            } else {
                1.0
            };
            self.target_wet = if settings.enabled && settings.gains != [0; 9] {
                1.0
            } else {
                0.0
            };
            self.ramp_frames = (self.rate * 0.01) as usize;
            self.settings = Some(settings);
        }
        if self.wet == 0.0 && self.target_wet == 0.0 {
            self.states.fill([[0.0; 2]; 9]);
            return;
        }
        // Crossfade bypass over 10ms. Filter histories continue through bypass fades.
        let step = 1.0 / (self.rate * 0.01);
        for frame in samples.chunks_exact_mut(self.channels) {
            if self.ramp_frames > 0 {
                let remaining = self.ramp_frames as f64;
                for (current, target) in self.coefficients.iter_mut().zip(self.target_coefficients)
                {
                    for (value, target) in current
                        .b
                        .iter_mut()
                        .zip(target.b)
                        .chain(current.a.iter_mut().zip(target.a))
                    {
                        *value += (target - *value) / remaining;
                    }
                }
                self.preamp += (self.target_preamp - self.preamp) / remaining;
                self.ramp_frames -= 1;
            }
            self.wet += (self.target_wet - self.wet).clamp(-step, step);
            for (sample, state) in frame.iter_mut().zip(&mut self.states) {
                let dry = f64::from(*sample);
                let mut wet = dry * self.preamp;
                for (coefficients, delay) in self.coefficients.iter().zip(state) {
                    let output = coefficients.b[0] * wet + delay[0];
                    delay[0] = coefficients.b[1] * wet - coefficients.a[0] * output + delay[1];
                    delay[1] = coefficients.b[2] * wet - coefficients.a[1] * output;
                    wet = output;
                }
                *sample = (dry + self.wet * (wet - dry)).clamp(-1.0, 1.0) as f32;
            }
        }
    }
}

#[cfg(test)]
mod tests;
