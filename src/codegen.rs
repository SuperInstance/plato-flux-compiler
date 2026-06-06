//! FLUX bytecode emitter for the Condition AST.
//!
//! Codegen strategy:
//! - Each sensor reference becomes a LOAD (via sensor index lookup)
//! - Comparisons emit PUSH immediate + CMP + conditional store
//! - AND: evaluate both sides, logical AND (short-circuit via JZ)
//! - OR: short-circuit via JNZ
//! - NOT: logical NOT on result
//! - Range: two comparisons AND'd together
//! - Threshold: simplified — loads sensor, compares against delta threshold
//!
//! Result is left in R0: 0 = no alarm, 1 = alarm triggered.

use flux_core::bytecode::opcodes::Op;

use crate::ast::{CmpOp, Condition};

/// Bytecode label for jump resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Label(#[allow(dead_code)] usize);

/// A compiled function with its bytecode and metadata.
#[derive(Debug, Clone)]
pub struct CompiledFunction {
    pub name: String,
    pub bytecode: Vec<u8>,
    pub sensor_slots: Vec<String>,
}

/// The code generator that walks the AST and emits FLUX bytecode.
pub struct Codegen {
    /// Mapping from sensor name to slot index.
    sensor_map: Vec<(String, u8)>,
    /// Bytecode buffer.
    code: Vec<u8>,
    /// Next available register for temporaries (R0 = result).
    next_reg: u8,
}

impl Codegen {
    pub fn new() -> Self {
        Codegen {
            sensor_map: Vec::new(),
            code: Vec::new(),
            next_reg: 1, // R0 reserved for result
        }
    }

    /// Get or allocate a sensor slot index.
    fn sensor_slot(&mut self, name: &str) -> u8 {
        if let Some((_, slot)) = self.sensor_map.iter().find(|(s, _)| s == name) {
            return *slot;
        }
        let slot = self.next_reg;
        self.next_reg += 1;
        self.sensor_map.push((name.to_string(), slot));
        slot
    }

    /// Allocate a temporary register.
    #[allow(dead_code)]
    fn alloc_reg(&mut self) -> u8 {
        let r = self.next_reg;
        self.next_reg += 1;
        r
    }

    /// Emit a single byte.
    fn emit(&mut self, byte: u8) {
        self.code.push(byte);
    }

    /// Emit an Op.
    fn emit_op(&mut self, op: Op) {
        self.code.push(op as u8);
    }

    /// Current code position (for jump target calculation).
    fn pos(&self) -> usize {
        self.code.len()
    }

    /// Patch a 16-bit address at `offset` with `target`.
    fn patch_u16(&mut self, offset: usize, target: usize) {
        let bytes = (target as u16).to_le_bytes();
        self.code[offset] = bytes[0];
        self.code[offset + 1] = bytes[1];
    }

    /// Compile a condition into bytecode, returning the register holding the result (always R0).
    pub fn compile(&mut self, cond: &Condition) -> Result<u8, String> {
        match cond {
            Condition::Comparison(c) => self.compile_comparison(&c.sensor, c.op, c.value),
            Condition::Range(r) => {
                // sensor >= low AND sensor <= high
                let left = Condition::Comparison(crate::ast::Comparison {
                    sensor: r.sensor.clone(),
                    op: CmpOp::GreaterEqual,
                    value: r.low,
                });
                let right = Condition::Comparison(crate::ast::Comparison {
                    sensor: r.sensor.clone(),
                    op: CmpOp::LessEqual,
                    value: r.high,
                });
                self.compile(&Condition::And(Box::new(left), Box::new(right)))
            }
            Condition::Threshold(t) => {
                // Simplified: check if sensor > delta (represents the rise amount)
                // In a real system this would access a time-series buffer
                self.compile_comparison(&t.sensor, CmpOp::GreaterThan, t.delta)
            }
            Condition::Not(inner) => {
                let reg = self.compile(inner)?;
                // INOT the result
                self.emit_op(Op::INEG); // Use INEG as logical NOT substitute
                self.emit(reg);
                Ok(reg)
            }
            Condition::And(left, right) => {
                let result_reg = 0u8; // R0
                let left_reg = self.compile(left)?;
                // Store left result in R0
                if left_reg != result_reg {
                    self.emit_op(Op::MOV);
                    self.emit(result_reg);
                    self.emit(left_reg);
                }
                // JZ to false (skip right side)
                self.emit_op(Op::JZ);
                self.emit(result_reg);
                let patch_pos = self.pos();
                self.emit(0u8); // placeholder low byte
                self.emit(0u8); // placeholder high byte
                // Right side
                let right_reg = self.compile(right)?;
                // MOV right result to R0
                if right_reg != result_reg {
                    self.emit_op(Op::MOV);
                    self.emit(result_reg);
                    self.emit(right_reg);
                }
                let end_pos = self.pos();
                self.patch_u16(patch_pos, end_pos);
                // AND: both in R0 now, right result is already there (short-circuit semantics)
                // If left was 0, we jumped past right, so R0=0. If left was 1, R0=right.
                // This gives us AND semantics.
                Ok(result_reg)
            }
            Condition::Or(left, right) => {
                let result_reg = 0u8;
                let left_reg = self.compile(left)?;
                if left_reg != result_reg {
                    self.emit_op(Op::MOV);
                    self.emit(result_reg);
                    self.emit(left_reg);
                }
                // JNZ to true (skip right side — already 1)
                self.emit_op(Op::JNZ);
                self.emit(result_reg);
                let patch_pos = self.pos();
                self.emit(0u8);
                self.emit(0u8);
                // Right side
                let right_reg = self.compile(right)?;
                if right_reg != result_reg {
                    self.emit_op(Op::MOV);
                    self.emit(result_reg);
                    self.emit(right_reg);
                }
                let end_pos = self.pos();
                self.patch_u16(patch_pos, end_pos);
                Ok(result_reg)
            }
        }
    }

    /// Compile a simple comparison: `sensor op value`
    fn compile_comparison(&mut self, sensor: &str, op: CmpOp, value: i64) -> Result<u8, String> {
        let result_reg = 0u8; // R0 = result
        let sensor_slot = self.sensor_slot(sensor);

        // LOAD sensor into its slot register
        self.emit_op(Op::LOAD);
        self.emit(sensor_slot);
        // Encode sensor as a 16-bit slot ID
        self.emit((sensor_slot as u16).to_le_bytes()[0]);
        self.emit((sensor_slot as u16).to_le_bytes()[1]);

        // PUSH the threshold value
        self.emit_op(Op::PUSH);
        // Encode value as i32 (4 bytes little-endian)
        let val_bytes = (value as i32).to_le_bytes();
        self.emit(val_bytes[0]);
        self.emit(val_bytes[1]);
        self.emit(val_bytes[2]);
        self.emit(val_bytes[3]);

        // MOV sensor_slot -> result_reg for comparison
        self.emit_op(Op::MOV);
        self.emit(result_reg);
        self.emit(sensor_slot);

        // CMP
        self.emit_op(Op::CMP);
        self.emit(result_reg);
        // The comparison opcode variant is encoded as a sub-op
        self.emit(match op {
            CmpOp::Equal => 0,
            CmpOp::NotEqual => 1,
            CmpOp::LessThan => 2,
            CmpOp::LessEqual => 3,
            CmpOp::GreaterThan => 4,
            CmpOp::GreaterEqual => 5,
        });

        // STORE result to R0
        self.emit_op(Op::STORE);
        self.emit(result_reg);
        self.emit(0); // result slot

        Ok(result_reg)
    }

    /// Take the generated bytecode and metadata, consuming the codegen.
    pub fn into_function(mut self, name: String) -> CompiledFunction {
        // Append HALT
        self.emit_op(Op::HALT);

        let sensor_slots: Vec<String> = self
            .sensor_map
            .iter()
            .map(|(name, slot)| {
                let _ = slot;
                name.clone()
            })
            .collect();

        CompiledFunction {
            name,
            bytecode: self.code,
            sensor_slots,
        }
    }

    /// Get the current bytecode (without HALT).
    pub fn bytecode(&self) -> &[u8] {
        &self.code
    }
}

/// Disassemble bytecode to a human-readable string.
pub fn disassemble(bytecode: &[u8]) -> String {
    let mut lines = Vec::new();
    let mut i = 0;
    while i < bytecode.len() {
        let offset = i;
        let op_byte = bytecode[i];
        let op_name = match Op::from_byte(op_byte) {
            Some(op) => format!("{}", op),
            None => match op_byte {
                0x02 => "LOAD".to_string(),
                0x03 => "STORE".to_string(),
                _ => format!("UNKNOWN(0x{:02X})", op_byte),
            },
        };

        match op_byte {
            // LOAD = 0x02 (not in flux-core from_byte)
            0x02 if i + 3 < bytecode.len() => {
                let slot = u16::from_le_bytes([bytecode[i + 2], bytecode[i + 3]]);
                lines.push(format!("{:04x}: LOAD R{}, [{}]", offset, bytecode[i + 1], slot));
                i += 4;
            }
            // STORE = 0x03
            0x03 if i + 1 < bytecode.len() => {
                lines.push(format!("{:04x}: STORE R{}, [{}]", offset, bytecode[i + 1], bytecode.get(i + 2).copied().unwrap_or(0)));
                i += 3;
            }
            _ => match Op::from_byte(op_byte) {
                Some(Op::MOV) if i + 2 < bytecode.len() => {
                    lines.push(format!("{:04x}: MOV R{}, R{}", offset, bytecode[i + 1], bytecode[i + 2]));
                    i += 3;
                }
                Some(Op::MOVI) if i + 1 < bytecode.len() => {
                    lines.push(format!("{:04x}: MOVI R{}, R{}", offset, bytecode[i + 1], bytecode.get(i + 2).copied().unwrap_or(0)));
                    i += 3;
                }
                Some(Op::PUSH) => {
                    if i + 4 < bytecode.len() {
                        let val = i32::from_le_bytes([
                            bytecode[i + 1],
                            bytecode[i + 2],
                            bytecode[i + 3],
                            bytecode[i + 4],
                        ]);
                        lines.push(format!("{:04x}: PUSH {}", offset, val));
                        i += 5;
                    } else {
                        lines.push(format!("{:04x}: PUSH (truncated)", offset));
                        i += 1;
                    }
                }
                Some(Op::CMP) if i + 1 < bytecode.len() => {
                    let cmp_type = bytecode.get(i + 2).copied().unwrap_or(0);
                    let cmp_name = match cmp_type {
                        0 => "EQ", 1 => "NE", 2 => "LT", 3 => "LE", 4 => "GT", 5 => "GE", _ => "??",
                    };
                    lines.push(format!("{:04x}: CMP R{}, {}", offset, bytecode[i + 1], cmp_name));
                    i += 3;
                }
                Some(Op::JZ) if i + 2 < bytecode.len() => {
                    let target = u16::from_le_bytes([bytecode[i + 2], bytecode[i + 3]]);
                    lines.push(format!("{:04x}: JZ R{}, 0x{:04x}", offset, bytecode[i + 1], target));
                    i += 4;
                }
                Some(Op::JNZ) if i + 2 < bytecode.len() => {
                    let target = u16::from_le_bytes([bytecode[i + 2], bytecode[i + 3]]);
                    lines.push(format!("{:04x}: JNZ R{}, 0x{:04x}", offset, bytecode[i + 1], target));
                    i += 4;
                }
                Some(Op::INEG) if i + 1 <= bytecode.len() => {
                    lines.push(format!("{:04x}: INEG R{}", offset, bytecode.get(i + 1).copied().unwrap_or(0)));
                    i += 2;
                }
                Some(Op::HALT) => {
                    lines.push(format!("{:04x}: HALT", offset));
                    i += 1;
                }
                _ => {
                    lines.push(format!("{:04x}: {} (raw)", offset, op_name));
                    i += 1;
                }
            }
        }
    }
    lines.join("\n")
}

impl Default for Codegen {
    fn default() -> Self {
        Self::new()
    }
}
