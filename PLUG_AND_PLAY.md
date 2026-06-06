# Plug & Play — plato-flux-compiler

> Copy these templates. Change the condition strings. You're compiling.

---

## Pattern 1: Compile a Single Alarm Condition

Parse and compile one condition string to FLUX bytecode.

```rust
use plato_flux_compiler::parser::Parser;
use plato_flux_compiler::codegen::{Codegen, disassemble};

fn main() {
    // ↓ Change this condition string ↓
    let condition = "coolant_temp_c > 95";

    let cond = Parser::parse(condition).expect("Parse failed");
    let mut codegen = Codegen::new();
    codegen.compile(&cond).expect("Compile failed");
    let func = codegen.into_function("my_alarm".to_string());

    println!("Compiled '{}' → {} ops", condition, func.bytecode.len());
    println!("{}", disassemble(&func.bytecode));
}
```

**Change:** the condition string. Supported syntax:
- `sensor > value`, `>=`, `<`, `<=`, `==`, `!=`
- `condition AND condition`, `OR`, `NOT`
- `sensor in [low, high]`
- `sensor rising_by delta in N_ticks`
- Parentheses for grouping: `(a > 1) AND (b < 2)`

---

## Pattern 2: Compile a Full Room Config

Multiple alarms compiled into a single dispatch-table-driven program.

```rust
use plato_flux_compiler::integration::{RoomConfig, AlarmDef, compile_room};

fn main() {
    let config = RoomConfig {
        room_name: "engine_room".to_string(),
        room_type: "marine_engine".to_string(),
        alarms: vec![
            // ↓ Change these alarm definitions ↓
            AlarmDef {
                id: "overheat".to_string(),
                condition: "coolant_temp_c > 95".to_string(),
                severity: "critical".to_string(),
            },
            AlarmDef {
                id: "low_oil".to_string(),
                condition: "oil_pressure_psi < 20".to_string(),
                severity: "critical".to_string(),
            },
            AlarmDef {
                id: "combined".to_string(),
                condition: "coolant_temp_c > 90 AND oil_pressure_psi < 30".to_string(),
                severity: "warning".to_string(),
            },
        ],
    };

    let room = compile_room(&config).expect("Compilation failed");

    println!("Room: {} — {} alarms, {} ops total",
        config.room_name, room.functions.len(), room.bytecode.len());
    for (id, offset) in &room.dispatch_table {
        println!("  {} → 0x{:04x}", id, offset);
    }
}
```

**Change:** room_name, room_type, alarm IDs, conditions, severities.

---

## Pattern 3: Parse + Inspect + Compile (Debugging)

See what the parser and codegen produce at each stage.

```rust
use plato_flux_compiler::parser::Parser;
use plato_flux_compiler::codegen::{Codegen, disassemble};
use plato_flux_compiler::optimizer::Optimizer;

fn main() {
    // ↓ Your condition ↓
    let source = "(temp > 90 AND pressure < 100) OR rpm > 3500";

    // 1. Parse → AST
    let cond = Parser::parse(source).expect("Parse error");
    println!("AST: {}", cond);
    println!("Sensors used: {:?}", cond.sensors());

    // 2. Compile → bytecode
    let mut cg = Codegen::new();
    cg.compile(&cond).expect("Codegen error");
    let func = cg.into_function("debug_alarm".to_string());
    println!("\nRaw bytecode ({} ops):", func.bytecode.len());
    println!("{}", disassemble(&func.bytecode));

    // 3. Optimize
    let optimized = Optimizer::optimize(func.bytecode);
    println!("\nOptimized ({} ops):", optimized.len());
    println!("{}", disassemble(&optimized));
}
```

**Change:** the condition string to test any alarm logic.

---

## Quick Reference

| What | API | Example |
|------|-----|---------|
| Parse condition | `Parser::parse("...")` | `Parser::parse("temp > 95")` |
| Compile to bytecode | `Codegen::new().compile(&ast)` | Chain `.compile()` then `.into_function("name")` |
| Disassemble | `disassemble(&bytecode)` | Human-readable FLUX assembly |
| Optimize | `Optimizer::optimize(bytecode)` | Dead code + jump threading |
| Compile room | `compile_room(&config)` | Multiple alarms → dispatch table |
| Get sensors | `condition.sensors()` | `Vec<String>` of referenced sensors |
