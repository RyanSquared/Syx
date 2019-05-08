#[macro_export]
macro_rules! try_from_enum {
    (
        $name:ident | $err:ident = $error_value:expr =>
        $($variant:ident = $value:expr),+
    )=> {
        #[repr(C)]
        #[derive(Debug, PartialEq)]
        pub enum $name {
        $(
            $variant,
        )+
        }

        impl std::convert::TryFrom<u8> for $name {
            type Error = $err;
            fn try_from(value: u8) -> Result<$name> {
                match value {
                    $(
                        $value => Ok(OpCode::$variant),
                     )+
                    _ => Err($error_value)
                }
            }
        }
    }
}
