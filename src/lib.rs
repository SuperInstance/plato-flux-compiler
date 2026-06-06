//! # plato-flux-compiler
//!
//! Compiles Plato room configurations and alarm logic into FLUX bytecode,
//! enabling deterministic room execution on any platform (ESP32, GPU, cloud)
//! via the flux-core VM.
//!
//! ## Architecture
//!
//! ```text
//! Condition string → Parser → AST → Codegen → FLUX bytecode → Optimizer
//! ```
//!
//! ## Example
//!
//! ```rust
//! use plato_flux_compiler::parser::Parser;
//! use plato_flux_compiler::codegen::Codegen;
//! use plato_flux_compiler::ast::Condition;
//!
//! let cond = Parser::parse("coolant_temp_c > 95").unwrap();
//! let mut codegen = Codegen::new();
//! codegen.compile(&cond).unwrap();
//! let func = codegen.into_function("coolant_overtemp".to_string());
//! assert!(!func.bytecode.is_empty());
//! ```

pub mod ast;
pub mod codegen;
pub mod integration;
pub mod optimizer;
pub mod parser;

pub use codegen::{CompiledFunction, disassemble};
pub use integration::{RoomConfig, CompiledRoom, compile_room};
pub use parser::Parser;
