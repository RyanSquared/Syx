use super::object::{SyxInteger, SyxNumber};

// Syx VM version information

pub const SYX_VERSION_MAJOR: u8 = 0x5;
pub const SYX_VERSION_MINOR: u8 = 0x3;

// Verification information

// <ESC>Lua, can't have <ESC> in source, so it's a useful escape character
pub const SYX_HEADER: &[u8] = b"\x1bLua";

pub const SYX_DATA: &[u8] = b"\x19\x93\r\n\x1a\n";
pub const SYX_VERSION: u8 = SYX_VERSION_MAJOR * 16 + SYX_VERSION_MINOR;
pub const SYX_FORMAT: u8 = 0; // official PUC-Rio format
pub const SYX_INT: SyxInteger = 0x5678;
pub const SYX_NUM: SyxNumber = (370.5f32 as SyxNumber);
