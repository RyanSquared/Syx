#[macro_use]
extern crate error_chain;

extern crate syx_codegen;

mod errors;
mod conf;
mod opcodes;
mod limits;
mod object;
mod state;
mod undump;

#[macro_use]
mod macros;

use object::SyxValue;
use std::fs::File;

fn main() {
    if let Err(e) = run() {
        use std::io::Write;
        let stderr = &mut ::std::io::stderr();
        let errmsg = "failed to write to stdout";

        writeln!(stderr, "error: {}", e).expect(errmsg);

        for e in e.iter().skip(1) {
            writeln!(stderr, "caused by: {}", e).expect(errmsg);
        }
    }
}

fn run() -> errors::Result<()> {
    let args: Vec<_> = ::std::env::args().collect();
    let main_chunk = match args.get(1) {
        None => {
            println!("test");
            panic!("Usage: {} [filename]", args[0]);
        }
        Some(file) => {
            let handle = File::open(file).unwrap();
            undump::LoadState::from_read(handle, file.clone())?
        }
    };
    if !main_chunk.constants.is_empty() {
        println!();
        println!("constants:");
        for constant in main_chunk.constants {
            match constant {
                SyxValue::Bool(boolean) => {
                    println!("bool: {}", boolean);
                }
                SyxValue::Number(n) => {
                    println!("number: {}", n);
                }
                SyxValue::Integer(n) => {
                    println!("integer: {}", n);
                }
                SyxValue::String(s) => match String::from_utf8(s.clone()) {
                    Ok(string) => println!("string: {}", string),
                    Err(_) => println!("vec<u8>: {:?}", s),
                },
                _ => (),
            }
        }
    }
    if !main_chunk.locvars.is_empty() {
        println!();
        println!("locals:");
        for local in main_chunk.locvars {
            if let Ok(name) = String::from_utf8(local.varname) {
                println!("local: {}", name)
            }
        }
    }
    if !main_chunk.upvalues.is_empty() {
        println!();
        println!("upvalues:");
        for upval in main_chunk.upvalues {
            if upval.name == vec![] {
                println!("instack: {}, idx: {}", upval.instack, upval.idx);
            } else if let Ok(string) = String::from_utf8(upval.name) {
                println!("{} [{}, {}]", string, upval.instack, upval.idx);
            }
        }
    }
    if !main_chunk.instructions.is_empty() {
        println!();
        println!("instructions:");
        for instr in main_chunk.instructions {
            println!("{:?}", instr);
        }
    }
    Ok(())
}
