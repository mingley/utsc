use std::hint::black_box;
use std::time::{Duration, Instant};

use utsc::Scanner;

fn scan(source: &str) -> usize {
    Scanner::new(black_box(source))
        .map(|token| token.end.wrapping_add(token.kind as usize))
        .fold(0, usize::wrapping_add)
}

fn main() {
    let cases = [
        ("ascii", include_str!("../tests/fixtures/representative.ts")),
        ("unicode", include_str!("../tests/fixtures/unicode.ts")),
    ];
    for (name, sample) in cases {
        let source = sample.repeat(4096);
        let tokens = Scanner::new(&source).count();
        let checksum = scan(&source);
        let mut warmup = Scanner::new(&source);
        warmup.by_ref().for_each(|token| {
            black_box(token);
        });
        assert!(warmup.diagnostics().is_empty());
        let deadline = Duration::from_millis(500);
        let mut samples = Vec::new();
        for _ in 0..7 {
            let start = Instant::now();
            let mut iterations = 0;
            while start.elapsed() < deadline {
                black_box(scan(&source));
                iterations += 1;
            }
            samples.push(
                source.len() as f64 * iterations as f64
                    / start.elapsed().as_secs_f64()
                    / 1_048_576.0,
            );
        }
        samples.sort_by(f64::total_cmp);
        println!(
            "{name}: {:.1} MiB/s median (range {:.1}–{:.1}); {} bytes, {} tokens, checksum={checksum}, 7 × ≥500 ms; scanner only",
            samples[3],
            samples[0],
            samples[6],
            source.len(),
            tokens
        );
    }
}
