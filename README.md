# plato-flux-compiler

> Compiles Plato room configurations and alarm logic into FLUX bytecode for deterministic execution on any platform.

## Why This Exists

The Plato system monitors physical and virtual environments — fishing boat engine rooms, server racks, factory floors, greenhouse zones. Each room has alarm conditions that must be evaluated **deterministically** and **fast**. We can't afford language runtime differences between an ESP32 sitting in a bilge pump and a GPU cluster in a data center.

**plato-flux-compiler** solves this by compiling alarm condition strings into **FLUX bytecode** — a portable, deterministic instruction set that runs identically everywhere via the [flux-core](https://github.com/SuperInstance/flux-core) VM. The same alarm logic that guards a marine diesel engine also guards a Kubernetes cluster, with zero behavioral difference.

### The Problem It Solves

| Without plato-flux-compiler | With plato-flux-compiler |
|---|---|
| Alarm logic embedded in application code | Alarm logic compiled to portable bytecode |
| Different implementations per platform | One bytecode, runs on ESP32/GPU/cloud |
| Hard to test alarm conditions offline | Compile + disassemble + unit test |
| No optimization of condition evaluation | Peephole optimizer for generated code |
| Condition changes require code redeployment | Conditions compiled at deploy time |

## Architecture

```
┌──────────────────────────────────────────────────────┐
│                 plato-flux-compiler                   │
│                                                      │
│  "coolant_temp_c > 95"                               │
│         │                                            │
│         ▼                                            │
│  ┌──────────┐    ┌─────────┐    ┌──────────┐        │
│  │  Parser   │───▶│   AST   │───▶│  Codegen │        │
│  └──────────┘    └─────────┘    └──────────┘        │
│                                      │               │
│                                      ▼               │
│                               ┌──────────┐           │
│                               │Optimizer │           │
│                               └──────────┘           │
│                                      │               │
│                                      ▼               │
│                            FLUX Bytecode              │
│                            (Op enum from flux-core)   │
└──────────────────────────────────────────────────────┘
         │                              │
         ▼                              ▼
  plato-room-configs              flux-core VM
  (condition strings)            (execution runtime)
                                        │
                                        ▼
                                 plato-engine-block
                                 (room orchestrator)
```

### Pipeline

1. **Parse** — Condition strings are tokenized and parsed into an AST via recursive descent
2. **AST** — Tree of comparisons, logical operators (AND/OR/NOT), range checks, and threshold monitors
3. **Codegen** — Walk the AST, emit FLUX bytecode (LOAD sensor, PUSH value, CMP, conditional jumps)
4. **Optimize** — Peephole optimizer applies dead code elimination and jump threading
5. **Integrate** — Compile full room configs with multiple alarms into a dispatch-table-driven FLUX program

## Condition DSL Reference

### Comparison

Basic sensor threshold check. Triggers when the comparison is true.

```
sensor_name operator value
```

| Operator | Meaning |
|----------|---------|
| `>` | Greater than |
| `>=` | Greater than or equal |
| `<` | Less than |
| `<=` | Less than or equal |
| `==` | Equal |
| `!=` | Not equal |

**Examples:**
```
coolant_temp_c > 95
oil_pressure_psi < 20
engine_rpm > 3500
ambient_temp_c >= 40
```

### Logical Operators

Combine conditions with AND, OR, and NOT.

```
condition AND condition
condition OR condition
NOT condition
```

**Examples:**
```
temp > 90 AND pressure < 100
temp > 100 OR pressure > 200
NOT temp < 50
cpu_temp > 80 AND ambient_temp > 30 OR fan_rpm < 500
```

AND has higher precedence than OR. Use parentheses for grouping:

```
(temp > 90 AND pressure < 100) OR status == 2
```

### Range Check

Triggers when a sensor value is **outside** the specified range.

```
sensor in [low, high]
```

**Example:**
```
coolant_temp_c in [80, 105]
```

Internally compiles to `sensor >= low AND sensor <= high`.

### Threshold Check

Triggers when a sensor has risen by a delta within a number of ticks.

```
sensor rising_by delta in N_ticks
```

**Example:**
```
exhaust_temp_c rising_by 50 in 10_ticks
```

## Codegen Examples

### Simple Comparison: `temp > 95`

```
LOAD R1, [1]          ; Load sensor "temp" into R1
PUSH 95               ; Push comparison value
MOV R0, R1            ; Move sensor value to result register
CMP R0, GT            ; Compare: greater than
STORE R0, [0]         ; Store result (0 or 1)
HALT
```

### AND Condition: `temp > 90 AND pressure < 100`

```
LOAD R1, [1]          ; Load "temp"
PUSH 90
MOV R0, R1
CMP R0, GT
STORE R0, [0]         ; Left side result → R0
MOV R0, R0            ; Ensure result in R0
JZ R0, 0x0020         ; Short-circuit: if false, skip right side
LOAD R2, [2]          ; Load "pressure"
PUSH 100
MOV R0, R2
CMP R0, LT
STORE R0, [0]         ; Right side result → R0
HALT
```

### OR Condition: `a > 1 OR b < 2`

```
; Left side evaluates
MOV R0, R0            ; Left result in R0
JNZ R0, 0x001a        ; Short-circuit: if true, skip right side
; Right side evaluates
HALT
```

### NOT Condition: `NOT temp > 50`

```
; Evaluate temp > 50 into R0
INEG R0               ; Logical negation
HALT
```

## Connection to the Plato Ecosystem

```
plato-room-configs ──▶ plato-flux-compiler ──▶ flux-core VM
       │                      │                      │
   JSON configs         Compile conditions      Execute bytecode
   with alarm            into portable           deterministically
   condition strings     FLUX programs           on any platform
                                                     │
                                              plato-engine-block
                                              orchestrates rooms,
                                              feeds sensor data,
                                              handles alarm triggers
```

### plato-room-configs
Source of truth for room definitions. Each room config contains alarm conditions as human-readable strings.

### flux-core
The FLUX bytecode VM. Provides the `Op` enum (instruction set), interpreter, and runtime. plato-flux-compiler outputs programs that execute on this VM.

### plato-engine-block
The room orchestrator. Loads compiled FLUX programs, feeds sensor data into the VM, and handles alarm triggers. Uses flux-core to execute the bytecode produced by this compiler.

## API Usage

```rust
use plato_flux_compiler::parser::Parser;
use plato_flux_compiler::codegen::{Codegen, disassemble};
use plato_flux_compiler::integration::{RoomConfig, AlarmDef, compile_room};

// Parse and compile a single condition
let cond = Parser::parse("coolant_temp_c > 95")?;
let mut codegen = Codegen::new();
codegen.compile(&cond)?;
let func = codegen.into_function("coolant_overtemp".to_string());

// Disassemble for debugging
println!("{}", disassemble(&func.bytecode));

// Compile a full room config
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
    ],
};
let room = compile_room(&config)?;
println!("Compiled {} alarms, {} bytes bytecode",
    room.functions.len(), room.bytecode.len());
```

## Modules

| Module | Description |
|--------|-------------|
| `parser` | Recursive-descent parser for condition strings |
| `ast` | Condition AST types (Comparison, RangeCheck, ThresholdCheck, Condition) |
| `codegen` | FLUX bytecode emitter with disassembler |
| `optimizer` | Peephole optimizations (dead code elimination, jump threading) |
| `integration` | Full room config compilation with dispatch tables |

## Optimizations

### Dead Code Elimination
Removes unreachable code after HALT instructions. Any bytecode following a HALT is stripped.

### Jump Threading
Collapses chains of jumps. If a JZ/JNZ/JMP targets another JMP, the first jump is patched to target the final destination directly, avoiding unnecessary indirection.

## Testing

```bash
cargo test
```

47 tests covering:
- Parser: all comparison operators, AND/OR/NOT, range checks, thresholds, nested expressions, error handling
- AST: construction, display formatting, equality, sensor collection
- Codegen: bytecode structure verification for each node type, disassembly
- Optimizer: dead code elimination, jump threading
- Integration: full room configs, fishing boat engine room, server rack, dispatch tables
- Roundtrip: parse → compile → disassemble → verify structure

## License

MIT
