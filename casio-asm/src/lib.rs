pub mod model;
pub mod lexer;
pub mod parser;
pub mod emit;
pub mod emit_asm;
pub mod charset;
pub mod npress;

pub use model::{Model, ModelDb, ModelPaths};

/// Compile a .csc source string for a given model into raw bytes (hex payload).
/// Returns (bytes, home_address, log).
pub fn compile_csc(source: &str, model: Model) -> Result<(Vec<u8>, u32, String), String> {
    let db = ModelDb::load(model, None, None)?;
    compile_csc_with_db(source, &db)
}

pub fn compile_csc_with_db(source: &str, db: &ModelDb) -> Result<(Vec<u8>, u32, String), String> {
    let tokens = lexer::lex(source)?;
    let ast = parser::parse(&tokens)?;
    emit::emit(&ast, db)
}

pub fn compile_csc_to_asm(source: &str) -> Result<String, String> {
    let tokens = lexer::lex(source)?;
    let ast = parser::parse(&tokens)?;
    emit_asm::emit_asm(&ast)
}
