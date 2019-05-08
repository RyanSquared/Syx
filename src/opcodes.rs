// Set up VM instructions
#![allow(dead_code)]

use std::convert::{TryFrom, TryInto};

use super::object;

pub type Word = u32;

/* Word Format:
 * |0bBBBBBBBBB_CCCCCCCCC_AAAAAAAA_IIIIII| -> B, C, A, Instruction
 * |0bBBBBBBBBB_BBBBBBBBB_AAAAAAAA_IIIIII| -> Bx, A, Instruction
 * |0bSBBBBBBBB_BBBBBBBBB_AAAAAAAA_IIIIII| -> sBx, A, Instruction
 * |0bAAAAAAAAA_AAAAAAAAA_AAAAAAAA_IIIIII| -> Ax, Instruction
 *
 * Bits are "right side lowest, left side highest"
 *
 * Almost every operation uses A B C unless the argument can take more than
 * what can be stored in the values, in which case Bx and Ax are used to store
 * more values than could be used normally. The value sBx is used regardless of
 * whether the space is needed for B, and the result is a signed integer.
 *
 * Consult the versioned lopcodes.h for more information.
 */

use super::errors::*;

#[macro_use]
use super::try_from_enum;

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

const BITMASK_IS_RK: u32 = 1 << (SIZE_B - 1); // match significant bit in B

// Is constant: C & BITMASK_IS_RK == 1
// Register number: (n as u32) & ~BITMASK_IS_RK

try_from_enum! { OpCode | Error = ErrorKind::InvalidOpCode.into() =>
Move = 0, // A B R(A) := R(B)
LoadK = 1, // A Bx R(A) := Kst(Bx)
LoadKX = 2, // A  R(A) := Kst(extra arg)
LoadBool = 3, // A B C R(A) := (Bool)B; if (C) pc++
LoadNil = 4, // A B R(A), R(A+1), ..., R(A+B) := nil

GetUpval = 5, // A B R(A) := UpValue[B]
GetTabUp = 6, // A B C R(A) := UpValue[B][RK(C)]
GetTable = 7, // A B C R(A) := R(B)[RK(C)]

SetTabUp = 8, // A B C UpValue[A][RK(B)] := RK(C)
SetUpval = 9, // A B UpValue[B] := R(A)
SetTable = 10, // A B C R(A)[RK(B)] := RK(C)

NewTable = 11, // A B C R(A) := {} (size = B,C)

// OP_SELF
SelfLoad = 12, // A B C R(A+1) := R(B); R(A) := R(B)[RK(C)]

Add = 13, // A B C R(A) := RK(B) + RK(C)
Sub = 14, // A B C R(A) := RK(B) - RK(C)
Mul = 15, // A B C R(A) := RK(B) * RK(C)
Mod = 16, // A B C R(A) := RK(B) % RK(C)
Pow = 17, // A B C R(A) := RK(B) ^ RK(C)
Div = 18, // A B C R(A) := RK(B) / RK(C)
IDiv = 19, // A B C R(A) := RK(B) // RK(C)
BAnd = 20, // A B C R(A) := RK(B) & RK(C)
BOr = 21, // A B C R(A) := RK(B) | RK(C)
BXOr = 22, // A B C R(A) := RK(B) ~ RK(C)
Shl = 23, // A B C R(A) := RK(B) << RK(C)
Shr = 24, // A B C R(A) := RK(B) >> RK(C)
Unm = 25, // A B R(A) := -R(B)
BNot = 26, // A B R(A) := ~R(B)
Not = 27, // A B R(A) := not R(B)
Len = 28, // A B R(A) := length of R(B)

Concat = 29, // A B C R(A) := R(B).. ... ..R(C)

Jmp = 30, // A sBx pc+=sBx; if (A) close all upvalues >= R(A - 1)
Eq = 31, // A B C if ((RK(B) == RK(C)) ~= A) then pc++
Lt = 32, // A B C if ((RK(B) <  RK(C)) ~= A) then pc++
Le = 33, // A B C if ((RK(B) <= RK(C)) ~= A) then pc++

Test = 34, // A C if not (R(A) <=> C) then pc++
TestSet = 35, // A B C if (R(B) <=> C) then R(A) := R(B) else pc++

Call = 36, // A B C R(A), ... ,R(A+C-2) := R(A)(R(A+1), ... ,R(A+B-1))
TailCall = 37, // A B C return R(A)(R(A+1), ... ,R(A+B-1))
Return = 38, // A B return R(A), ... ,R(A+B-2) (see note)

ForLoop = 39, // A sBx R(A)+=R(A+2); if R(A) <?= R(A+1) then { pc+=sBx; R(A+3)=R(A) }
ForPrep = 40, // A sBx R(A)-=R(A+2); pc+=sBx

TForCall = 41, // A C R(A+3), ... ,R(A+2+C) := R(A)(R(A+1), R(A+2));
TForLoop = 42, // A sBx if R(A+1) ~= nil then { R(A)=R(A+1); pc += sBx }

SetList = 43, // A B C R(A)[(C-1)*FPF+i] := R(A+i), 1 <= i <= B

Closure = 44, // A Bx R(A) := closure(KPROTO[Bx])

VarArg = 45, // A B R(A), R(A+1), ..., R(A+B-2) = vararg

ExtraArg = 46 // Ax extra (larger) argument for previous opcode
}

/*===========================================================================
  Notes:
  (*) In OP_CALL, if (B == 0) then B = top. If (C == 0), then 'top' is
  set to last_result+1, so next open instruction (OP_CALL, OP_RETURN,
  OP_SETLIST) may use 'top'.

  (*) In OP_VARARG, if (B == 0) then use actual number of varargs and
  set top (like in OP_CALL with C == 0).

  (*) In OP_RETURN, if (B == 0) then return up to 'top'.

  (*) In OP_SETLIST, if (B == 0) then B = 'top'; if (C == 0) then next
  'instruction' is EXTRAARG(real C).

  (*) In OP_LOADKX, the next 'instruction' is always EXTRAARG.

  (*) For comparisons, A specifies what condition the test should accept
  (true or false).

  (*) All 'skips' (pc++) assume that next instruction is a jump.

===========================================================================*/

#[derive(Debug, PartialEq)]
pub enum Instruction {
    ABC {
        instruction: OpCode,
        a: u8, // 8
        b: u16, // 9
        c: u16, // 9
    },
    ABx {
        instruction: OpCode,
        a: u8, // 8
        bx: u32, // 18
    },
    AsBx {
        instruction: OpCode,
        a: u8, // 8
        sbx: i32, // 1 + 17
    },
    Ax {
        instruction: OpCode,
        ax: u32, // 26
    },
}

impl TryFrom<Word> for Instruction {
    type Error = Error;

    fn try_from(instr: Word) -> Result<Instruction> {
        let opcode = (instr >> OFFSET_OP) & BITMASK_OP;
        let _enum: OpCode = OpCode::try_from(opcode as u8)?;
        Ok(match _enum {
            | OpCode::Move     // A B
            | OpCode::LoadKX   // A <extra arg>
            | OpCode::LoadBool // A B C
            | OpCode::LoadNil  // A B
            | OpCode::GetUpval // A B
            | OpCode::GetTabUp // A B C
            | OpCode::GetTable // A B C
            | OpCode::SetTabUp // A B C
            | OpCode::SetUpval // A B
            | OpCode::SetTable // A B C
            | OpCode::NewTable // A B C
            | OpCode::SelfLoad // A B C
            | OpCode::Add  // A B C
            | OpCode::Sub  // A B C
            | OpCode::Mul  // A B C
            | OpCode::Mod  // A B C
            | OpCode::Pow  // A B C
            | OpCode::Div  // A B C
            | OpCode::IDiv // A B C
            | OpCode::BAnd // A B C
            | OpCode::BOr  // A B C
            | OpCode::BXOr // A B C
            | OpCode::Shl  // A B C
            | OpCode::Shr  // A B C
            | OpCode::Unm  // A B
            | OpCode::BNot // A B
            | OpCode::Not  // A B
            | OpCode::Len  // A B
            | OpCode::Concat   // A B C
            | OpCode::Eq       // A B C
            | OpCode::Lt       // A B C
            | OpCode::Le       // A B C
            | OpCode::Test     // A _ C
            | OpCode::TestSet  // A B C
            | OpCode::Call     // A B C
            | OpCode::TailCall // A B C
            | OpCode::Return   // A B
            | OpCode::TForCall // A _ C
            | OpCode::SetList  // A B C
            | OpCode::VarArg   // A B
            => Instruction::ABC {
                instruction: _enum,
                a: ((instr >> OFFSET_A) & BITMASK_A) as u8,
                b: ((instr >> OFFSET_B) & BITMASK_B) as u16,
                c: ((instr >> OFFSET_C) & BITMASK_C) as u16,
            },
            | OpCode::LoadK
            | OpCode::Closure
            => Instruction::ABx {
                instruction: _enum,
                a: ((instr >> OFFSET_A) & BITMASK_A) as u8,
                bx: ((instr >> OFFSET_B) & BITMASK_BX) as u32,
            },
            | OpCode::Jmp
            | OpCode::ForLoop
            | OpCode::ForPrep
            | OpCode::TForLoop
            => Instruction::AsBx {
                instruction: _enum,
                a: ((instr >> OFFSET_A) & BITMASK_A) as u8,
                sbx: ((instr >> OFFSET_B) & BITMASK_BX) as i32,
            },
            | OpCode::ExtraArg
            => Instruction::Ax {
                instruction: _enum,
                ax: ((instr >> OFFSET_A) & BITMASK_AX) as u32,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /*
     * Tests are written in the format "test_<format>"
     * format refers to the layout of the items used. for instance,
     * bcai uses the "b, c, a, i" layout, which is the "standard" opcode.
     */

    #[test]
    fn test_bcai() {
        {
            let instr: Instruction = 0b000100000_000000000_00000100_000000u32.try_into().unwrap();
            let instr_comp = Instruction::ABC {
                instruction: OpCode::Move,
                a: 0b00000100,
                b: 0b000100000,
                c: 0b000000000,
            };
            assert_eq!(instr, instr_comp);
        }
        {
            let instr: Instruction = 0b000100100_000000000_10000101_000010u32.try_into().unwrap();
            let instr_comp = Instruction::ABC {
                instruction: OpCode::LoadKX,
                a: 0b10000101,
                b: 0b000100100,
                c: 0b000000000,
            };
            assert_eq!(instr, instr_comp);
        }
    }
}
