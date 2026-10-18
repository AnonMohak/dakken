use std::time::Duration;

/// One key press that adds text (a letter or a space). Backspace is not a press.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Press {
    /// Time since the test started.
    pub at: Duration,
    pub correct: bool,
    /// The key the test asked for. `None` for a key that is not checked,
    /// such as an extra letter.
    pub key: Option<char>,
    /// Index of the word the press is in.
    pub word: usize,
}

/// Final character counts: correct, incorrect, extra and missed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CharCounts {
    pub correct: usize,
    pub incorrect: usize,
    pub extra: usize,
    pub missed: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestResult {
    pub wpm: f64,
    pub raw_wpm: f64,
    /// Percent of correct presses, 0 to 100.
    pub accuracy: f64,
    /// Percent, 0 to 100. Higher means a more even speed.
    pub consistency: f64,
    pub chars: CharCounts,
    pub duration: Duration,
    /// Raw WPM for each second of the test.
    pub raw_per_second: Vec<f64>,
    /// WPM from the start to the end of each second, from correct presses.
    pub wpm_over_time: Vec<f64>,
    /// The correct word with the lowest WPM, and that WPM.
    pub slowest_word: Option<(String, f64)>,
    /// The highest WPM of one correct word.
    pub fastest_burst: Option<f64>,
}

/// Words per minute, with one word = 5 characters.
pub fn wpm(chars: usize, duration: Duration) -> f64 {
    let minutes = duration.as_secs_f64() / 60.0;
    if minutes <= 0.0 {
        return 0.0;
    }
    chars as f64 / 5.0 / minutes
}

pub fn accuracy(presses: &[Press]) -> f64 {
    if presses.is_empty() {
        return 0.0;
    }
    let correct = presses.iter().filter(|p| p.correct).count();
    correct as f64 / presses.len() as f64 * 100.0
}

/// Raw WPM in each 1-second bucket. The last part-second counts only if it is
/// at least half a second long, and it is scaled up to a full second.
pub fn raw_per_second(presses: &[Press], duration: Duration) -> Vec<f64> {
    let total = duration.as_secs_f64();
    let full = total.floor() as usize;
    let rest = total - full as f64;
    let buckets = if rest >= 0.5 { full + 1 } else { full };

    let mut counts = vec![0usize; buckets];
    for p in presses {
        let i = p.at.as_secs() as usize;
        if i < buckets {
            counts[i] += 1;
        }
    }
    counts
        .iter()
        .enumerate()
        .map(|(i, &n)| {
            let len = if i == full { rest } else { 1.0 };
            n as f64 / len * 60.0 / 5.0
        })
        .collect()
}

/// WPM from the start of the test to the end of each 1-second bucket, from
/// correct presses. It has the same buckets as `raw_per_second`.
pub fn wpm_over_time(presses: &[Press], duration: Duration) -> Vec<f64> {
    let total = duration.as_secs_f64();
    let buckets = raw_per_second(presses, duration).len();
    (0..buckets)
        .map(|i| {
            let end = ((i + 1) as f64).min(total);
            let correct = presses
                .iter()
                .filter(|p| p.correct && p.at.as_secs_f64() < end)
                .count();
            wpm(correct, Duration::from_secs_f64(end))
        })
        .collect()
}

/// Consistency score: kogasa(stddev / mean) of raw WPM per second.
pub fn consistency(raw_per_second: &[f64]) -> f64 {
    if raw_per_second.is_empty() {
        return 0.0;
    }
    let n = raw_per_second.len() as f64;
    let mean = raw_per_second.iter().sum::<f64>() / n;
    if mean <= 0.0 {
        return 0.0;
    }
    let var = raw_per_second
        .iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>()
        / n;
    kogasa(var.sqrt() / mean)
}

fn kogasa(cov: f64) -> f64 {
    100.0 * (1.0 - (cov + cov.powi(3) / 3.0 + cov.powi(5) / 5.0).tanh())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(ms: u64, correct: bool) -> Press {
        Press {
            at: Duration::from_millis(ms),
            correct,
            key: None,
            word: 0,
        }
    }

    #[test]
    fn wpm_counts_five_chars_as_one_word() {
        assert_eq!(wpm(50, Duration::from_secs(60)), 10.0);
        assert_eq!(wpm(25, Duration::from_secs(30)), 10.0);
        assert_eq!(wpm(10, Duration::ZERO), 0.0);
    }

    #[test]
    fn accuracy_is_correct_over_all_presses() {
        let p = [
            press(0, true),
            press(1, true),
            press(2, true),
            press(3, false),
        ];
        assert_eq!(accuracy(&p), 75.0);
        assert_eq!(accuracy(&[]), 0.0);
    }

    #[test]
    fn even_speed_gives_full_consistency() {
        assert_eq!(consistency(&[60.0, 60.0, 60.0]), 100.0);
    }

    #[test]
    fn uneven_speed_gives_lower_consistency() {
        let c = consistency(&[30.0, 90.0, 30.0, 90.0]);
        assert!(c > 0.0 && c < 60.0, "got {c}");
    }

    #[test]
    fn raw_per_second_buckets_presses() {
        // 5 presses in second 0, 10 in second 1 -> 60 and 120 raw WPM.
        let mut p: Vec<Press> = (0..5).map(|i| press(i * 100, true)).collect();
        p.extend((0..10).map(|i| press(1000 + i * 50, false)));
        assert_eq!(
            raw_per_second(&p, Duration::from_secs(2)),
            vec![60.0, 120.0]
        );
    }

    #[test]
    fn wpm_over_time_counts_correct_presses_from_the_start() {
        // Second 0: 5 correct. Second 1: 5 correct and 5 wrong.
        let mut p: Vec<Press> = (0..5).map(|i| press(i * 100, true)).collect();
        p.extend((0..10).map(|i| press(1000 + i * 50, i % 2 == 0)));
        assert_eq!(wpm_over_time(&p, Duration::from_secs(2)), vec![60.0, 60.0]);
        // The last part-second ends at the end of the test.
        assert_eq!(
            wpm_over_time(&p[..5], Duration::from_millis(1500)),
            vec![60.0, 40.0]
        );
    }

    #[test]
    fn raw_per_second_scales_or_drops_the_last_part_second() {
        let p = [press(1100, true)];
        // 0.5 s part counts double: 1 press in 0.5 s = 2/s = 24 WPM.
        assert_eq!(
            raw_per_second(&p, Duration::from_millis(1500)),
            vec![0.0, 24.0]
        );
        // 0.2 s part is dropped.
        assert_eq!(raw_per_second(&p, Duration::from_millis(1200)), vec![0.0]);
    }
}
