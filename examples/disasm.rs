use plato_flux_compiler::parser::Parser;
use plato_flux_compiler::codegen::{Codegen, disassemble};

fn main() {
    let cond = Parser::parse("temp > 95").unwrap();
    let mut cg = Codegen::new();
    cg.compile(&cond).unwrap();
    let func = cg.into_function("test".to_string());
    println!("bytecode len: {}", func.bytecode.len());
    for (i, b) in func.bytecode.iter().enumerate() {
        print!("{:02x} ", b);
    }
    println!();
    println!("---");
    let listing = disassemble(&func.bytecode);
    println!("{}", listing);
}
