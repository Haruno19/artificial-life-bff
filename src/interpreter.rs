use crate::constants::*;

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

fn match_forward(tape: &[u8; COMBINED_SIZE], from: usize) -> Option<usize> {
    let mut depth: i32 = 1;
    let mut i = from + 1;
    while i < COMBINED_SIZE {
        match tape[i] {
            OP_LOOP_OPEN => depth += 1,
            OP_LOOP_CLOSE => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn match_backward(tape: &[u8; COMBINED_SIZE], from: usize) -> Option<usize> {
    let mut depth: i32 = 1;
    let mut i = from;
    while i > 0 {
        i -= 1;
        match tape[i] {
            OP_LOOP_CLOSE => depth += 1,
            OP_LOOP_OPEN => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

// Run the BFF interpreter on a 128-byte combined tape (tape A ++ tape B).
// Mutates the tape in place — this is how programs modify themselves and each other.
// Stops after MAX_STEPS instructions.
//
// State:
//   pc  — program counter, reads opcodes sequentially
//   h0  — head 0, used for arithmetic and as copy source/dest
//   h1  — head 1, used as the second copy source/dest
//
// All three pointers wrap around modulo COMBINED_SIZE.
pub fn run(tape: &mut [u8; COMBINED_SIZE]) {
    let mut pc: usize = 0;
    let mut h0: usize = 0;
    let mut h1: usize = 0;

    for _ in 0..MAX_STEPS {
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
                if tape[h0] == 0 {
                    if let Some(target) = match_forward(tape, pc) {
                        pc = target;
                    }
                }
            }
            OP_LOOP_CLOSE => {
                if tape[h0] != 0 {
                    if let Some(target) = match_backward(tape, pc) {
                        pc = target;
                    }
                }
            }
            _ => {} // everything else is raw data, no-op
        }

        pc = (pc + 1) % COMBINED_SIZE;
    }
}
