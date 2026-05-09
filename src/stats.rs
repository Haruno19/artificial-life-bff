use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::{Write, stdout};

use rand::Rng;
use serde_json::json;

use crate::constants::*;

const OPCODES: [u8; 10] = [b'<', b'>', b'{', b'}', b'+', b'-', b'.', b',', b'[', b']'];

// Width of the progress bar in chars between [ and ].
// Kept comfortably narrower than the table so it fits on one terminal line
// (otherwise \r only resets the wrapped line and the bar duplicates).
const BAR_WIDTH: usize = 66;

// ----- metrics -----

pub fn entropy(tapes: &[[u8; TAPE_SIZE]]) -> f64 {
    let total = (tapes.len() * TAPE_SIZE) as f64;
    let mut counts = [0u64; 256];

    for tape in tapes {
        for &byte in tape {
            counts[byte as usize] += 1;
        }
    }

    counts
        .iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = c as f64 / total;
            -p * p.log2()
        })
        .sum()
}

pub fn opcode_frequency(tapes: &[[u8; TAPE_SIZE]]) -> f64 {
    let mut count: f64 = 0.0;
    for tape in tapes {
        for byte in tape {
            if OPCODES.contains(byte) {
                count += 1.0;
            }
        }
    }
    count / (TAPE_SIZE * tapes.len()) as f64
}

pub fn unique_count(tapes: &[[u8; TAPE_SIZE]]) -> usize {
    let unique: HashSet<&[u8; TAPE_SIZE]> = tapes.iter().collect();
    unique.len()
}

pub fn unique_code(tapes: &[[u8; TAPE_SIZE]]) -> usize {
    let unique: HashSet<[u8; TAPE_SIZE]> = tapes
        .iter()
        .map(|t| {
            let mut skel = [0u8; TAPE_SIZE];
            for (i, &b) in t.iter().enumerate() {
                if OPCODES.contains(&b) {
                    skel[i] = b;
                }
            }
            skel
        })
        .collect();
    unique.len()
}

pub fn sample_tapes(tapes: &[[u8; TAPE_SIZE]], n: usize) -> Vec<String> {
    let mut rng = rand::thread_rng();
    let mut picked = HashSet::new();
    let mut log = Vec::with_capacity(n);

    while picked.len() < n {
        picked.insert(rng.gen_range(0..tapes.len()));
    }

    for i in picked {
        log.push(
            tapes[i]
                .iter()
                .map(|&b| if OPCODES.contains(&b) { b as char } else { ' ' })
                .collect(),
        );
    }
    log
}

// ----- formatting helpers -----

fn format_row(epoch: usize, h: f64, op_freq: f64, unique: usize, unique_c: usize) -> String {
    format!(
        "│ {:>9} │ {:>8.4} │ {:>11.2}% │ {:>12} │ {:>11} │",
        epoch,
        h,
        op_freq * 100.0,
        unique,
        unique_c,
    )
}

fn make_progress_bar(current: usize, total: usize) -> String {
    let filled = (current * BAR_WIDTH) / total.max(1);
    let empty = BAR_WIDTH - filled;
    format!("[{}{}]", "=".repeat(filled), ".".repeat(empty))
}

// ----- public printing API -----

// Print the table header (one-time, at start).
pub fn print_header() {
    println!("┌───────────┬──────────┬──────────────┬──────────────┬─────────────┐");
    println!("│   epoch   │ entropy  │ opcode freq. │ unique tapes │ unique code │");
    println!("├───────────┼──────────┼──────────────┼──────────────┼─────────────┤");
}

// First-time stats print (epoch 0). Prints the row, two blank lines, and the
// initial empty progress bar.
pub fn init_print(tapes: &[[u8; TAPE_SIZE]], log_path: &str) {
    let h = entropy(tapes);
    let op_freq = opcode_frequency(tapes);
    let unique = unique_count(tapes);
    let unique_c = unique_code(tapes);

    println!("{}", format_row(0, h, op_freq, unique, unique_c));
    println!();
    println!();
    print!("{}", make_progress_bar(0, EVAL_STEPS));
    stdout().flush().unwrap();

    write_log(
        0,
        h,
        op_freq,
        unique,
        unique_c,
        sample_tapes(tapes, 4),
        log_path,
    );
}

// Periodic stats report. Moves cursor up to overwrite the progress bar area,
// prints the new row, then redraws the empty lines and a fresh progress bar.
pub fn report(tapes: &[[u8; TAPE_SIZE]], epoch: usize, log_path: &str) {
    let h = entropy(tapes);
    let op_freq = opcode_frequency(tapes);
    let unique = unique_count(tapes);
    let unique_c = unique_code(tapes);

    // \x1b[2A = up 2 lines, \r = column 0, \x1b[J = clear from cursor to end of screen
    // After this we're sitting on the first empty line below the table; print our row,
    // then 2 newlines, then a fresh progress bar.
    print!("\x1b[2A\r\x1b[J");
    println!("{}", format_row(epoch, h, op_freq, unique, unique_c));
    println!();
    println!();
    print!("{}", make_progress_bar(0, EVAL_STEPS));
    stdout().flush().unwrap();

    write_log(
        epoch,
        h,
        op_freq,
        unique,
        unique_c,
        sample_tapes(tapes, 4),
        log_path,
    );
}

// Update the progress bar in place, called every epoch.
pub fn update_progress(current: usize, total: usize) {
    print!("\r\x1b[K{}", make_progress_bar(current, total));
    stdout().flush().unwrap();
}

// Print a final closing line for the table when the run ends.
pub fn print_footer() {
    // wipe progress bar + 2 empty lines, then close the table
    print!("\x1b[2A\r\x1b[J");
    println!("└───────────┴──────────┴──────────────┴──────────────┴─────────────┘");
}

// ----- jsonl logging -----

fn write_log(
    epoch: usize,
    h: f64,
    op_freq: f64,
    unique: usize,
    unique_c: usize,
    samples: Vec<String>,
    log_path: &str,
) {
    let record = json!({
        "epoch":         epoch,
        "entropy":       h,
        "opcode_freq":   op_freq,
        "unique_tapes":  unique,
        "unique_code":   unique_c,
        "samples":       samples,
    });

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .expect("could not open log file");

    writeln!(file, "{}", record).expect("could not write to log file");
}
