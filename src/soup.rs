use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

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

    fn interact(&mut self, i: usize, j: usize) {
        let mut combined = [0u8; COMBINED_SIZE];
        combined[..TAPE_SIZE].copy_from_slice(&self.tapes[i]);
        combined[TAPE_SIZE..].copy_from_slice(&self.tapes[j]);

        interpreter::run(&mut combined);

        self.tapes[i].copy_from_slice(&combined[..TAPE_SIZE]);
        self.tapes[j].copy_from_slice(&combined[TAPE_SIZE..]);
    }

    fn epoch(&mut self) {
        let size = self.tapes.len();
        let mut order: Vec<usize> = (0..size).collect();
        order.shuffle(&mut self.rng);

        for chunk in order.chunks_exact(2) {
            self.interact(chunk[0], chunk[1]);
        }

        if MUTATION_RATE > 0.0 {
            self.mutate_soup();
        }
    }

    // Apply soup-wide background mutation.
    fn mutate_soup(&mut self) {
        let total_bytes = (self.tapes.len() * TAPE_SIZE) as f64;
        let n_mutations = (total_bytes * MUTATION_RATE).round() as usize;

        for _ in 0..n_mutations {
            let tape_idx = self.rng.gen_range(0..self.tapes.len());
            let byte_idx = self.rng.gen_range(0..TAPE_SIZE);
            self.tapes[tape_idx][byte_idx] = self.rng.gen_range(0u8..=255);
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

        for _ in 0..EPOCHS {
            self.epoch();
            self.epoch_count += 1;

            if self.epoch_count % EVAL_STEPS == 0 {
                stats::report(&self.tapes, self.epoch_count, log_path);
            } else {
                // update the progress bar towards the next eval
                let progress = self.epoch_count % EVAL_STEPS;
                stats::update_progress(progress, EVAL_STEPS);
            }
        }

        stats::print_footer();
    }
}
