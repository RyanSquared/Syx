use super::limits;
use super::object::{Instruction, SyxInteger, SyxInt, SyxNumber, SyxString,
                    Proto, SyxValue, SyxType, Upvalue};
use super::state;

type SyxResult = Result<(), String>;

// ::TODO:: move to a config or something?

const SYX_VERSION_MAJOR: u8 = 0x5;
const SYX_VERSION_MINOR: u8 = 0x3;

// Verification information

// <ESC>Lua, can't have <ESC> in source, so it's a useful escape character
const SYX_HEADER: &[u8] = b"\x1bLua"; 

const SYX_DATA: &[u8] = b"\x19\x93\r\n\x1a\n";
const SYX_VERSION: u8 = SYX_VERSION_MAJOR * 16 + SYX_VERSION_MINOR;
const SYX_FORMAT: u8  = 0; // official PUC-Rio format
const SYX_INT: SyxInteger = 0x5678;
const SYX_NUM: SyxNumber = (370.5f32 as SyxNumber);

pub struct LoadState {
    input: Box<Iterator<Item = u8>>,
    name: Box<::std::fmt::Display>,
    state: Option<state::SyxState>,
}

trait Primitives {}

macro_rules! primitive {
    ($($item:ty),*) => { $(impl Primitives for $item {})* }
}

primitive!(u8, u16, u32, u64);
primitive!(i8, i16, i32, i64);
primitive!(usize, isize);
primitive!(f32, f64);

macro_rules! expand {
    ($item:ty) => {{
        (::std::mem::size_of::<$item>(), stringify!($item))
    }}
}

#[allow(dead_code)]
impl LoadState {
    pub fn from_read(mut input: impl ::std::io::Read,
                 name: impl Into<String>) -> Result<Proto, String> {
        let mut buffer: Vec<u8> = Vec::new();
        let string_name = name.into();
        input.read_to_end(&mut buffer).expect(
            &format!("no values read from buffer: {}", string_name));
        let mut state = LoadState {
            input: Box::new(buffer.into_iter()),
            name: Box::new(string_name),
            state: None,
        };
        let proto = state.load_chunk(state::SyxState::new())?;
        match state.load::<u8>() {
            Err(_) => Ok(proto),
            Ok(_) => Err("bytes left over in stream, did not load all code".to_owned()),
        }
    }

    fn assert_verification(&mut self, val: bool, err: impl ::std::fmt::Display)
        -> SyxResult
    {
        if !val {
            return self.raise_from_verification(err);
        }
        Ok(())
    }

    fn raise_from_verification(&mut self, err: impl ::std::fmt::Display)
        -> SyxResult
    {
        Err(format!("Error with {}: {}", self.name, err))
    }

    fn load_range(&mut self, range: usize) -> Result<Vec<u8>, String> {
        let v: Vec<u8> = self.input.by_ref().take(range).collect();
        self.assert_verification(v.len() == range, format!("Not enough bytes: {}", range))?;
        Ok(v)
        // made redundant by the above
        /*
        let mut ret: Vec<u8> = Vec::with_capacity(range);
        for i in 0..range {
            if let Some(mut ch) = self.input.next() {
                ret.push(ch);
            } else {
                self.raise_from_verification(format!("Missing byte at pos: {}", i))?;
            }
        };
        Ok(ret)
        */
    }

    fn load<T: Copy + Primitives>(&mut self) -> Result<T, String> {
        /*
         * Safety of this method
         * ---
         * I had to mark unsafe because of the transmutation, but this is why
         * it will alwasy pass:
         *
         * 1. It will always transmute bytes directly to the size of T
         * 2. The size of T is loaded from self.load_range, which either grabs
         *    the whole thing or fails to load
         * 3. All values of type `Primitives` are defined at the top of this
         *    file and will always be Rust primitives.
         */
        // ::TODO:: optimize for <u8> when specializations lands:
        // https://github.com/rust-lang/rust/issues/31844
        // https://github.com/rust-lang/rfcs/blob/master/text/1210-impl-specialization.md
        let size = ::std::mem::size_of::<T>();
        let bytes = self.load_range(size)?;
        Ok(unsafe {* ::std::mem::transmute::<&u8, &T>(&bytes[0])})
    }

    fn load_string(&mut self) -> Result<SyxString, String> {
        let mut size: usize = self.load::<u8>()? as usize;
        if size == 0xFF {
            size = self.load::<usize>()?;
        }
        if size == 0 {
            // Turns out it can happen with stripped debug info. We'll just
            // return an empty string as it's not likely to be empty if it does
            // exist - wait, what happens in PUC-Rio Lua?..
            return Ok(vec![]);
        } else {
            // So, Lua has a concept of "short" and "long" strings. This can be
            // optimized later in the future, as well as the SyxString type, to
            // include a hash field.
            size -= 1;
            if size < limits::SYX_MAXSHORTLEN {
                // short string
                return self.load_range(size);
            } else { // long string
                return self.load_range(size);
            }
        }
    }

    fn load_constants(&mut self, proto: &mut Proto) -> SyxResult {
        let constant_count: isize = self.load::<i32>()? as isize;
        proto.constants.clear();
        for _ in 0..constant_count {
            // get type from byte
            proto.constants.push(match SyxType::from_u8(self.load::<u8>()?) {
                SyxType::TNIL => SyxValue::Nil,
                SyxType::TBOOLEAN => SyxValue::Bool(self.load::<u8>()? == 1),
                SyxType::TNUMFLT => SyxValue::Number(self.load::<SyxNumber>()?),
                SyxType::TNUMINT => SyxValue::Integer(self.load::<SyxInteger>()?),
                | SyxType::TSHRSTR
                | SyxType::TLNGSTR => SyxValue::String(self.load_string()?),
                x => {
                    return Err(format!("bad value for constant: {:?}", x));
                }
            });
        }
        Ok(())
    }

    fn load_code(&mut self, proto: &mut Proto) -> SyxResult {
        let count = self.load::<SyxInt>()?;
        proto.instructions.clear();
        proto.instructions.reserve(count as usize);
        for _ in 0..(count) {
            proto.instructions.push(self.load::<Instruction>()?);
        }
        Ok(())
    }

    fn load_protos(&mut self, proto: &mut Proto) -> SyxResult {
        let count = self.load::<SyxInt>()?;
        proto.protos.clear();
        proto.protos.reserve(count as usize);
        for _ in 0..(count) {
            proto.protos.push(Proto::new());
        }
        Ok(())
    }
    
    fn load_upvalues(&mut self, proto: &mut Proto) -> SyxResult {
        let upvalues_count = self.load::<SyxInt>()?;
        proto.upvalues.clear();
        proto.upvalues.reserve(upvalues_count as usize);
        for _ in 0..upvalues_count {
            proto.upvalues.push(Upvalue {
                name: vec![],
                instack: self.load::<u8>()?,
                idx: self.load::<u8>()?,
            })
        }
        Ok(())
    }

    fn load_debug(&mut self, proto: &mut Proto) -> SyxResult {
        let lines = self.load::<SyxInt>()? as usize;
        proto.lineinfo.clear();
        proto.lineinfo.reserve(lines);
        for _ in 0..lines {
            proto.lineinfo.push(self.load::<SyxInt>()?);
        }
        // ::TODO:: load LocVars
        // for now? trash them
        let size = self.load::<SyxInt>()? as usize;
        // load locvars
        for _ in 0..size {
            self.load_string()?; // varname
            self.load::<SyxInt>()?; // startpc
            self.load::<SyxInt>()?; // endpc
        }
        // end trash
        let upvalue_count = self.load::<SyxInt>()? as usize;
        for i in 0..upvalue_count {
            match proto.upvalues.get_mut(i) {
                Some(value) => value.name = self.load_string()?,
                None => return Err(format!("could not find upvalue index {}", i))
            }
        }
        Ok(())
    }

    fn load_function(&mut self, proto: &mut Proto, source: SyxString)
        -> SyxResult
    {
        let loaded_source = self.load_string()?;
        proto.source = match String::from_utf8({
            if loaded_source.len() != 0 {
                loaded_source
            } else {
                source
            }
        }) {
            Ok(val) => val,
            Err(_) => return Err(format!("invalid source name"))
        };
        proto.linedefined = self.load::<SyxInt>()?;
        proto.lastlinedefined = self.load::<SyxInt>()?;
        proto.numparams = self.load::<u8>()?;
        proto.is_vararg = self.load::<u8>()? != 0;
        proto.maxstacksize = self.load::<u8>()?;
        self.load_code(proto)?;
        self.load_constants(proto)?;
        self.load_upvalues(proto)?;
        self.load_protos(proto)?;
        self.load_debug(proto)?;
        Ok(())
    }
    
    fn check_size(&mut self, size: (usize, &'static str))
        -> SyxResult
    {
        if let Ok(bytecode_size) = self.load::<u8>() {
            self.assert_verification(bytecode_size == (size.0 as u8),
                                     format!("size mismatch: {}", size.1))
        } else {
            Ok(())
        }
    }

    fn check_literal(&mut self, value_impl: impl Into<Vec<u8>>,
                     err: impl ::std::fmt::Display) -> SyxResult
    {
        let value = value_impl.into();
        if let Ok(literal) = self.load_range(value.len()) {
            self.assert_verification(literal == value,
                                     format!("literal mismatch: {}", err))
        } else {
            Ok(())
        }
    }

    fn check_header(&mut self) -> SyxResult {
        self.check_literal(SYX_HEADER, "header")?;
        let bt = self.load::<u8>()?;
        self.assert_verification(bt == SYX_VERSION, "version mismatch")?;
        let bt = self.load::<u8>()?;
        self.assert_verification(bt == SYX_FORMAT, "format mismatch")?;
        self.check_literal(SYX_DATA, "load order verification")?;
        self.check_size(expand!(i32))?;
        self.check_size(expand!(usize))?;
        self.check_size(expand!(Instruction))?;
        self.check_size(expand!(SyxInteger))?;
        self.check_size(expand!(SyxNumber))?;
        let int: SyxInteger = self.load::<SyxInteger>()?;
        self.assert_verification(int == SYX_INT, "endianness mismatch")?;
        let float: SyxNumber = self.load::<SyxNumber>()?;
        self.assert_verification(float == SYX_NUM, "float format mismatch")?;
        Ok(())
    }

    fn load_chunk(&mut self, _lstate: state::SyxState)
        -> Result<Proto, String>
    {
        self.state = Some(state::SyxState {});
        // ::TODO:: ::XXX:: here is where i left off
        // cl->p
        self.check_header()?;
        let mut proto = Proto::new();
        let _upvals = self.load::<u8>()?;
        self.load_function(&mut proto, vec![])?;
        Ok(proto)
    }
}
