use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;

use rand::Rng;
use serde_json::json;

use crate::constants::*;

const OPCODES: [u8; 10] = [b'<', b'>', b'{', b'}', b'+', b'-', b'.', b',', b'[', b']'];

// Compute Shannon entropy over all byte values across all tapes.
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

// Compute what fraction of bytes across all tapes are valid opcodes.
// A rising value signals that programs are writing opcodes into the soup.
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

// Count how many distinct (unique) tapes exist in the soup.
// Starts near the soup size (all random = all unique).
// A sharp drop signals replicators are copying themselves everywhere.
pub fn unique_count(tapes: &[[u8; TAPE_SIZE]]) -> usize {
    let unique: HashSet<&[u8; TAPE_SIZE]> = tapes.iter().collect();
    unique.len()
}

// Count distinct "code skeletons" — tapes compared on opcode bytes only,
// with all non-opcode bytes replaced by 0. Catches families of similar
// replicators that the strict unique_count misses because their data
// bytes drift.
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

// Sample `n` random tapes and render each as a string.
// Opcode bytes are shown as their ASCII character, everything else as '.'.
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
                .map(|&b| {
                    if OPCODES.contains(&b) {
                        b as char
                    } else {
                        '　'
                    }
                })
                .collect(),
        );
    }
    return log;
}

// Print a stats report to the terminal and append a JSON line to log.jsonl.
// Samples are only written to the file, not printed.
pub fn report(tapes: &[[u8; TAPE_SIZE]], epoch: usize, log_path: &str) {
    let h = entropy(tapes);
    let op_freq = opcode_frequency(tapes);
    let unique = unique_count(tapes);
    let unique_c = unique_code(tapes);
    let samples = sample_tapes(tapes, 4);

    // print summary to terminal (no samples)
    println!("--- epoch {} ---", epoch);
    println!("  entropy:       {:.4}", h);
    println!("  opcode freq:   {:.2}%", op_freq * 100.0);
    println!("  unique tapes:  {}", unique);
    println!("  unique code:   {}", unique_c);

    // append one JSON line to the log file
    let record = json!({
        "epoch":         epoch,
        "entropy":       h,
        "opcode_freq":   op_freq,
        "unique_tapes":  unique,
        "unique_code":   unique_c,
        "samples":       samples,
    });

    let mut file = OpenOptions::new()
        .create(true) // create if it doesn't exist
        .append(true) // never overwrite, always add to the end
        .open(log_path)
        .expect("could not open log file");

    writeln!(file, "{}", record).expect("could not write to log file");
}
