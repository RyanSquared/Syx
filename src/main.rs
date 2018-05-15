mod undump;
mod limits;
mod object;
mod state;

use std::fs::File;

fn main() -> std::io::Result<()> {
    let args: Vec<_> = ::std::env::args().collect();
    match args.get(1) {
        None => {
            panic!("Usage: {} [filename]", args.get(0).unwrap());
        },
        Some(file) => {
            let handle = File::open(file)?;
            match undump::LoadState::from_read(handle, file.clone()) {
                Err(err) => panic!("fail! => {}", err),
                Ok(_state) => ()
            }
            Ok(())
        }
    }
}
