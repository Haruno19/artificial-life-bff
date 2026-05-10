use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::{Write, stdout};

use rand::Rng;
use rayon::prelude::*;
use serde_json::json;

use crate::constants::*;

const OPCODES: [u8; 10] = [b'<', b'>', b'{', b'}', b'+', b'-', b'.', b',', b'[', b']'];

// 256-entry lookup table: IS_OPCODE[b] == true iff b is one of the 10 BFF opcodes.
// Avoids a linear scan over OPCODES on every byte in the soup.
const IS_OPCODE: [bool; 256] = {
    let mut t = [false; 256];
    let mut i = 0;
    while i < OPCODES.len() {
        t[OPCODES[i] as usize] = true;
        i += 1;
    }
    t
};

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
            if IS_OPCODE[*byte as usize] {
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
                if IS_OPCODE[b as usize] {
                    skel[i] = b;
                }
            }
            skel
        })
        .collect();
    unique.len()
}

// ----- replicator detection -----
//
// "Compress" a tape down to its opcode/zero skeleton: keep only bytes that are
// one of the 10 BFF opcodes or the literal '0' character (matching the sample
// renderer). Everything else (the random non-code bytes that make up most of
// each tape) is dropped. The result is a much shorter byte sequence that
// captures the program's structure.
fn compress_tape(tape: &[u8; TAPE_SIZE]) -> Vec<u8> {
    let mut out = Vec::with_capacity(TAPE_SIZE);
    for &b in tape {
        if IS_OPCODE[b as usize] || b == b'0' {
            out.push(b);
        }
    }
    out
}

// Replicator family classification.
//   A  — body contains {`}`, `<`, `,`} (h0 is the copy destination)
//   B  — body contains {`>`, `{`, `.`} (h1 is the copy destination)
//   AB — body contains both trigrams (hybrid: copies in both directions)
//   None — not a replicator
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum Family {
    None,
    A,
    B,
    AB,
}

impl Family {
    fn as_str(self) -> &'static str {
        match self {
            Family::A => "A",
            Family::B => "B",
            Family::AB => "AB",
            Family::None => "",
        }
    }
}

fn classify_body(body: &[u8]) -> Family {
    let (mut h1r, mut h0l, mut c10) = (false, false, false);
    let (mut h0r, mut h1l, mut c01) = (false, false, false);

    for &b in body {
        match b {
            b'}' => h1r = true,
            b'<' => h0l = true,
            b',' => c10 = true,
            b'>' => h0r = true,
            b'{' => h1l = true,
            b'.' => c01 = true,
            _ => {}
        }
    }

    let a = h1r && h0l && c10;
    let b = h0r && h1l && c01;
    match (a, b) {
        (true, true) => Family::AB,
        (true, false) => Family::A,
        (false, true) => Family::B,
        (false, false) => Family::None,
    }
}

// Find every well-formed [...] in the compressed tape whose body qualifies as
// a replicator. Brackets are matched left-to-right with a stack, so nested
// loops are paired correctly; we report each matching pair independently
// (an outer loop and a qualifying inner loop both get counted).
//
// Each loop is returned as (family, pattern) where pattern includes its [ and ].
fn find_replicator_loops(compressed: &[u8]) -> Vec<(Family, String)> {
    let mut stack: Vec<usize> = Vec::new();
    let mut loops: Vec<(Family, String)> = Vec::new();

    for (i, &b) in compressed.iter().enumerate() {
        if b == b'[' {
            stack.push(i);
        } else if b == b']' {
            if let Some(open) = stack.pop() {
                let body = &compressed[open + 1..i];
                let family = classify_body(body);
                if family != Family::None {
                    // safe: compressed only contains opcode bytes + '0', all ASCII
                    let s = std::str::from_utf8(&compressed[open..=i])
                        .expect("compressed tape should be ASCII")
                        .to_string();
                    loops.push((family, s));
                }
            }
        }
    }

    loops
}

// One bucket per family in the catalogue output.
struct FamilyBucket {
    total: u64,
    unique: usize,
    patterns: Vec<(String, u64)>,
}

// Walk the soup, count occurrences of each distinct replicator pattern, grouped
// by family. Parallelised with rayon: per-thread (family, pattern) HashMap,
// merged at the end and split into family buckets.
fn catalogue_replicators(
    tapes: &[[u8; TAPE_SIZE]],
) -> HashMap<&'static str, FamilyBucket> {
    let counts: HashMap<(Family, String), u64> = tapes
        .par_iter()
        .fold(HashMap::<(Family, String), u64>::new, |mut acc, tape| {
            let compressed = compress_tape(tape);
            for (family, pattern) in find_replicator_loops(&compressed) {
                *acc.entry((family, pattern)).or_insert(0) += 1;
            }
            acc
        })
        .reduce(HashMap::<(Family, String), u64>::new, |mut a, b| {
            for (k, v) in b {
                *a.entry(k).or_insert(0) += v;
            }
            a
        });

    // bucket by family
    let mut buckets: HashMap<&'static str, FamilyBucket> = HashMap::new();
    for ((family, pattern), count) in counts {
        let entry = buckets.entry(family.as_str()).or_insert(FamilyBucket {
            total: 0,
            unique: 0,
            patterns: Vec::new(),
        });
        entry.total += count;
        entry.unique += 1;
        entry.patterns.push((pattern, count));
    }

    // sort each bucket's patterns: descending count, ties by lexicographic key
    for bucket in buckets.values_mut() {
        bucket
            .patterns
            .sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    }

    buckets
}

fn sample_tapes(tapes: &[[u8; TAPE_SIZE]]) -> Vec<String> {
    let mut rng = rand::thread_rng();
    let mut picked = HashSet::new();
    let mut log = Vec::with_capacity(SAMPLE_COUNT);

    while picked.len() < SAMPLE_COUNT {
        picked.insert(rng.gen_range(0..tapes.len()));
    }

    for i in picked {
        log.push(
            tapes[i]
                .iter()
                .map(|&b| if IS_OPCODE[b as usize] || b == b'0' { b as char } else { ' ' })
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
pub fn init_print(tapes: &[[u8; TAPE_SIZE]], log_path: &str, samples_path: &str) {
    let h = entropy(tapes);
    let op_freq = opcode_frequency(tapes);
    let unique = unique_count(tapes);
    let unique_c = unique_code(tapes);

    println!("{}", format_row(0, h, op_freq, unique, unique_c));
    println!();
    println!();
    print!("{}", make_progress_bar(0, EVAL_STEPS));
    stdout().flush().unwrap();

    write_log(0, h, op_freq, unique, unique_c, log_path);
    write_samples(tapes, 0, samples_path);
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

    write_log(epoch, h, op_freq, unique, unique_c, log_path);
}

// Update the progress bar in place, called every epoch.
pub fn update_progress(current: usize, total: usize) {
    let prev_filled = (current.saturating_sub(1) * BAR_WIDTH) / total.max(1);
    let filled = (current * BAR_WIDTH) / total.max(1);
    if filled != prev_filled {
        print!("\r\x1b[K{}", make_progress_bar(current, total));
        stdout().flush().unwrap();
    }
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
    log_path: &str,
) {
    let record = json!({
        "epoch":        epoch,
        "entropy":      h,
        "opcode_freq":  op_freq,
        "unique_tapes": unique,
        "unique_code":  unique_c,
    });

    append_jsonl(log_path, &record);
}

pub fn write_samples(tapes: &[[u8; TAPE_SIZE]], epoch: usize, samples_path: &str) {
    let record = json!({
        "epoch":   epoch,
        "samples": sample_tapes(tapes),
    });

    append_jsonl(samples_path, &record);
}

pub fn write_replicators(tapes: &[[u8; TAPE_SIZE]], epoch: usize, repl_path: &str) {
    let buckets = catalogue_replicators(tapes);

    let total_all: u64 = buckets.values().map(|b| b.total).sum();
    let unique_all: usize = buckets.values().map(|b| b.unique).sum();

    // build the per-family object — only include keys that actually have entries
    let mut families = serde_json::Map::new();
    for key in ["A", "B", "AB"] {
        if let Some(bucket) = buckets.get(key) {
            let patterns: Vec<serde_json::Value> = bucket
                .patterns
                .iter()
                .map(|(s, c)| json!([s, c]))
                .collect();
            families.insert(
                key.to_string(),
                json!({
                    "total":    bucket.total,
                    "unique":   bucket.unique,
                    "patterns": patterns,
                }),
            );
        }
    }

    let record = json!({
        "epoch":              epoch,
        "total_replicators":  total_all,
        "unique_replicators": unique_all,
        "replicators":        families,
    });

    append_jsonl(repl_path, &record);
}

fn append_jsonl(path: &str, record: &serde_json::Value) {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("could not open log file");

    writeln!(file, "{}", record).expect("could not write to log file");
}
