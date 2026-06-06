# Tutorial — plato-flux-compiler

> **By the end of this tutorial, you will have built a complete alarm compilation pipeline** — parsing condition strings, generating FLUX bytecode, optimizing it, and compiling a full multi-alarm room configuration.

---

## Prerequisites

- Rust 1.70+
- 15 minutes

## Step 1: Create the Project

```bash
cargo new alarm-compiler
cd alarm-compiler
```

```toml
[dependencies]
plato-flux-compiler = "0.1"
```

## Step 2: Parse a Simple Condition

The parser converts condition strings into an AST:

```rust
use plato_flux_compiler::parser::Parser;

fn main() {
    // Simple comparison
    let cond = Parser::parse("coolant_temp_c > 95").unwrap();
    println!("Parsed: {}", cond);

    // Compound condition
    let cond = Parser::parse("temp > 90 AND pressure < 100").unwrap();
    println!("Parsed: {}", cond);

    // Complex with NOT and parentheses
    let cond = Parser::parse("(temp > 90 AND pressure < 100) OR NOT status == 2").unwrap();
    println!("Parsed: {}", cond);
}
```

Output:
```
Parsed: coolant_temp_c > 95
Parsed: (temp > 90) AND (pressure < 100)
Parsed: ((temp > 90) AND (pressure < 100)) OR (NOT (status == 2))
```

**What happened:** The parser tokenizes the string, then uses recursive descent with operator precedence (NOT > AND > OR) to build an AST. Parentheses override precedence.

## Step 3: Compile to FLUX Bytecode

Generate executable bytecode from the AST:

```rust
use plato_flux_compiler::parser::Parser;
use plato_flux_compiler::codegen::Codegen;
use plato_flux_compiler::codegen::disassemble;

fn main() {
    let cond = Parser::parse("coolant_temp_c > 95").unwrap();

    let mut codegen = Codegen::new();
    codegen.compile(&cond).unwrap();
    let func = codegen.into_function("coolant_overtemp".to_string());

    println!("Function: {}", func.name);
    println!("Bytecode ({} ops):", func.bytecode.len());
    println!("{}", disassemble(&func.bytecode));
}
```

Output (illustrative):
```
Function: coolant_overtemp
Bytecode (6 ops):
  0x0000  LOAD R1, [1]          ; sensor "coolant_temp_c"
  0x0002  PUSH 95
  0x0004  MOV R0, R1
  0x0006  CMP R0, GT
  0x0008  STORE R0, [0]
  0x000a  HALT
```

**What happened:** The codegen walks the AST and emits FLUX instructions. A comparison loads the sensor value, pushes the threshold, compares, and stores the result. The `HALT` instruction marks the end.

## Step 4: Compile Compound Conditions

AND and OR conditions generate short-circuit evaluation code:

```rust
use plato_flux_compiler::parser::Parser;
use plato_flux_compiler::codegen::{Codegen, disassemble};

fn main() {
    // AND: if left is false, skip right
    let cond = Parser::parse("temp > 90 AND pressure < 100").unwrap();
    let mut cg = Codegen::new();
    cg.compile(&cond).unwrap();
    let func = cg.into_function("and_example".into());
    println!("=== AND condition ===");
    println!("{}", disassemble(&func.bytecode));

    // OR: if left is true, skip right
    let cond = Parser::parse("temp > 100 OR pressure > 200").unwrap();
    let mut cg = Codegen::new();
    cg.compile(&cond).unwrap();
    let func = cg.into_function("or_example".into());
    println!("\n=== OR condition ===");
    println!("{}", disassemble(&func.bytecode));

    // NOT: negate result
    let cond = Parser::parse("NOT temp < 50").unwrap();
    let mut cg = Codegen::new();
    cg.compile(&cond).unwrap();
    let func = cg.into_function("not_example".into());
    println!("\n=== NOT condition ===");
    println!("{}", disassemble(&func.bytecode));
}
```

**What happened:**
- **AND** evaluates the left side first. If false (`JZ`), it jumps past the right side — no wasted sensor reads.
- **OR** evaluates the left side first. If true (`JNZ`), it jumps past the right side.
- **NOT** evaluates the inner condition and applies `INEG` to flip the result.

## Step 5: Use Range and Threshold Checks

The DSL supports range and rising-by conditions:

```rust
use plato_flux_compiler::parser::Parser;
use plato_flux_compiler::codegen::{Codegen, disassemble};

fn main() {
    // Range check: value outside [80, 105]
    let cond = Parser::parse("coolant_temp_c in [80, 105]").unwrap();
    println!("Range: {}", cond);

    // Threshold check: rising by 50 in 10 ticks
    let cond = Parser::parse("exhaust_temp_c rising_by 50 in 10_ticks").unwrap();
    println!("Threshold: {}", cond);

    // Compile range check
    let mut cg = Codegen::new();
    cg.compile(&cond).unwrap();
    let func = cg.into_function("exhaust_rising".into());
    println!("\n{}", disassemble(&func.bytecode));
}
```

**What happened:** Range checks are lowered to `sensor >= low AND sensor <= high` in the AST. Threshold checks track sensor deltas over time windows.

## Step 6: Compile a Full Room Config

Put it all together — compile a multi-alarm room:

```rust
use plato_flux_compiler::integration::{RoomConfig, AlarmDef, compile_room};

fn main() {
    let config = RoomConfig {
        room_name: "engine_room".to_string(),
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
                id: "engine_overrev".to_string(),
                condition: "engine_rpm > 3500".to_string(),
                severity: "warning".to_string(),
            },
            AlarmDef {
                id: "combined_crisis".to_string(),
                condition: "coolant_temp_c > 90 AND oil_pressure_psi < 30".to_string(),
                severity: "critical".to_string(),
            },
        ],
    };

    let room = compile_room(&config).unwrap();

    println!("Room: {} ({})", config.room_name, config.room_type);
    println!("Compiled {} alarms", room.functions.len());
    println!("Total bytecode: {} ops", room.bytecode.len());
    println!("\nDispatch table:");
    for (id, offset) in &room.dispatch_table {
        println!("  {} → offset 0x{:04x}", id, offset);
    }

    // Disassemble each alarm
    for func in &room.functions {
        println!("\n--- {} ---", func.name);
        println!("{}", disassemble(&func.bytecode));
    }
}
```

**What happened:** `compile_room()` parses each alarm condition, generates bytecode, runs the optimizer, and concatenates everything into a single program with a dispatch table. The VM can evaluate individual alarms by jumping to their offset.

## Step 7: Verify Optimizations

The optimizer runs automatically during compilation. See what it does:

```rust
use plato_flux_compiler::optimizer::Optimizer;

// Before optimization: may contain dead code or jump chains
let raw_bytecode = /* ... */;
let optimized = Optimizer::optimize(raw_bytecode);
```

The optimizer applies:
- **Dead code elimination:** Removes instructions after HALT
- **Jump threading:** Collapses `JZ → JMP` chains into direct jumps

## Complete Pipeline Summary

```rust
// 1. Define your room
let config = RoomConfig { /* ... */ };

// 2. Compile
let room = compile_room(&config)?;

// 3. The output is ready for the flux-core VM
// room.bytecode — concatenated FLUX program
// room.dispatch_table — alarm_id → offset mapping
// room.functions — per-alarm compiled functions

// 4. Feed to plato-engine-block for execution
// The engine block loads the bytecode, feeds sensor data,
// and handles alarm triggers from the VM.
```

**Congratulations!** You've built a complete alarm compilation pipeline. Your condition strings are now deterministic bytecode that runs the same on every platform — from ESP32 to GPU cluster.

## What's Next?

- Integrate with `plato-engine-block` to execute compiled bytecode in a running room
- Use `plato-ternary-bridge` to feed ternary state into compiled conditions
- Use `plato-fleet-manager` to deploy compiled rooms across a fleet
- Add custom DSL operators for domain-specific alarm patterns
