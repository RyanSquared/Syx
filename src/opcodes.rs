// Set up VM instructions
#![allow(dead_code)]

use super::object;

/* Word Format:
 * |0bIIIIII_AAAAAAAA_BBBBBBBBB_CCCCCCCCC| -> Instruction, A B C
 * |0bIIIIII_AAAAAAAA_BBBBBBBBB_BBBBBBBBB| -> Instruction, A Bx
 * |0bIIIIII_AAAAAAAA_SBBBBBBBB_BBBBBBBBB| -> Instruction, A sBx (same as above)
 * |0bIIIIII_AAAAAAAA_AAAAAAAAA_AAAAAAAAA| -> Instruction, Ax
 *
 * Almost every operation uses A B C unless the argument can take more than
 * what can be stored in the values, in which case Bx and Ax are used to store
 * more values than could be used normally. The value sBx is used regardless of
 * whether the space is needed for B, and the result is a signed integer.
 *
 * Consult the versioned lopcodes.h for more information.
 */

pub mod errors {
    error_chain! {
    }
}

const SIZE_OP: u32 = 6;

const SIZE_C: u32 = 9;
const SIZE_B: u32 = 9;
const SIZE_BX: u32 = SIZE_C + SIZE_B;
const SIZE_A: u32 = 8;
const SIZE_AX: u32 = SIZE_C + SIZE_B + SIZE_A;

const OFFSET_OP: u32 = (32 - SIZE_OP + 1);
const OFFSET_A: u32 = (32 - SIZE_OP - SIZE_A);
const OFFSET_B: u32 = (32 - SIZE_OP - SIZE_A - SIZE_B);
const OFFSET_C: u32 = 0;

const BITMASK_OP: u32 = (1 << SIZE_OP) - 1;
const BITMASK_A: u32 = (1 << SIZE_A) - 1;
const BITMASK_AX: u32 = (1 << SIZE_AX) - 1;
const BITMASK_B: u32 = (1 << SIZE_B) - 1;
const BITMASK_BX: u32 = (1 << SIZE_BX) - 1;
const BITMASK_C: u32 = (1 << SIZE_C) - 1;

#[repr(C)]
#[derive(Debug)]
pub enum OpCode {
Move, // A B R(A) := R(B)
LoadK, // A Bx R(A) := Kst(Bx)
LoadKX, // A  R(A) := Kst(extra arg)
LoadBool, // A B C R(A) := (Bool)B; if (C) pc++
LoadNil, // A B R(A), R(A+1), ..., R(A+B) := nil

GetUpval, // A B R(A) := UpValue[B]
GetTabUp, // A B C R(A) := UpValue[B][RK(C)]
GetTable, // A B C R(A) := R(B)[RK(C)]

SetTabUp, // A B C UpValue[A][RK(B)] := RK(C)
SetUpval, // A B UpValue[B] := R(A)
SetTable, // A B C R(A)[RK(B)] := RK(C)

NewTable, // A B C R(A) := {} (size = B,C)

// OP_SELF
SelfLoad, // A B C R(A+1) := R(B); R(A) := R(B)[RK(C)]

Add, // A B C R(A) := RK(B) + RK(C)
Sub, // A B C R(A) := RK(B) - RK(C)
Mul, // A B C R(A) := RK(B) * RK(C)
Mod, // A B C R(A) := RK(B) % RK(C)
Pow, // A B C R(A) := RK(B) ^ RK(C)
Div, // A B C R(A) := RK(B) / RK(C)
IDiv, // A B C R(A) := RK(B) // RK(C)
BAnd, // A B C R(A) := RK(B) & RK(C)
BOr, // A B C R(A) := RK(B) | RK(C)
BXOr, // A B C R(A) := RK(B) ~ RK(C)
Shl, // A B C R(A) := RK(B) << RK(C)
Shr, // A B C R(A) := RK(B) >> RK(C)
Unm, // A B R(A) := -R(B)
BNot, // A B R(A) := ~R(B)
Not, // A B R(A) := not R(B)
Len, // A B R(A) := length of R(B)

Concat, // A B C R(A) := R(B).. ... ..R(C)

Jmp, // A sBx pc+=sBx; if (A) close all upvalues >= R(A - 1)
Eq, // A B C if ((RK(B) == RK(C)) ~= A) then pc++
Lt, // A B C if ((RK(B) <  RK(C)) ~= A) then pc++
Le, // A B C if ((RK(B) <= RK(C)) ~= A) then pc++

Test, // A C if not (R(A) <=> C) then pc++
TestSet, // A B C if (R(B) <=> C) then R(A) := R(B) else pc++

Call, // A B C R(A), ... ,R(A+C-2) := R(A)(R(A+1), ... ,R(A+B-1))
TailCall, // A B C return R(A)(R(A+1), ... ,R(A+B-1))
Return, // A B return R(A), ... ,R(A+B-2) (see note)

ForLoop, // A sBx R(A)+=R(A+2); if R(A) <?= R(A+1) then { pc+=sBx; R(A+3)=R(A) }
ForPrep, // A sBx R(A)-=R(A+2); pc+=sBx

TForCall, // A C R(A+3), ... ,R(A+2+C) := R(A)(R(A+1), R(A+2));
TForLoop, // A sBx if R(A+1) ~= nil then { R(A)=R(A+1); pc += sBx }

SetList, // A B C R(A)[(C-1)*FPF+i] := R(A+i), 1 <= i <= B

Closure, // A Bx R(A) := closure(KPROTO[Bx])

VarArg, // A B R(A), R(A+1), ..., R(A+B-2) = vararg

ExtraArg // Ax extra (larger) argument for previous opcode
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

#[derive(Debug)]
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

impl Into<Instruction> for object::Instruction {
    fn into(self) -> Instruction {
        let opcode = (self >> (32 - SIZE_OP + 1)) & BITMASK_OP; // OpCode is 6 bits
        println!("opcode: {} {}", opcode, OFFSET_B);
        let _enum = unsafe { ::std::mem::transmute::<object::Instruction, OpCode>(opcode) };
        match _enum {
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
                a: ((self >> OFFSET_A) & BITMASK_A) as u8,
                b: ((self >> OFFSET_B) & BITMASK_B) as u16,
                c: ((self >> OFFSET_C) & BITMASK_C) as u16,
            },
            | OpCode::LoadK
            | OpCode::Closure
            => Instruction::ABx {
                instruction: _enum,
                a: ((self >> OFFSET_A) & BITMASK_A) as u8,
                bx: ((self >> OFFSET_B) & BITMASK_BX) as u32,
            },
            | OpCode::Jmp
            | OpCode::ForLoop
            | OpCode::ForPrep
            | OpCode::TForLoop
            => Instruction::AsBx {
                instruction: _enum,
                a: ((self >> OFFSET_A) & BITMASK_A) as u8,
                sbx: ((self >> OFFSET_B) & BITMASK_BX) as i32,
            },
            | OpCode::ExtraArg
            => Instruction::Ax {
                instruction: _enum,
                ax: ((self >> OFFSET_A) & BITMASK_AX) as u32,
            },
        }
    }
}
