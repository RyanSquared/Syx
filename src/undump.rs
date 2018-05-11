// ::TODO:: move to a config or something?

const SYX_VERSION_MAJOR: u8 = 0x5;
const SYX_VERSION_MINOR: u8 = 0x3;

// Verification information

const SYX_HEADER: &[u8] = b"\x1bLua"; // <ESC>Lua, can't have <ESC> in source

const SYX_DATA: &[u8] = b"\x19\x93\r\n\x1a\n";
const SYX_VERSION: u8 = SYX_VERSION_MAJOR * 16 + SYX_VERSION_MINOR;
const SYX_FORMAT: u8  = 0; // official PUC-Rio format

type Instruction = u32;
type SyxInteger = u64;
type SyxNumber = f64;

pub struct LoadState {
    // ::TODO:: lua_State *L; ?
    input: Box<Iterator<Item = u8>>,
    name: Box<::std::fmt::Display>,
}

macro_rules! expand {
    ($item:ty) => {{
        (::std::mem::size_of::<$item>(), stringify!($item))
    }}
}

#[allow(dead_code)]
impl LoadState {
    pub fn from_read(mut input: impl ::std::io::Read,
                 name: impl Into<String>) -> LoadState {
        let mut buffer: Vec<u8> = Vec::new();
        let string_name = name.into();
        input.read_to_end(&mut buffer).expect(
            &format!("no values read from buffer: {}", string_name));
        LoadState {
            input: Box::new(buffer.into_iter()),
            name: Box::new(string_name),
        }
    }

    fn assert_verification(&mut self, val: bool, err: impl ::std::fmt::Display) {
        if !val {
            self.raise_from_verification(err);
        }
    }

    fn raise_from_verification(&mut self, err: impl ::std::fmt::Display) {
        // ::TODO:: push error onto VM stack?
        // ::TODO:: actually *have* a VM stack?
        panic!("Error with {}: {}", self.name, err);
    }

    fn get_byte(&mut self) -> Option<u8> {
        self.input.next()
    }
    
    fn get_range(&mut self, range: usize) -> Option<Vec<u8>> {
        let mut ret: Vec<u8> = Vec::with_capacity(range);
        for i in 0..range {
            if let Some(mut ch) = self.input.next() {
                ret.push(ch);
            } else {
                self.raise_from_verification(format!("Missing byte at pos: {}", i));
                return None;
            }
        };
        Some(ret)
    }

    fn check_size(&mut self, size: (usize, &'static str)) {
        if let Some(bytecode_size) = self.get_byte() {
            self.assert_verification(bytecode_size == (size.0 as u8),
                                     format!("size mismatch: {}", size.1))
        }
    }

    fn check_literal(&mut self, value_impl: impl Into<Vec<u8>>,
                     err: impl ::std::fmt::Display)
    {
        let value = value_impl.into();
        if let Some(literal) = self.get_range(value.len()) {
            self.assert_verification(literal == value,
                                     format!("literal mismatch: {}", err));
        }
    }

    pub fn check_header(&mut self) {
        self.check_literal(SYX_HEADER, "header");
        if let Some(version) = self.get_byte() {
            self.assert_verification(version == SYX_VERSION, "version mismatch");
        }
        if let Some(format) = self.get_byte() {
            self.assert_verification(format == SYX_FORMAT, "format mismatch");
        }
        self.check_literal(SYX_DATA, "load order verification");
        self.check_size(expand!(u32));
        self.check_size(expand!(usize));
        self.check_size(expand!(Instruction));
        self.check_size(expand!(SyxInteger));
        self.check_size(expand!(SyxNumber));
    }
}
