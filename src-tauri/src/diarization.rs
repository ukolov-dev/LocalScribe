use crate::types::{TranscriptSegment, TranscriptionResult};

const SAMPLE_RATE_HZ: usize = 16_000;
const FEATURE_COUNT: usize = 9;
const MIN_SEGMENT_SAMPLES: usize = SAMPLE_RATE_HZ / 3;
const MAX_ANALYSIS_SAMPLES: usize = SAMPLE_RATE_HZ * 6 / 5;
const FEATURE_WEIGHTS: [f32; FEATURE_COUNT] = [0.45, 0.9, 1.6, 0.6, 1.0, 1.0, 1.0, 1.0, 1.0];

pub fn assign_speakers(result: &mut TranscriptionResult, samples: &[f32]) {
    let mut extracted = Vec::new();

    for (segment_position, segment) in result.segments.iter().enumerate() {
        if segment.text.trim().is_empty() {
            continue;
        }

        if let Some(feature) = extract_segment_feature(samples, segment) {
            extracted.push((segment_position, feature));
        }
    }

    if extracted.is_empty() {
        return;
    }

    if extracted.len() < 3 {
        assign_single_speaker(result);
        return;
    }

    let mut feature_values = extracted
        .iter()
        .map(|(_, feature)| *feature)
        .collect::<Vec<_>>();
    normalize_features(&mut feature_values);

    let mut labels = cluster_two_speakers(&feature_values);
    smooth_isolated_labels(&mut labels, &extracted, &result.segments);

    let mut segment_speakers = vec![None; result.segments.len()];
    let mut cluster_speaker_ids = [0usize; 2];
    let mut next_speaker_id = 1usize;

    for ((segment_position, _), label) in extracted.iter().zip(labels.iter()) {
        let label = *label;
        if cluster_speaker_ids[label] == 0 {
            cluster_speaker_ids[label] = next_speaker_id;
            next_speaker_id += 1;
        }
        segment_speakers[*segment_position] = Some(cluster_speaker_ids[label]);
    }

    fill_missing_speakers(&mut segment_speakers);

    for (segment, speaker_id) in result.segments.iter_mut().zip(segment_speakers.iter()) {
        if !segment.text.trim().is_empty() {
            segment.speaker = speaker_id.map(|id| format!("Speaker {id}"));
        }
    }
}

fn assign_single_speaker(result: &mut TranscriptionResult) {
    for segment in result.segments.iter_mut() {
        if !segment.text.trim().is_empty() {
            segment.speaker = Some("Speaker 1".to_string());
        }
    }
}

fn extract_segment_feature(
    samples: &[f32],
    segment: &TranscriptSegment,
) -> Option<[f32; FEATURE_COUNT]> {
    let start = ms_to_sample_index(segment.start_ms, samples.len());
    let end = ms_to_sample_index(segment.end_ms, samples.len());
    if end <= start || end - start < MIN_SEGMENT_SAMPLES {
        return None;
    }

    let segment_samples = &samples[start..end];
    let analysis_samples = centered_window(segment_samples, MAX_ANALYSIS_SAMPLES);
    let mean = analysis_samples.iter().sum::<f32>() / analysis_samples.len() as f32;
    let centered = analysis_samples
        .iter()
        .map(|sample| sample - mean)
        .collect::<Vec<_>>();

    let rms = root_mean_square(&centered);
    if rms < 0.0008 {
        return None;
    }

    let zero_crossing_rate = zero_crossing_rate(&centered);
    let (pitch_hz, pitch_confidence) = pitch_feature(&centered);
    let bands = relative_band_powers(&centered);

    Some([
        safe_log10(rms),
        zero_crossing_rate,
        pitch_hz / 300.0,
        pitch_confidence,
        bands[0],
        bands[1],
        bands[2],
        bands[3],
        bands[4],
    ])
}

fn centered_window(samples: &[f32], max_len: usize) -> &[f32] {
    if samples.len() <= max_len {
        return samples;
    }

    let start = (samples.len() - max_len) / 2;
    &samples[start..start + max_len]
}

fn root_mean_square(samples: &[f32]) -> f32 {
    (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32).sqrt()
}

fn zero_crossing_rate(samples: &[f32]) -> f32 {
    if samples.len() < 2 {
        return 0.0;
    }

    let crossings = samples
        .windows(2)
        .filter(|pair| (pair[0] >= 0.0 && pair[1] < 0.0) || (pair[0] < 0.0 && pair[1] >= 0.0))
        .count();
    crossings as f32 / (samples.len() - 1) as f32
}

fn pitch_feature(samples: &[f32]) -> (f32, f32) {
    let max_len = (SAMPLE_RATE_HZ * 6 / 5).min(samples.len());
    if max_len < SAMPLE_RATE_HZ / 4 {
        return (0.0, 0.0);
    }

    let samples = centered_window(samples, max_len);
    let min_lag = SAMPLE_RATE_HZ / 320;
    let max_lag = SAMPLE_RATE_HZ / 70;
    let mut best_lag = 0usize;
    let mut best_correlation = 0.0f32;

    for lag in (min_lag..=max_lag).step_by(2) {
        let mut correlation = 0.0f32;
        let mut energy_a = 0.0f32;
        let mut energy_b = 0.0f32;

        let mut index = 0usize;
        while index + lag < samples.len() {
            let left = samples[index];
            let right = samples[index + lag];
            correlation += left * right;
            energy_a += left * left;
            energy_b += right * right;
            index += 8;
        }

        if energy_a <= f32::EPSILON || energy_b <= f32::EPSILON {
            continue;
        }

        let normalized = correlation / (energy_a * energy_b).sqrt();
        if normalized > best_correlation {
            best_correlation = normalized;
            best_lag = lag;
        }
    }

    if best_lag == 0 || best_correlation < 0.18 {
        return (0.0, best_correlation.max(0.0));
    }

    (SAMPLE_RATE_HZ as f32 / best_lag as f32, best_correlation)
}

fn relative_band_powers(samples: &[f32]) -> [f32; 5] {
    let frequencies = [250.0, 500.0, 1_000.0, 2_000.0, 3_500.0];
    let mut powers = [0.0f32; 5];

    for (index, frequency) in frequencies.iter().enumerate() {
        powers[index] = goertzel_power(samples, *frequency);
    }

    let total = powers.iter().sum::<f32>().max(1.0e-9);
    for power in powers.iter_mut() {
        *power = safe_log10(*power / total);
    }
    powers
}

fn goertzel_power(samples: &[f32], frequency_hz: f32) -> f32 {
    let omega = 2.0 * std::f32::consts::PI * frequency_hz / SAMPLE_RATE_HZ as f32;
    let coefficient = 2.0 * omega.cos();
    let mut q1 = 0.0f32;
    let mut q2 = 0.0f32;

    for sample in samples {
        let q0 = coefficient * q1 - q2 + *sample;
        q2 = q1;
        q1 = q0;
    }

    (q1 * q1 + q2 * q2 - coefficient * q1 * q2) / samples.len() as f32
}

fn safe_log10(value: f32) -> f32 {
    value.max(1.0e-9).log10()
}

fn normalize_features(features: &mut [[f32; FEATURE_COUNT]]) {
    let mut means = [0.0f32; FEATURE_COUNT];
    for feature in features.iter() {
        for index in 0..FEATURE_COUNT {
            means[index] += feature[index];
        }
    }
    for mean in means.iter_mut() {
        *mean /= features.len() as f32;
    }

    let mut std_devs = [0.0f32; FEATURE_COUNT];
    for feature in features.iter() {
        for index in 0..FEATURE_COUNT {
            let delta = feature[index] - means[index];
            std_devs[index] += delta * delta;
        }
    }
    for std_dev in std_devs.iter_mut() {
        *std_dev = (*std_dev / features.len() as f32).sqrt().max(1.0e-5);
    }

    for feature in features.iter_mut() {
        for index in 0..FEATURE_COUNT {
            feature[index] = ((feature[index] - means[index]) / std_devs[index]).clamp(-4.0, 4.0)
                * FEATURE_WEIGHTS[index];
        }
    }
}

fn cluster_two_speakers(features: &[[f32; FEATURE_COUNT]]) -> Vec<usize> {
    let (left_index, right_index) = farthest_feature_pair(features);
    let mut centroids = [features[left_index], features[right_index]];
    let mut labels = vec![0usize; features.len()];

    for _ in 0..24 {
        let mut changed = false;
        for (index, feature) in features.iter().enumerate() {
            let next_label = if squared_distance(feature, &centroids[0])
                <= squared_distance(feature, &centroids[1])
            {
                0
            } else {
                1
            };

            if labels[index] != next_label {
                labels[index] = next_label;
                changed = true;
            }
        }

        let mut sums = [[0.0f32; FEATURE_COUNT]; 2];
        let mut counts = [0usize; 2];
        for (feature, label) in features.iter().zip(labels.iter()) {
            counts[*label] += 1;
            for dimension in 0..FEATURE_COUNT {
                sums[*label][dimension] += feature[dimension];
            }
        }

        if counts[0] == 0 || counts[1] == 0 {
            return vec![0; features.len()];
        }

        for label in 0..2 {
            for dimension in 0..FEATURE_COUNT {
                centroids[label][dimension] = sums[label][dimension] / counts[label] as f32;
            }
        }

        if !changed {
            break;
        }
    }

    labels
}

fn farthest_feature_pair(features: &[[f32; FEATURE_COUNT]]) -> (usize, usize) {
    let mut best = (0usize, 1usize);
    let mut best_distance = -1.0f32;

    for left in 0..features.len() {
        for right in left + 1..features.len() {
            let distance = squared_distance(&features[left], &features[right]);
            if distance > best_distance {
                best_distance = distance;
                best = (left, right);
            }
        }
    }

    best
}

fn squared_distance(left: &[f32; FEATURE_COUNT], right: &[f32; FEATURE_COUNT]) -> f32 {
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| {
            let delta = *left - *right;
            delta * delta
        })
        .sum()
}

fn smooth_isolated_labels(
    labels: &mut [usize],
    extracted: &[(usize, [f32; FEATURE_COUNT])],
    segments: &[TranscriptSegment],
) {
    if labels.len() < 3 {
        return;
    }

    for index in 1..labels.len() - 1 {
        if labels[index - 1] != labels[index + 1] || labels[index] == labels[index - 1] {
            continue;
        }

        let segment = &segments[extracted[index].0];
        if segment.end_ms.saturating_sub(segment.start_ms) <= 500 {
            labels[index] = labels[index - 1];
        }
    }
}

fn fill_missing_speakers(segment_speakers: &mut [Option<usize>]) {
    let mut previous = None;
    for speaker in segment_speakers.iter_mut() {
        if speaker.is_none() {
            *speaker = previous;
        } else {
            previous = *speaker;
        }
    }

    let first = segment_speakers.iter().find_map(|speaker| *speaker);
    if let Some(first) = first {
        for speaker in segment_speakers.iter_mut() {
            if speaker.is_some() {
                break;
            }
            *speaker = Some(first);
        }
    }
}

fn ms_to_sample_index(ms: u64, sample_count: usize) -> usize {
    ((ms as usize).saturating_mul(SAMPLE_RATE_HZ) / 1000).min(sample_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assigns_stable_labels_for_alternating_synthetic_voices() {
        let segment_len = SAMPLE_RATE_HZ;
        let gap_len = SAMPLE_RATE_HZ / 5;
        let mut samples = Vec::new();
        for frequency in [120.0, 220.0, 120.0, 220.0] {
            for index in 0..segment_len {
                let phase =
                    2.0 * std::f32::consts::PI * frequency * index as f32 / SAMPLE_RATE_HZ as f32;
                samples.push(phase.sin() * 0.25);
            }
            samples.extend(std::iter::repeat(0.0).take(gap_len));
        }

        let mut result = TranscriptionResult {
            job_id: "test".to_string(),
            text: String::new(),
            segments: (0..4)
                .map(|index| TranscriptSegment {
                    index,
                    start_ms: index as u64 * 1_200,
                    end_ms: index as u64 * 1_200 + 1_000,
                    text: format!("segment {index}"),
                    speaker: None,
                })
                .collect(),
            benchmark: None,
        };

        assign_speakers(&mut result, &samples);

        assert_eq!(result.segments[0].speaker, result.segments[2].speaker);
        assert_eq!(result.segments[1].speaker, result.segments[3].speaker);
        assert_ne!(result.segments[0].speaker, result.segments[1].speaker);
    }
}
