use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;

use crate::constants::*;
use crate::interpreter;
use crate::stats;

pub struct Soup {
    tapes: Vec<[u8; TAPE_SIZE]>, // all the tapes
    rng: SmallRng,               // persistent RNG, from the rand crate
    epoch_count: usize,          // how many epochs have run so far
}

impl Soup {
    pub fn new(size: usize) -> Self {
        let mut rng = SmallRng::from_entropy();
        let mut tapes = Vec::with_capacity(size);

        for _ in 0..size {
            let mut tape = [0u8; TAPE_SIZE];
            rng.fill(&mut tape);
            tapes.push(tape);
        }

        Self {
            epoch_count: 0,
            rng,
            tapes,
        }
    }


    fn epoch(&mut self) {
        self.tapes.shuffle(&mut self.rng);
        self.tapes.par_chunks_mut(2).for_each(|pair| {
            interact_pair(pair);
        });

        if MUTATION_RATE > 0.0 {
            self.mutate_soup();
        }
    }

    // Apply soup-wide background mutation.
    fn mutate_soup(&mut self) {
        let total_bytes = (self.tapes.len() * TAPE_SIZE) as f64;
        let n_mutations = (total_bytes * MUTATION_RATE).round() as usize;

        let mut rng = rand::thread_rng();
        for _ in 0..n_mutations {
            let tape_idx = rng.gen_range(0..self.tapes.len());
            let byte_idx = rng.gen_range(0..TAPE_SIZE);
            self.tapes[tape_idx][byte_idx] = rng.gen_range(0u8..=255);
        }
    }

    pub fn run(&mut self, log_path: &str) {
        println!("=== run parameters ===");
        println!("  soup size:     {}", SOUP_SIZE);
        println!("  tape size:     {}", TAPE_SIZE);
        println!("  max steps:     {}", MAX_STEPS);
        println!("  epochs:        {}", EPOCHS);
        println!("  eval every:    {}", EVAL_STEPS);
        println!("  mutation rate: {}", MUTATION_RATE);
        println!("  log file:      {}", log_path);
        println!();

        // table header + initial (epoch 0) row + progress bar
        stats::print_header();
        stats::init_print(&self.tapes, log_path);

        // Track the last rendered bar fill so we only redraw on visual change.
        let mut last_filled: usize = 0;

        for _ in 0..EPOCHS {
            self.epoch();
            self.epoch_count += 1;

            if self.epoch_count % EVAL_STEPS == 0 {
                stats::report(&self.tapes, self.epoch_count, log_path);
                last_filled = 0; // fresh bar after a report
            } else {
                let progress = self.epoch_count % EVAL_STEPS;
                let filled = (progress * BAR_WIDTH) / EVAL_STEPS;
                if filled != last_filled {
                    stats::update_progress(progress, EVAL_STEPS);
                    last_filled = filled;
                }
            }
        }

        stats::print_footer();
    }
}

fn interact_pair(pair: &mut [[u8; TAPE_SIZE]]) {
    let mut combined = [0u8; COMBINED_SIZE];
    combined[..TAPE_SIZE].copy_from_slice(&pair[0]);
    combined[TAPE_SIZE..].copy_from_slice(&pair[1]);
    interpreter::run(&mut combined);
    pair[0].copy_from_slice(&combined[..TAPE_SIZE]);
    pair[1].copy_from_slice(&combined[TAPE_SIZE..]);
}
