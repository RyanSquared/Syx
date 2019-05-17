#![recursion_limit="1024"]

extern crate proc_macro;
use proc_macro::TokenStream;

extern crate syn;
use syn::parse::{Parse, ParseStream, Result, Error};
use syn::punctuated::{Punctuated, Pair, IntoIter};
use syn::{parse_macro_input, Expr, Ident, Token, Type, Visibility};
use syn::spanned::Spanned;
use syn::export::Span;

extern crate quote;
use quote::quote;

enum OpCodeType {
    ABC(Ident, Ident, Ident),
    AB(Ident, Ident),
    A(Ident),
    ABx(Ident, Ident),
    AsBx(Ident, Ident),
    Ax(Ident),
}

struct OpCodeContainer(Ident, OpCodeType);

struct OpCodeParse {
    opcode_name: Ident,
    error_name: Ident,
    error_expr: Expr,
    list: Vec<OpCodeContainer>
}

const INVALID_FORMAT: &str = "expected one of `ABC`, `AB`, `A`, `ABx`, `AsBx`, `Ax`";
const ALLOWED_RHS: [&str; 7] = ["Register", "Constant", "RegisterConstant",
                                "Integer", "SInteger", "Bool", "UpValue"];
const INVALID_RHS: &str = "expected one of `Register`, `Constant`, \
                           `RegisterConstant`, `Integer`, `SInteger`, `Bool`, `UpValue`";

macro_rules! bad_count_rhs {
    () => {"expected {} arguments, got {}"}
}

fn get_arg(args: &mut IntoIter<Ident, Token![,]>, count: usize, max: u8, error_span: Span)
        -> syn::parse::Result<Ident> {
    let item = args.next()
        .ok_or(format!(bad_count_rhs!(), count, max))
        .map_err(|y| Error::new(error_span, y))?;
    if !ALLOWED_RHS.contains(&item.to_string().as_str()) {
        return Err(Error::new(item.span(), INVALID_RHS));
    }
    Ok(item)
}

impl Parse for OpCodeParse {
    fn parse(input: ParseStream) -> Result<Self> {
        let opcode_name = input.parse::<Ident>()?;
        input.parse::<Token![|]>()?;
        let error_name = input.parse::<Ident>()?;
        input.parse::<Token![=]>()?;
        let error_expr = input.parse::<Expr>()?;
        input.parse::<Token![=]>()?;
        input.parse::<Token![>]>()?;
        let mut output = Vec::new();
        while !input.is_empty() {
            // match something like: `Move: AB = Register, Register`
            // as well as: `MoveK: ABx = Register, Constant`
            // valid values for RHS are:
            // Register, Constant, RegisterConstant, Integer, Bool, UpValue
            let current_name: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            let format: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let arg_types_punct = Punctuated::<Ident, Token![,]>::parse_separated_nonempty(input)?;
            let arg_types_span = arg_types_punct.span();
            let mut arg_types = arg_types_punct.into_iter();
            let arg_count = arg_types.len();
            let mut expected_arg_count = 0;
            match format.to_string().as_str() {
                "ABC" => {
                    expected_arg_count = 3;
                    let first = get_arg(&mut arg_types, arg_count, expected_arg_count, arg_types_span)?;
                    let second = get_arg(&mut arg_types, arg_count, expected_arg_count, arg_types_span)?;
                    let third = get_arg(&mut arg_types, arg_count, expected_arg_count, arg_types_span)?;
                    output.push(OpCodeContainer(current_name, OpCodeType::ABC(first, second, third)))
                },
                "AB" => {
                    expected_arg_count = 2;
                    let first = get_arg(&mut arg_types, arg_count, expected_arg_count, arg_types_span)?;
                    let second = get_arg(&mut arg_types, arg_count, expected_arg_count, arg_types_span)?;
                    output.push(OpCodeContainer(current_name, OpCodeType::AB(first, second)))
                },
                "A" => {
                    expected_arg_count = 1;
                    let first = get_arg(&mut arg_types, arg_count, expected_arg_count, arg_types_span)?;
                    output.push(OpCodeContainer(current_name, OpCodeType::A(first)))
                },
                "ABx" => {
                    expected_arg_count = 2;
                    let first = get_arg(&mut arg_types, arg_count, expected_arg_count, arg_types_span)?;
                    let second = get_arg(&mut arg_types, arg_count, expected_arg_count, arg_types_span)?;
                    output.push(OpCodeContainer(current_name, OpCodeType::ABx(first, second)))
                },
                "AsBx" => {
                    expected_arg_count = 2;
                    let first = get_arg(&mut arg_types, arg_count, expected_arg_count, arg_types_span)?;
                    let second = get_arg(&mut arg_types, arg_count, expected_arg_count, arg_types_span)?;
                    output.push(OpCodeContainer(current_name, OpCodeType::AsBx(first, second)))
                },
                "Ax" => {
                    expected_arg_count = 1;
                    let first = get_arg(&mut arg_types, arg_count, expected_arg_count, arg_types_span)?;
                    output.push(OpCodeContainer(current_name, OpCodeType::Ax(first)))
                },
                _ => return Err(Error::new(format.span(), INVALID_FORMAT))
            }
            if let Some(arg) = arg_types.next() {
                return Err(Error::new(arg.span(),
                                      format!(bad_count_rhs!(), expected_arg_count, arg_count)))
            }
            input.parse::<Token![;]>()?;
        }
        Ok(OpCodeParse {
            opcode_name: opcode_name,
            error_name: error_name,
            error_expr: error_expr,
            list: output,
        })
    }
}

#[proc_macro]
pub fn bytecode(input: TokenStream) -> TokenStream {
    let OpCodeParse {
        opcode_name: opcode_name,
        error_name: error_name,
        error_expr: error_expr,
        list: opcode_list,
    } = parse_macro_input!(input as OpCodeParse);

    let number = 0u8..255u8;

    let opcode_variant: Vec<_> = opcode_list
        .iter()
        .map(|x| x.0.clone())
        .collect();

    let opcode_variant_map: Vec<_> = opcode_list
        .iter()
        .map(|x| x.0.clone())
        .collect();

    let opcode_name_repeat = ::std::iter::repeat(opcode_name.clone());

    let result = quote! {
        const SIZE_OP: u32 = 6;

        const SIZE_C: u32 = 9;
        const SIZE_B: u32 = 9;
        const SIZE_BX: u32 = SIZE_C + SIZE_B;
        const SIZE_A: u32 = 8;
        const SIZE_AX: u32 = SIZE_C + SIZE_B + SIZE_A;

        const OFFSET_OP: u32 = 0;
        const OFFSET_A: u32 = (OFFSET_OP + SIZE_OP);
        const OFFSET_C: u32 = (OFFSET_A + SIZE_A);
        const OFFSET_B: u32 = (OFFSET_C + SIZE_C);

        const OFFSET_BX: u32 = OFFSET_C;
        const OFFSET_AX: u32 = OFFSET_A;

        const BITMASK_OP: u32 = (1 << SIZE_OP) - 1;
        const BITMASK_A: u32 = (1 << SIZE_A) - 1;
        const BITMASK_AX: u32 = (1 << SIZE_AX) - 1;
        const BITMASK_B: u32 = (1 << SIZE_B) - 1;
        const BITMASK_BX: u32 = (1 << SIZE_BX) - 1;
        const BITMASK_C: u32 = (1 << SIZE_C) - 1;

        const BITMASK_IS_RK: u32 = 1 << (SIZE_B - 1);

        // Is constant: C & BITMASK_IS_RK == 1
        // Register number: (n as u32) & ~BITMASK_IS_RK

        enum Argument {
            Register(u32),
            Constant(u32),
            RegisterConstant(u32),
            SInteger(i32),
            Integer(u32),
            Bool(u32),
            UpValue(u32),
        }

        impl ::std::fmt::Debug for Argument {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                match self {
                    Argument::Register(n) => write!(f, "Register({})", n),
                    Argument::Constant(n) => write!(f, "Constant({})", n),
                    Argument::RegisterConstant(n) => {
                        write!(f, "RegisterConstant(");
                        if n & BITMASK_IS_RK == 0 {
                            write!(f, "Register({}))", n);
                        } else {
                            write!(f, "Constant({}))", n & !BITMASK_IS_RK);
                        }
                        Ok(())
                    },
                    _ => unimplemented!()
                }
            }
        }

        #[derive(Debug, Eq, PartialEq)]
        pub enum #opcode_name {
        #(
            #opcode_variant,
        )*
        }

        impl ::std::convert::TryFrom<u8> for #opcode_name {
            type Error = #error_name;
            fn try_from(value: u8) -> Result<#opcode_name> {
                match value {
                #(
                    #number => Ok(#opcode_name_repeat::#opcode_variant_map),
                 )*
                _ => Err(#error_expr)
                }
            }
        }
    };

    result.into()
}
