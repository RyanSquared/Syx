extern crate conv;
use self::conv::{TryFrom, TryInto};

pub type Instruction = u32;
pub type SyxInt = i32; // because Lua hates me
pub type SyxInteger = i64;
pub type SyxNumber = f64;
pub type SyxString = Vec<u8>;

use super::errors::*;

#[derive(Debug)]
pub enum SyxType {
    TNIL,
    TBOOLEAN,
    TLIGHTUSERDATA,
    TNUMBER,
    TSTRING,
    TTABLE,
    TFUNCTION,
    TUSERDATA,
    TTHREAD,
    TNUMFLT,
    TNUMINT,
    TSHRSTR,
    TLNGSTR,
}

pub const SYX_TNUMFLT: u8 = (SyxType::TNUMBER as u8) | (0 << 4);
pub const SYX_TNUMINT: u8 = (SyxType::TNUMBER as u8) | (1 << 4);

pub const SYX_TSHRSTR: u8 = (SyxType::TSTRING as u8) | (0 << 4);
pub const SYX_TLNGSTR: u8 = (SyxType::TSTRING as u8) | (1 << 4);

impl TryFrom<u8> for SyxType {
    type Err = Error;

    fn try_from(value: u8) -> Result<SyxType> {
        match value {
            SYX_TNUMFLT => Ok(SyxType::TNUMFLT),
            SYX_TNUMINT => Ok(SyxType::TNUMINT),
            SYX_TSHRSTR => Ok(SyxType::TSHRSTR),
            SYX_TLNGSTR => Ok(SyxType::TLNGSTR),
            0 => Ok(SyxType::TNIL),
            1 => Ok(SyxType::TBOOLEAN),
            2 => Ok(SyxType::TLIGHTUSERDATA),
            // 3 => Ok(SyxType::TNUMBER),
            // 4 => Ok(SyxType::TSTRING),
            5 => Ok(SyxType::TTABLE),
            6 => Ok(SyxType::TFUNCTION),
            7 => Ok(SyxType::TUSERDATA),
            8 => Ok(SyxType::TTHREAD),
            _ => Err(ErrorKind::InvalidType(value).into()),
        }
    }
}

pub enum SyxValue {
    Bool(bool),
    Number(SyxNumber),
    Integer(SyxInteger),
    String(SyxString),
    Nil,
}

pub struct Upvalue {
    pub name: SyxString,
    pub instack: u8, // ::TODO:: bool?
    pub idx: u8,
}

pub struct LocVar {
    pub varname: SyxString, // name of local variable
    pub startpc: SyxInt,    // point where variable is alive
    pub endpc: SyxInt,      // point where variable is dead
}

pub struct Proto {
    // Function Prototypes
    pub numparams: u8,       // number of fixed parameters (does not include vararg)
    pub is_vararg: bool,     // should be obvious
    pub maxstacksize: u8,    // amount of registers needed
    pub linedefined: SyxInt, // debug
    pub lastlinedefined: SyxInt, // debug
    pub constants: Vec<SyxValue>, // constants used by the function
    pub ip: i32,             // instruction pointer, used for instruction index
    pub instructions: Vec<Instruction>, // function opcodes
    pub protos: Vec<Proto>,  // functions defined in this function
    pub lineinfo: Vec<i32>,  // map from opcode to source lines ::TODO:: what?
    pub upvalues: Vec<Upvalue>, // upvalue information
    pub locvars: Vec<LocVar>, // local variables
    pub source: String,
}

impl Proto {
    pub fn new() -> Proto {
        Proto {
            numparams: 0,
            is_vararg: false,
            maxstacksize: 0,
            linedefined: 0,
            lastlinedefined: 0,
            constants: Vec::new(),
            ip: 0,
            instructions: Vec::new(),
            protos: Vec::new(),
            lineinfo: Vec::new(),
            upvalues: Vec::new(),
            locvars: Vec::new(),
            source: "".to_owned(),
        }
    }
}

// typedef struct Proto {
//   CommonHeader;
//   lu_byte numparams;  /* number of fixed parameters */
//   lu_byte is_vararg;
//   lu_byte maxstacksize;  /* number of registers needed by this function */
//   int sizeupvalues;  /* size of 'upvalues' */
//   int sizek;  /* size of 'k' */
//   int sizecode;
//   int sizelineinfo;
//   int sizep;  /* size of 'p' */
//   int sizelocvars;
//   int linedefined;  /* debug information  */
//   int lastlinedefined;  /* debug information  */
//   TValue *k;  /* constants used by the function */
//   Instruction *code;  /* opcodes */
//   struct Proto **p;  /* functions defined inside the function */
//   int *lineinfo;  /* map from opcodes to source lines (debug information) */
//   LocVar *locvars;  /* information about local variables (debug information) */
//   Upvaldesc *upvalues;  /* upvalue information */
//   struct LClosure *cache;  /* last-created closure with this prototype */
//   TString  *source;  /* used for debug information */
//   GCObject *gclist;
// } Proto;
