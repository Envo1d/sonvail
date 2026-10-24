use super::*;

fn tone(frequency: f64, rate: u32) -> Vec<f32> {
    (0..rate)
        .map(|index| {
            (0.1 * (2.0 * PI * frequency * f64::from(index) / f64::from(rate)).sin()) as f32
        })
        .collect()
}

fn rms(samples: &[f32]) -> f64 {
    (samples
        .iter()
        .map(|value| f64::from(*value).powi(2))
        .sum::<f64>()
        / samples.len() as f64)
        .sqrt()
}

#[test]
fn bypass_and_flat_are_bit_exact() {
    for settings in [
        EqualizerSettings {
            enabled: false,
            gains: [12; 9],
        },
        EqualizerSettings {
            enabled: true,
            gains: [0; 9],
        },
    ] {
        let original = tone(1000.0, 48000);
        let mut samples = original.clone();
        Processor::new(48000, 1).process(&mut samples, settings);
        assert_eq!(samples, original);
    }
}

#[test]
fn every_band_has_the_requested_center_gain() {
    for frequency in FREQUENCIES {
        for gain in [-12.0, 12.0] {
            let filter = Coefficients::peak(frequency, gain, 48000.0);
            let db = 20.0 * filter.magnitude(2.0 * PI * frequency / 48000.0).log10();
            assert!((db - gain).abs() < 0.0001, "{frequency} Hz: {db} dB");
        }
    }
}

#[test]
fn cuts_affect_the_selected_band_and_channels_remain_independent() {
    let input = tone(1000.0, 48000);
    let mut samples: Vec<_> = input.iter().flat_map(|sample| [*sample, 0.0]).collect();
    let mut settings = EqualizerSettings {
        enabled: true,
        gains: [0; 9],
    };
    settings.gains[4] = -12;
    Processor::new(48000, 2).process(&mut samples, settings);
    let left: Vec<_> = samples
        .as_chunks::<2>()
        .0
        .iter()
        .skip(2400)
        .map(|frame| frame[0])
        .collect();
    assert!(
        samples
            .as_chunks::<2>()
            .0
            .iter()
            .all(|frame| frame[1] == 0.0)
    );
    let db = 20.0 * (rms(&left) / rms(&input[2400..])).log10();
    assert!((db + 12.0).abs() < 0.05, "{db} dB");
}

#[test]
fn low_sample_rates_and_extreme_live_changes_stay_finite_and_bounded() {
    for rate in [8000, 22050, 44100, 48000, 192000] {
        let mut processor = Processor::new(rate, 1);
        for gains in [
            [12; 9],
            [-12; 9],
            [12, -12, 12, -12, 12, -12, 12, -12, 12],
            [0; 9],
        ] {
            let mut samples = tone(500.0, rate);
            processor.process(
                &mut samples,
                EqualizerSettings {
                    enabled: true,
                    gains,
                },
            );
            assert!(
                samples
                    .iter()
                    .all(|sample| sample.is_finite() && sample.abs() <= 1.0)
            );
        }
        assert_eq!(
            Coefficients::peak(f64::from(rate), 12.0, f64::from(rate)).b,
            Coefficients::IDENTITY.b
        );
    }
}

#[test]
fn boosting_creates_headroom_and_bypass_restores_original_level() {
    let mut processor = Processor::new(48000, 1);
    let original = tone(1000.0, 48000);
    let mut boosted = original.clone();
    processor.process(
        &mut boosted,
        EqualizerSettings {
            enabled: true,
            gains: [12; 9],
        },
    );
    assert!(processor.preamp < 1.0);
    assert!(rms(&boosted[2400..]) <= rms(&original[2400..]));
    let mut bypassed = original.clone();
    processor.process(&mut bypassed, EqualizerSettings::default());
    assert_eq!(bypassed[2400..], original[2400..]);
}
