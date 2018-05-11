pub mod undump;

use std::fs::File;

fn main() -> std::io::Result<()> {
    let args: Vec<_> = ::std::env::args().collect();
    match args.get(1) {
        None => {
            panic!("Usage: {} [filename]", args.get(0).unwrap());
        },
        Some(file) => {
            let handle = File::open(file)?;
            let mut state = undump::LoadState::from_read(handle, file.clone());
            state.check_header();
            println!("test!");
            Ok(())
        }
    }
}
