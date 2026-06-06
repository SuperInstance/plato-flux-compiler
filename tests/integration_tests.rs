//! Comprehensive test suite for plato-flux-compiler.

#[cfg(test)]
mod parser_tests {
    use plato_flux_compiler::ast::*;
    use plato_flux_compiler::parser::Parser;

    #[test]
    fn test_simple_comparison_gt() {
        let cond = Parser::parse("temp > 95").unwrap();
        assert_eq!(
            cond,
            Condition::Comparison(Comparison {
                sensor: "temp".to_string(),
                op: CmpOp::GreaterThan,
                value: 95,
            })
        );
    }

    #[test]
    fn test_simple_comparison_lt() {
        let cond = Parser::parse("rpm < 1500").unwrap();
        assert_eq!(
            cond,
            Condition::Comparison(Comparison {
                sensor: "rpm".to_string(),
                op: CmpOp::LessThan,
                value: 1500,
            })
        );
    }

    #[test]
    fn test_comparison_equal() {
        let cond = Parser::parse("status == 1").unwrap();
        assert_eq!(
            cond,
            Condition::Comparison(Comparison {
                sensor: "status".to_string(),
                op: CmpOp::Equal,
                value: 1,
            })
        );
    }

    #[test]
    fn test_comparison_not_equal() {
        let cond = Parser::parse("status != 0").unwrap();
        assert_eq!(
            cond,
            Condition::Comparison(Comparison {
                sensor: "status".to_string(),
                op: CmpOp::NotEqual,
                value: 0,
            })
        );
    }

    #[test]
    fn test_comparison_gte() {
        let cond = Parser::parse("pressure >= 100").unwrap();
        assert_eq!(
            cond,
            Condition::Comparison(Comparison {
                sensor: "pressure".to_string(),
                op: CmpOp::GreaterEqual,
                value: 100,
            })
        );
    }

    #[test]
    fn test_comparison_lte() {
        let cond = Parser::parse("flow <= 50").unwrap();
        assert_eq!(
            cond,
            Condition::Comparison(Comparison {
                sensor: "flow".to_string(),
                op: CmpOp::LessEqual,
                value: 50,
            })
        );
    }

    #[test]
    fn test_and_condition() {
        let cond = Parser::parse("temp > 90 AND pressure < 100").unwrap();
        match cond {
            Condition::And(left, right) => {
                assert_eq!(*left, Condition::Comparison(Comparison {
                    sensor: "temp".to_string(),
                    op: CmpOp::GreaterThan,
                    value: 90,
                }));
                assert_eq!(*right, Condition::Comparison(Comparison {
                    sensor: "pressure".to_string(),
                    op: CmpOp::LessThan,
                    value: 100,
                }));
            }
            _ => panic!("expected AND condition"),
        }
    }

    #[test]
    fn test_or_condition() {
        let cond = Parser::parse("temp > 100 OR pressure > 200").unwrap();
        match cond {
            Condition::Or(_, _) => {}
            _ => panic!("expected OR condition"),
        }
    }

    #[test]
    fn test_not_condition() {
        let cond = Parser::parse("NOT temp > 50").unwrap();
        match cond {
            Condition::Not(inner) => {
                assert_eq!(*inner, Condition::Comparison(Comparison {
                    sensor: "temp".to_string(),
                    op: CmpOp::GreaterThan,
                    value: 50,
                }));
            }
            _ => panic!("expected NOT condition"),
        }
    }

    #[test]
    fn test_range_check() {
        let cond = Parser::parse("temp in [20, 80]").unwrap();
        assert_eq!(
            cond,
            Condition::Range(RangeCheck {
                sensor: "temp".to_string(),
                low: 20,
                high: 80,
            })
        );
    }

    #[test]
    fn test_threshold_check() {
        let cond = Parser::parse("temp rising_by 5 in 10_ticks").unwrap();
        assert_eq!(
            cond,
            Condition::Threshold(ThresholdCheck {
                sensor: "temp".to_string(),
                delta: 5,
                ticks: 10,
            })
        );
    }

    #[test]
    fn test_complex_nested() {
        // (temp > 90 AND pressure < 100) OR status == 2
        let cond = Parser::parse("temp > 90 AND pressure < 100 OR status == 2").unwrap();
        match cond {
            Condition::Or(left, right) => {
                match *left {
                    Condition::And(_, _) => {}
                    _ => panic!("expected AND on left side"),
                }
                match *right {
                    Condition::Comparison(Comparison { ref op, .. }) => {
                        assert_eq!(*op, CmpOp::Equal);
                    }
                    _ => panic!("expected comparison on right side"),
                }
            }
            _ => panic!("expected OR condition"),
        }
    }

    #[test]
    fn test_negative_value() {
        let cond = Parser::parse("temp > -10").unwrap();
        assert_eq!(
            cond,
            Condition::Comparison(Comparison {
                sensor: "temp".to_string(),
                op: CmpOp::GreaterThan,
                value: -10,
            })
        );
    }

    #[test]
    fn test_parse_error_invalid_op() {
        let result = Parser::parse("temp >> 95");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_empty() {
        let result = Parser::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_underscore_sensor_names() {
        let cond = Parser::parse("coolant_temp_c > 95").unwrap();
        match cond {
            Condition::Comparison(c) => {
                assert_eq!(c.sensor, "coolant_temp_c");
            }
            _ => panic!("expected comparison"),
        }
    }
}

#[cfg(test)]
mod ast_tests {
    use plato_flux_compiler::ast::*;

    #[test]
    fn test_comparison_display() {
        let c = Comparison {
            sensor: "temp".to_string(),
            op: CmpOp::GreaterThan,
            value: 95,
        };
        assert_eq!(format!("{}", c), "temp > 95");
    }

    #[test]
    fn test_range_display() {
        let r = RangeCheck {
            sensor: "temp".to_string(),
            low: 20,
            high: 80,
        };
        assert_eq!(format!("{}", r), "temp in [20, 80]");
    }

    #[test]
    fn test_threshold_display() {
        let t = ThresholdCheck {
            sensor: "temp".to_string(),
            delta: 5,
            ticks: 10,
        };
        assert_eq!(format!("{}", t), "temp rising_by 5 in 10_ticks");
    }

    #[test]
    fn test_condition_not_display() {
        let cond = Condition::Not(Box::new(Condition::Comparison(Comparison {
            sensor: "temp".to_string(),
            op: CmpOp::GreaterThan,
            value: 50,
        })));
        assert_eq!(format!("{}", cond), "NOT (temp > 50)");
    }

    #[test]
    fn test_condition_and_display() {
        let cond = Condition::And(
            Box::new(Condition::Comparison(Comparison {
                sensor: "a".to_string(),
                op: CmpOp::GreaterThan,
                value: 1,
            })),
            Box::new(Condition::Comparison(Comparison {
                sensor: "b".to_string(),
                op: CmpOp::LessThan,
                value: 2,
            })),
        );
        assert_eq!(format!("{}", cond), "(a > 1 AND b < 2)");
    }

    #[test]
    fn test_condition_or_display() {
        let cond = Condition::Or(
            Box::new(Condition::Comparison(Comparison {
                sensor: "x".to_string(),
                op: CmpOp::GreaterThan,
                value: 10,
            })),
            Box::new(Condition::Comparison(Comparison {
                sensor: "y".to_string(),
                op: CmpOp::LessThan,
                value: 5,
            })),
        );
        assert_eq!(format!("{}", cond), "(x > 10 OR y < 5)");
    }

    #[test]
    fn test_sensors_comparison() {
        let cond = Condition::Comparison(Comparison {
            sensor: "temp".to_string(),
            op: CmpOp::GreaterThan,
            value: 95,
        });
        assert_eq!(cond.sensors(), vec!["temp"]);
    }

    #[test]
    fn test_sensors_and() {
        let cond = Condition::And(
            Box::new(Condition::Comparison(Comparison {
                sensor: "temp".to_string(),
                op: CmpOp::GreaterThan,
                value: 90,
            })),
            Box::new(Condition::Comparison(Comparison {
                sensor: "pressure".to_string(),
                op: CmpOp::LessThan,
                value: 100,
            })),
        );
        let sensors = cond.sensors();
        assert!(sensors.contains(&"pressure".to_string()));
        assert!(sensors.contains(&"temp".to_string()));
    }

    #[test]
    fn test_cmp_op_from_str() {
        assert_eq!(CmpOp::from_str(">"), Some(CmpOp::GreaterThan));
        assert_eq!(CmpOp::from_str(">="), Some(CmpOp::GreaterEqual));
        assert_eq!(CmpOp::from_str("<"), Some(CmpOp::LessThan));
        assert_eq!(CmpOp::from_str("<="), Some(CmpOp::LessEqual));
        assert_eq!(CmpOp::from_str("=="), Some(CmpOp::Equal));
        assert_eq!(CmpOp::from_str("="), Some(CmpOp::Equal));
        assert_eq!(CmpOp::from_str("!="), Some(CmpOp::NotEqual));
        assert_eq!(CmpOp::from_str("??"), None);
    }

    #[test]
    fn test_equality() {
        let c1 = Comparison {
            sensor: "temp".to_string(),
            op: CmpOp::GreaterThan,
            value: 95,
        };
        let c2 = Comparison {
            sensor: "temp".to_string(),
            op: CmpOp::GreaterThan,
            value: 95,
        };
        assert_eq!(c1, c2);
    }
}

#[cfg(test)]
mod codegen_tests {
    use flux_core::bytecode::opcodes::Op;
    use plato_flux_compiler::ast::*;
    use plato_flux_compiler::codegen::{Codegen, disassemble};
    use plato_flux_compiler::parser::Parser;

    #[test]
    fn test_codegen_comparison() {
        let cond = Parser::parse("temp > 95").unwrap();
        let mut cg = Codegen::new();
        let reg = cg.compile(&cond).unwrap();
        assert_eq!(reg, 0); // result in R0

        let func = cg.into_function("test".to_string());
        let bc = &func.bytecode;

        // Should contain LOAD, PUSH, MOVI, CMP, STORE, HALT
        assert!(bc.iter().any(|&b| b == Op::LOAD as u8), "missing LOAD");
        assert!(bc.iter().any(|&b| b == Op::PUSH as u8), "missing PUSH");
        assert!(bc.iter().any(|&b| b == Op::CMP as u8), "missing CMP");
        assert!(bc.iter().any(|&b| b == Op::STORE as u8), "missing STORE");
        assert!(bc.iter().any(|&b| b == Op::HALT as u8), "missing HALT");
    }

    #[test]
    fn test_codegen_comparison_bytecode_sequence() {
        // temp > 95 → LOAD, PUSH 95, MOVI, CMP GT, STORE
        let cond = Parser::parse("temp > 95").unwrap();
        let mut cg = Codegen::new();
        cg.compile(&cond).unwrap();
        let bc = cg.bytecode().to_vec();

        // First instruction: LOAD
        assert_eq!(bc[0], Op::LOAD as u8);
        // Should find PUSH
        let push_pos = bc.iter().position(|&b| b == Op::PUSH as u8).unwrap();
        // Value after PUSH: 95 as i32 LE
        let val = i32::from_le_bytes([
            bc[push_pos + 1],
            bc[push_pos + 2],
            bc[push_pos + 3],
            bc[push_pos + 4],
        ]);
        assert_eq!(val, 95);
        // CMP should use GT (4)
        let cmp_pos = bc.iter().position(|&b| b == Op::CMP as u8).unwrap();
        assert_eq!(bc[cmp_pos + 2], 4); // GT
    }

    #[test]
    fn test_codegen_and() {
        let cond = Parser::parse("a > 1 AND b < 2").unwrap();
        let mut cg = Codegen::new();
        cg.compile(&cond).unwrap();
        let func = cg.into_function("test".to_string());

        // Should have JZ for short-circuit AND
        assert!(func.bytecode.iter().any(|&b| b == Op::JZ as u8), "missing JZ for AND");
    }

    #[test]
    fn test_codegen_or() {
        let cond = Parser::parse("a > 1 OR b < 2").unwrap();
        let mut cg = Codegen::new();
        cg.compile(&cond).unwrap();
        let func = cg.into_function("test".to_string());

        // Should have JNZ for short-circuit OR
        assert!(func.bytecode.iter().any(|&b| b == Op::JNZ as u8), "missing JNZ for OR");
    }

    #[test]
    fn test_codegen_not() {
        let cond = Parser::parse("NOT temp > 50").unwrap();
        let mut cg = Codegen::new();
        cg.compile(&cond).unwrap();
        let func = cg.into_function("test".to_string());

        // Should have INEG for NOT
        assert!(func.bytecode.iter().any(|&b| b == Op::INEG as u8), "missing INEG for NOT");
    }

    #[test]
    fn test_codegen_range() {
        let cond = Parser::parse("temp in [20, 80]").unwrap();
        let mut cg = Codegen::new();
        cg.compile(&cond).unwrap();
        let func = cg.into_function("test".to_string());

        // Range expands to (temp >= 20 AND temp <= 80), should have JZ
        assert!(func.bytecode.iter().any(|&b| b == Op::JZ as u8));
    }

    #[test]
    fn test_codegen_threshold() {
        let cond = Parser::parse("temp rising_by 5 in 10_ticks").unwrap();
        let mut cg = Codegen::new();
        cg.compile(&cond).unwrap();
        let func = cg.into_function("test".to_string());

        // Should have LOAD, PUSH, CMP
        assert!(func.bytecode.iter().any(|&b| b == Op::LOAD as u8));
        assert!(func.bytecode.iter().any(|&b| b == Op::CMP as u8));
    }

    #[test]
    fn test_disassemble() {
        let cond = Parser::parse("temp > 95").unwrap();
        let mut cg = Codegen::new();
        cg.compile(&cond).unwrap();
        let func = cg.into_function("test".to_string());

        let listing = disassemble(&func.bytecode);
        assert!(listing.contains("LOAD"));
        assert!(listing.contains("PUSH"));
        assert!(listing.contains("CMP"));
        assert!(listing.contains("HALT"));
    }

    #[test]
    fn test_compiled_function_sensor_slots() {
        let cond = Parser::parse("coolant_temp_c > 95").unwrap();
        let mut cg = Codegen::new();
        cg.compile(&cond).unwrap();
        let func = cg.into_function("test".to_string());
        assert!(func.sensor_slots.contains(&"coolant_temp_c".to_string()));
    }
}

#[cfg(test)]
mod roundtrip_tests {
    use plato_flux_compiler::codegen::{self, disassemble};
    use plato_flux_compiler::parser::Parser;

    #[test]
    fn test_roundtrip_comparison() {
        let cond = Parser::parse("coolant_temp_c > 95").unwrap();
        let mut cg = codegen::Codegen::new();
        cg.compile(&cond).unwrap();
        let func = cg.into_function("coolant_overtemp".to_string());
        let listing = disassemble(&func.bytecode);

        // Should have the expected structure
        assert!(listing.contains("LOAD"));
        assert!(listing.contains("PUSH 95"));
        assert!(listing.contains("CMP"));
        assert!(listing.contains("HALT"));
    }

    #[test]
    fn test_roundtrip_complex() {
        let cond = Parser::parse("temp > 90 AND pressure < 100 OR status == 2").unwrap();
        let mut cg = codegen::Codegen::new();
        cg.compile(&cond).unwrap();
        let func = cg.into_function("complex_alarm".to_string());
        let listing = disassemble(&func.bytecode);

        // Should have jumps for OR and AND
        assert!(listing.contains("JNZ") || listing.contains("JZ"));
        assert!(listing.contains("HALT"));
    }

    #[test]
    fn test_roundtrip_not() {
        let cond = Parser::parse("NOT temp > 50").unwrap();
        let mut cg = codegen::Codegen::new();
        cg.compile(&cond).unwrap();
        let func = cg.into_function("inverted".to_string());
        let listing = disassemble(&func.bytecode);

        assert!(listing.contains("INEG"));
        assert!(listing.contains("HALT"));
    }

    #[test]
    fn test_roundtrip_range() {
        let cond = Parser::parse("temp in [20, 80]").unwrap();
        let mut cg = codegen::Codegen::new();
        cg.compile(&cond).unwrap();
        let func = cg.into_function("range_check".to_string());
        let listing = disassemble(&func.bytecode);

        // Range → two comparisons + AND
        assert!(listing.contains("CMP"));
        assert!(listing.contains("HALT"));
    }
}
