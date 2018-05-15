mod undump;
mod limits;
mod object;
mod state;

use std::fs::File;
use object::SyxValue;

fn main() -> std::io::Result<()> {
    let args: Vec<_> = ::std::env::args().collect();
    let main_chunk = match args.get(1) {
        None => {
            panic!("Usage: {} [filename]", args.get(0).unwrap());
        },
        Some(file) => {
            let handle = File::open(file)?;
            match undump::LoadState::from_read(handle, file.clone()) {
                Err(err) => panic!("fail! => {}", err),
                Ok(main_chunk) => main_chunk
            }
        }
    };
    if main_chunk.constants.len() > 0 {
        println!("");
        println!("constants:");
        for constant in main_chunk.constants {
            match constant {
                SyxValue::Bool(boolean) => {
                    println!("bool: {}", boolean);
                },
                SyxValue::Number(n) => {
                    println!("number: {}", n);
                },
                SyxValue::Integer(n) => {
                    println!("integer: {}", n);
                },
                SyxValue::String(s) => {
                    match String::from_utf8(s.clone()) {
                        Ok(string) => println!("string: {}", string),
                        Err(_) => println!("vec<u8>: {:?}", s),
                    }
                }
                _ => {
                    ()
                }
            }
        }
    }
    if main_chunk.locvars.len() > 0 {
        println!("");
        println!("locals:");
        for local in main_chunk.locvars {
            if let Ok(name) = String::from_utf8(local.varname) {
                println!("local: {}", name)
            }
        }
    }
    if main_chunk.upvalues.len() > 0 {
        println!("");
        println!("upvalues:");
        for upval in main_chunk.upvalues {
            if upval.name == vec![] {
                println!("instack: {}, idx: {}", upval.instack, upval.idx);
            } else if let Ok(string) = String::from_utf8(upval.name) {
                println!("{} [{}, {}]", string, upval.instack, upval.idx);
            }
        }
    }
    Ok(())
}
