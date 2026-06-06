//! Integration module: compile a full room config into a FLUX program.
//!
//! A room config is a JSON structure containing:
//! - Room name and metadata
//! - A list of alarm conditions (as condition strings)
//!
//! This module compiles all conditions into individual FLUX functions
//! and produces a dispatch table mapping alarm IDs to function offsets.

use crate::ast::Condition;
use crate::codegen::{self, CompiledFunction};
use crate::optimizer;
use crate::parser::Parser;

/// A single alarm definition from a room config.
#[derive(Debug, Clone)]
pub struct AlarmDef {
    pub id: String,
    pub condition: String,
    pub severity: String,
}

/// A room configuration.
#[derive(Debug, Clone)]
pub struct RoomConfig {
    pub room_name: String,
    pub room_type: String,
    pub alarms: Vec<AlarmDef>,
}

/// A compiled room program with all functions and dispatch metadata.
#[derive(Debug, Clone)]
pub struct CompiledRoom {
    pub room_name: String,
    pub functions: Vec<CompiledFunction>,
    /// Maps alarm ID to function index.
    pub dispatch: Vec<(String, usize)>,
    /// Combined bytecode with a dispatch header.
    pub bytecode: Vec<u8>,
}

/// Compile a room config into a complete FLUX program.
pub fn compile_room(config: &RoomConfig) -> Result<CompiledRoom, String> {
    let mut functions = Vec::new();
    let mut dispatch = Vec::new();

    for (i, alarm) in config.alarms.iter().enumerate() {
        let cond = Parser::parse(&alarm.condition)
            .map_err(|e| format!("alarm '{}': {}", alarm.id, e.message))?;

        let mut codegen = codegen::Codegen::new();
        codegen.compile(&cond)?;
        let mut func = codegen.into_function(format!("alarm_{}", alarm.id));

        // Apply optimizations
        optimizer::optimize(&mut func);

        functions.push(func);
        dispatch.push((alarm.id.clone(), i));
    }

    // Build combined bytecode with dispatch table header
    let mut bytecode = Vec::new();

    // Header: number of functions (2 bytes LE)
    let num_funcs = functions.len() as u16;
    bytecode.extend_from_slice(&num_funcs.to_le_bytes());

    // For each function: offset (2 bytes), then the bytecode
    let header_size = 2 + functions.len() * 2;
    let mut offsets = Vec::new();
    let mut current_offset = header_size;

    for func in &functions {
        offsets.push(current_offset as u16);
        current_offset += func.bytecode.len();
    }

    // Write offset table
    for off in &offsets {
        bytecode.extend_from_slice(&off.to_le_bytes());
    }

    // Append all function bytecodes
    for func in &functions {
        bytecode.extend_from_slice(&func.bytecode);
    }

    Ok(CompiledRoom {
        room_name: config.room_name.clone(),
        functions,
        dispatch,
        bytecode,
    })
}

/// Parse conditions for testing convenience.
pub fn parse_condition(input: &str) -> Result<Condition, String> {
    Parser::parse(input).map_err(|e| e.message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(name: &str, alarms: Vec<(&str, &str)>) -> RoomConfig {
        RoomConfig {
            room_name: name.to_string(),
            room_type: "generic".to_string(),
            alarms: alarms
                .into_iter()
                .enumerate()
                .map(|(i, (cond, sev))| AlarmDef {
                    id: format!("alarm_{}", i),
                    condition: cond.to_string(),
                    severity: sev.to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn test_compile_simple_room() {
        let config = make_config("test_room", vec![
            ("temp > 95", "critical"),
            ("rpm < 1500", "warning"),
        ]);
        let result = compile_room(&config).unwrap();
        assert_eq!(result.functions.len(), 2);
        assert_eq!(result.dispatch.len(), 2);
        assert!(result.bytecode.len() > 0);
    }

    #[test]
    fn test_fishing_boat_engine_room() {
        let config = RoomConfig {
            room_name: "fishing_boat_engine".to_string(),
            room_type: "marine_engine".to_string(),
            alarms: vec![
                AlarmDef {
                    id: "coolant_overtemp".to_string(),
                    condition: "coolant_temp_c > 95".to_string(),
                    severity: "critical".to_string(),
                },
                AlarmDef {
                    id: "oil_pressure_low".to_string(),
                    condition: "oil_pressure_psi < 20".to_string(),
                    severity: "critical".to_string(),
                },
                AlarmDef {
                    id: "rpm_excessive".to_string(),
                    condition: "engine_rpm > 3500".to_string(),
                    severity: "warning".to_string(),
                },
                AlarmDef {
                    id: "exhaust_overtemp".to_string(),
                    condition: "exhaust_temp_c > 550".to_string(),
                    severity: "critical".to_string(),
                },
                AlarmDef {
                    id: "coolant_range".to_string(),
                    condition: "coolant_temp_c in [80, 105]".to_string(),
                    severity: "info".to_string(),
                },
            ],
        };
        let result = compile_room(&config).unwrap();
        assert_eq!(result.functions.len(), 5);
        // Verify dispatch table
        assert_eq!(result.dispatch[0].0, "coolant_overtemp");
        assert_eq!(result.dispatch[4].0, "coolant_range");
    }

    #[test]
    fn test_server_rack_config() {
        let config = RoomConfig {
            room_name: "server_rack_a1".to_string(),
            room_type: "datacenter".to_string(),
            alarms: vec![
                AlarmDef {
                    id: "cpu_overtemp".to_string(),
                    condition: "cpu_temp_c > 85".to_string(),
                    severity: "critical".to_string(),
                },
                AlarmDef {
                    id: "ambient_high".to_string(),
                    condition: "ambient_temp_c > 35".to_string(),
                    severity: "warning".to_string(),
                },
                AlarmDef {
                    id: "fan_speed_low".to_string(),
                    condition: "fan_rpm < 1000".to_string(),
                    severity: "warning".to_string(),
                },
                AlarmDef {
                    id: "combined_alarm".to_string(),
                    condition: "cpu_temp_c > 80 AND ambient_temp_c > 30".to_string(),
                    severity: "critical".to_string(),
                },
            ],
        };
        let result = compile_room(&config).unwrap();
        assert_eq!(result.functions.len(), 4);
        // Check the combined alarm uses AND
        let combined = &result.functions[3];
        assert!(combined.bytecode.len() > 10); // should have multiple instructions
    }

    #[test]
    fn test_dispatch_table_offsets() {
        let config = make_config("test", vec![
            ("a > 1", "low"),
            ("b < 2", "low"),
        ]);
        let result = compile_room(&config).unwrap();
        // Bytecode should start with num_functions = 2
        let num = u16::from_le_bytes([result.bytecode[0], result.bytecode[1]]);
        assert_eq!(num, 2);
        // First offset should be after header (2 + 2*2 = 6 bytes)
        let first_offset = u16::from_le_bytes([result.bytecode[2], result.bytecode[3]]);
        assert_eq!(first_offset, 6);
    }

    #[test]
    fn test_empty_room() {
        let config = RoomConfig {
            room_name: "empty_room".to_string(),
            room_type: "test".to_string(),
            alarms: vec![],
        };
        let result = compile_room(&config).unwrap();
        assert_eq!(result.functions.len(), 0);
        assert_eq!(result.dispatch.len(), 0);
        // Bytecode is just the header: 0 functions
        assert_eq!(result.bytecode.len(), 2);
    }
}
