//! Peephole optimizer for generated FLUX bytecode.
//!
//! Optimizations:
//! - Constant folding: simplify `LOAD const; PUSH const; CMP` sequences
//! - Dead code elimination: remove code after HALT/JMP
//! - Jump threading: collapse chains of JMP/JZ/JNZ

use flux_core::bytecode::opcodes::Op;

use crate::codegen::CompiledFunction;

/// Optimize a compiled function in-place.
pub fn optimize(func: &mut CompiledFunction) {
    dead_code_elimination(&mut func.bytecode);
    jump_threading(&mut func.bytecode);
    // Constant folding is more nuanced with the sensor model; skip for now
}

/// Remove unreachable code after HALT/JMP.
fn dead_code_elimination(code: &mut Vec<u8>) {
    let mut i = 0;
    let mut halt_pos: Option<usize> = None;

    while i < code.len() {
        let op_byte = code[i];
        match Op::from_byte(op_byte) {
            Some(Op::HALT) => {
                halt_pos = Some(i + 1); // keep the HALT itself
                break;
            }
            Some(Op::LOAD) => i += 4,
            Some(Op::STORE) => i += 3,
            Some(Op::MOV) => i += 3,
            Some(Op::MOVI) => i += 3,
            Some(Op::PUSH) => i += 5,
            Some(Op::CMP) => i += 3,
            Some(Op::JZ) | Some(Op::JNZ) => i += 4,
            Some(Op::JMP) => i += 3,
            Some(Op::INEG) => i += 2,
            _ => i += 1,
        }
    }

    if let Some(pos) = halt_pos {
        code.truncate(pos);
    }
}

/// Thread chained jumps: if a JZ/JNZ/JMP target is another JMP, follow it.
fn jump_threading(code: &mut Vec<u8>) {
    let mut changed = true;
    while changed {
        changed = false;
        let mut i = 0;
        while i < code.len() {
            let op_byte = code[i];
            match Op::from_byte(op_byte) {
                Some(Op::JZ) | Some(Op::JNZ) => {
                    if i + 3 < code.len() {
                        let target = u16::from_le_bytes([code[i + 2], code[i + 3]]) as usize;
                        if target < code.len() {
                            if let Some(Op::JMP) = Op::from_byte(code[target]) {
                                if target + 2 < code.len() {
                                    let final_target =
                                        u16::from_le_bytes([code[target + 1], code[target + 2]]);
                                    code[i + 2] = final_target.to_le_bytes()[0];
                                    code[i + 3] = final_target.to_le_bytes()[1];
                                    changed = true;
                                }
                            }
                        }
                    }
                    i += 4;
                }
                Some(Op::JMP) => {
                    if i + 2 < code.len() {
                        let target = u16::from_le_bytes([code[i + 1], code[i + 2]]) as usize;
                        if target < code.len() {
                            if let Some(Op::JMP) = Op::from_byte(code[target]) {
                                if target + 2 < code.len() {
                                    let final_target =
                                        u16::from_le_bytes([code[target + 1], code[target + 2]]);
                                    code[i + 1] = final_target.to_le_bytes()[0];
                                    code[i + 2] = final_target.to_le_bytes()[1];
                                    changed = true;
                                }
                            }
                        }
                    }
                    i += 3;
                }
                Some(Op::LOAD) => i += 4,
                Some(Op::STORE) => i += 3,
                Some(Op::MOV) => i += 3,
                Some(Op::MOVI) => i += 3,
                Some(Op::PUSH) => i += 5,
                Some(Op::CMP) => i += 3,
                Some(Op::INEG) => i += 2,
                Some(Op::HALT) => break,
                _ => i += 1,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dead_code_elimination() {
        let mut code = vec![
            Op::MOVI as u8, 0, 42, // MOVI R0, 42
            Op::HALT as u8, // HALT
            Op::MOVI as u8, 1, 99, // dead code
            Op::MOVI as u8, 2, 77, // dead code
        ];
        dead_code_elimination(&mut code);
        assert_eq!(code.len(), 4); // 3 bytes MOVI + 1 byte HALT
        assert_eq!(code[3], Op::HALT as u8);
    }

    #[test]
    fn test_jump_threading_chain() {
        // JMP to 0x05, which is JMP to 0x0A
        let mut code = vec![
            Op::JMP as u8, 0x05, 0x00, // JMP 0x0005
            Op::NOP as u8,
            Op::NOP as u8,
            Op::JMP as u8, 0x0A, 0x00, // JMP 0x000A
            Op::HALT as u8,
        ];
        jump_threading(&mut code);
        // First JMP should now target 0x000A
        let target = u16::from_le_bytes([code[1], code[2]]);
        assert_eq!(target, 0x000A);
    }

    #[test]
    fn test_optimize_removes_dead_code() {
        let mut func = CompiledFunction {
            name: "test".to_string(),
            bytecode: vec![
                Op::MOVI as u8, 0, 1,
                Op::HALT as u8,
                Op::MOVI as u8, 1, 2,
                Op::HALT as u8,
            ],
            sensor_slots: vec![],
        };
        optimize(&mut func);
        // Should truncate after first HALT
        assert_eq!(func.bytecode.len(), 4);
    }
}
