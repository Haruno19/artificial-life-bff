pub const TAPE_SIZE: usize = 64;
pub const COMBINED_SIZE: usize = TAPE_SIZE * 2;
pub const MAX_STEPS: usize = 8192;

// Soup parameters
pub const SOUP_SIZE: usize = 1 << 17; // number of tapes in the soup
pub const EPOCHS: usize = 50_000; // total epochs to run (1 epoch = SOUP_SIZE/2 pairings)
pub const EVAL_STEPS: usize = 10_000; // print stats every N epochs

// Set to 0.0 to disable random mutation.
pub const MUTATION_RATE: f64 = 0.00024;
