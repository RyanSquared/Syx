use super::object::{SyxType};

error_chain! {
    errors {
        // undump.rs

        BufferNotReadable(t: String) {
            display("no values read from buffer: {}", t),
        }

        BufferNotEmpty {
            display("bytes left over from buffer"),
        }

        InvalidVerification(name: String, err: String) {
            display("error verifying {}: {}", name, err),
        }

        InvalidConstantType(t: SyxType) {
            display("bad value for constant: {:?}", t),
        }

        InvalidUpvalueIndex(index: usize) {
            display("could not find upvalue index: {}", index),
        }

        InvalidSourceName {
            display("could not match source name from UTF8"),
        }

        // opcodes.rs

        InvalidOpCode {
            display("opcode is not valid"),
        }

        // objects.rs

        InvalidType(t: u8) {
            display("invalid type parameter loaded: {}", t),
        }
    }
}
