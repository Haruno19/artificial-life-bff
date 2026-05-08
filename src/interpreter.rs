pub const TAPE_SIZE: usize = 64;
pub const COMBINED_SIZE: usize = TAPE_SIZE * 2;
pub const MAX_STEPS: usize = 8192;

// BFF opcodes - the 10 valid instructions
// Everything else on the tape is treated as a no-op (raw data)
const OP_H0_LEFT: u8 = b'<';
const OP_H0_RIGHT: u8 = b'>';
const OP_H1_LEFT: u8 = b'{';
const OP_H1_RIGHT: u8 = b'}';
const OP_INC: u8 = b'+';
const OP_DEC: u8 = b'-';
const OP_COPY_0_TO_1: u8 = b'.'; // tape[h1] = tape[h0]
const OP_COPY_1_TO_0: u8 = b','; // tape[h0] = tape[h1]
const OP_LOOP_OPEN: u8 = b'[';
const OP_LOOP_CLOSE: u8 = b']';

// Precompute matching bracket positions so [ and ] are O(1) during execution.
// Returns an array where jumps[i] = matching bracket index for position i,
// or 0 if position i is not a bracket.
// TODO: implement — use a stack to match brackets as you scan left to right.
//       Unmatched brackets should be left as 0 (treated as no-ops during execution).
fn build_jump_table(tape: &[u8; COMBINED_SIZE]) -> [Option<usize>; COMBINED_SIZE] {
    let mut jumps = [None; COMBINED_SIZE];
    let mut stack: Vec<usize> = Vec::new();

    for i in 0..COMBINED_SIZE {
        if tape[i] == OP_LOOP_OPEN {
            stack.push(i);
        }
        if tape[i] == OP_LOOP_CLOSE {
            if let Some(open) = stack.pop() {
                jumps[open] = Some(i);
                jumps[i] = Some(open);
            }
        }
    }
    return jumps;
}

// Run the BFF interpreter on a 128-byte combined tape (tape A ++ tape B).
// Mutates the tape in place — this is how programs modify themselves and each other.
// Stops after MAX_STEPS instructions or when pc goes out of bounds.
//
// State:
//   pc  — program counter, reads opcodes sequentially
//   h0  — head 0, used for arithmetic and as copy source/dest
//   h1  — head 1, used as the second copy source/dest
//
// All three pointers wrap around modulo COMBINED_SIZE.
pub fn run(tape: &mut [u8; COMBINED_SIZE]) {
    let jumps = build_jump_table(tape);

    let mut pc: usize = 0;
    let mut h0: usize = 0;
    let mut h1: usize = 0;

    for _ in 0..MAX_STEPS {
        if pc >= COMBINED_SIZE {
            break;
        }

        match tape[pc] {
            OP_H0_LEFT => h0 = (h0 + COMBINED_SIZE - 1) % COMBINED_SIZE,
            OP_H0_RIGHT => h0 = (h0 + 1) % COMBINED_SIZE,
            OP_H1_LEFT => h1 = (h1 + COMBINED_SIZE - 1) % COMBINED_SIZE,
            OP_H1_RIGHT => h1 = (h1 + 1) % COMBINED_SIZE,
            OP_INC => tape[h0] = tape[h0].wrapping_add(1),
            OP_DEC => tape[h0] = tape[h0].wrapping_sub(1),
            OP_COPY_0_TO_1 => tape[h1] = tape[h0],
            OP_COPY_1_TO_0 => tape[h0] = tape[h1],
            OP_LOOP_OPEN => {
                if let Some(target) = jumps[pc] {
                    if tape[h0] == 0 {
                        pc = target
                    }
                }
            }
            OP_LOOP_CLOSE => {
                if let Some(target) = jumps[pc] {
                    if tape[h0] != 0 {
                        pc = target;
                    }
                }
            }
            _ => {} // everything else is raw data, no-op
        }

        pc += 1;
    }
}
