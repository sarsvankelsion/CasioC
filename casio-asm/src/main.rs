use casio_asm::{compile_csc_to_asm, Model, ModelDb, compile_csc_with_db};
use std::env;
use std::fs;

fn print_usage() {
    eprintln!("casioc - CasioC (.csc) compiler (580vnx/880btg)  [csc -> asm]");
    eprintln!("Usage: casioc [--model 580vnx|880btg] [--emit asm|hex] [-f hex|key] <file.csc>");
    eprintln!("       casioc --model 580vnx --emit asm hello.csc > hello.asm");
    eprintln!("       casioc --model 580vnx --emit hex -f hex hello.csc");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut model = Model::Fx580vnx;
    let mut format = "hex".to_string();
    let mut emit_mode = "asm".to_string();
    let mut file: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--model" | "-m" => {
                if i + 1 < args.len() {
                    if let Some(m) = Model::parse(&args[i + 1]) {
                        model = m;
                    } else {
                        eprintln!("unknown model {}", args[i + 1]);
                        print_usage();
                        std::process::exit(1);
                    }
                    i += 2;
                } else { i += 1; }
            }
            "-f" | "--format" => {
                if i + 1 < args.len() { format = args[i + 1].clone(); i += 2; } else { i += 1; }
            }
            "--emit" | "-e" => {
                if i + 1 < args.len() { emit_mode = args[i + 1].clone(); i += 2; } else { i += 1; }
            }
            "--help" | "-h" => { print_usage(); return; }
            s if s.starts_with('-') => { eprintln!("unknown flag {s}"); print_usage(); std::process::exit(1); }
            s => { file = Some(s.to_string()); i += 1; }
        }
    }

    let source = if let Some(path) = file {
        fs::read_to_string(&path).unwrap_or_else(|e| { eprintln!("read {path}: {e}"); std::process::exit(1); })
    } else {
        use std::io::Read;
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s).unwrap();
        s
    };

    if emit_mode == "asm" {
        match compile_csc_to_asm(&source) {
            Ok(asm) => { print!("{}", asm); }
            Err(e) => { eprintln!("compile error: {e}"); std::process::exit(1); }
        }
        return;
    }
    // hex/key emit needs model DB
    let db = ModelDb::load(model, None, None).unwrap_or_else(|e| { eprintln!("load model {}: {e}", model.id()); std::process::exit(1); });
    match compile_csc_with_db(&source, &db) {
        Ok((bytes, home, log)) => {
            eprint!("{log}");
            eprintln!("model={} home=[{:04X}] len={} bytes", db.model.id(), home, bytes.len());
            if format == "hex" {
                println!("{}", bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" "));
            } else {
                println!("home=[{:04X}] {}", home, bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" "));
            }
        }
        Err(e) => {
            eprintln!("compile error: {e}");
            std::process::exit(1);
        }
    }
}
