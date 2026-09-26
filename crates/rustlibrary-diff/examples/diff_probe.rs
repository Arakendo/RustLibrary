use rustlibrary_diff::{Limits, diff_lines};
use std::{hint::black_box, time::Instant};

fn main() {
    for (name, count, repeated) in [
        ("small", 100, false),
        ("larger", 900, false),
        ("repeated", 900, true),
        ("over_cell_budget", 1500, false),
    ] {
        let old: String = (0..count)
            .map(|i| {
                if repeated {
                    "same line\n".into()
                } else {
                    format!("line {i:04}\n")
                }
            })
            .collect();
        let mut new = old.clone();
        new.replace_range(0..10, "changed!\n");
        let mut times = Vec::new();
        let mut outcome = String::new();
        for _ in 0..21 {
            let start = Instant::now();
            let result = diff_lines(black_box(&old), black_box(&new), Limits::default());
            times.push(start.elapsed().as_micros());
            outcome = match result {
                Ok(changes) => format!("{} changes", changes.len()),
                Err(error) => format!("{error:?}"),
            };
        }
        let first = times.remove(0);
        times.sort_unstable();
        println!(
            "{name}: old_lines={count}, input_bytes={}, first_us={first}, median20_us={}, outcome={outcome}",
            old.len() + new.len(),
            times[10]
        );
    }
}
