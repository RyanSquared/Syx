// Set up VM instructions
#![allow(dead_code)]

#[macro_use]
use syx_codegen::bytecode;

use std::convert::{TryFrom, TryInto};

use super::object;

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

bytecode! { Instruction | OpCode | Error = ErrorKind::InvalidOpCode.into() =>
    Move: AB = Register, Register; // R(A) := R(B)
    LoadK: ABx = Register, Constant; // R(A) = Kst(Bx)
    LoadKX: A = Register; // R(A) = Kst(extra arg); see ExtraArg
    LoadBool: ABC = Register, Bool, Integer; // R(A) := (Bool)B; if C pc++
    LoadNil: AB = Register, Integer; // R(A .. A+B) := nil

    GetUpval: AB = Register, UpValue; // R(A) = UpValue[B]
    GetTabUp: ABC = Register, UpValue, RegisterConstant; // R(A) := UpValue[B][RK(C)]
    GetTable: ABC = Register, Register, RegisterConstant; // R(A) := R(B)[RK(C)]

    SetTabUp: ABC = UpValue, RegisterConstant, RegisterConstant; // UpValue[A][RK(B)] = RK(C)
    SetUpval: AB = UpValue, Register; // UpValue[B] := R(A)
    SetTable: ABC = Register, RegisterConstant, RegisterConstant; // R(A)[RK(B)] := RK(C)

    NewTable: ABC = Register, Integer, Integer; // R(A) := {} (size: array = B, hash = C)

    // OP_SELF
    // move the table to the next item of the registers, assign A to the "method"
    SelfLoad: ABC = Register, Register, RegisterConstant; // R(A+1) := R(B); R(A) = R(B)[RK(C)]

    Add: ABC = Register, RegisterConstant, RegisterConstant; // R(A) = RK(B) + RK(C)
    Sub: ABC = Register, RegisterConstant, RegisterConstant; // R(A) = RK(B) - RK(C)
    Mul: ABC = Register, RegisterConstant, RegisterConstant; // R(A) = RK(B) * RK(C)
    Mod: ABC = Register, RegisterConstant, RegisterConstant; // R(A) = RK(B) % RK(C)
    Pow: ABC = Register, RegisterConstant, RegisterConstant; // R(A) = RK(B) ^ RK(C)
    Div: ABC = Register, RegisterConstant, RegisterConstant; // R(A) = RK(B) / RK(C)
    IDiv: ABC = Register, RegisterConstant, RegisterConstant; // R(A) = RK(B) // RK(C)
    BAnd: ABC = Register, RegisterConstant, RegisterConstant; // R(A) = RK(B) & RK(C)
    BOr: ABC = Register, RegisterConstant, RegisterConstant; // R(A) = RK(B) | RK(C)
    BXOr: ABC = Register, RegisterConstant, RegisterConstant; // R(A) = RK(B) ~ RK(C)
    Shl: ABC = Register, RegisterConstant, RegisterConstant; // R(A) = RK(B) << RK(C)
    Shr: ABC = Register, RegisterConstant, RegisterConstant; // R(A) = RK(B) >> RK(C)
    Unm: AB = Register, RegisterConstant; // R(A) = RK(B)  RK(C)
    BNot: AB = Register, RegisterConstant; // R(A) = RK(B) + RK(C)
    Not: AB = Register, RegisterConstant; // R(A) = RK(B) + RK(C)
    Len: AB = Register, RegisterConstant; // R(A) = RK(B) + RK(C)

    Concat: ABC = Register, Register, Register; // R(A) := R(B).. ... ..R(C)

    Jmp: AsBx = Integer, SInteger; // pc += sBx; if A != 0 close upvalues >= R(A - 1)
    Eq: ABC = Integer, RegisterConstant, RegisterConstant; // if ((RK(B) == RK(C)) ~= A) pc++
    Lt: ABC = Integer, RegisterConstant, RegisterConstant; // if ((RK(B) <  RK(C)) ~= A) pc++
    Le: ABC = Integer, RegisterConstant, RegisterConstant; // if ((RK(B) <= RK(C)) ~= A) pc++

    Test: ABC = Register, Register, Integer; // if !(R(A) != C) pc++
    TestSet: ABC = Register, Register, Integer; // !(R(B) != C) ? (R(A) := R(B)) : pc++

    // R(A), ... R(A+C-2) := R(A)(R(A+1), ..., R(A+B-1))
    // Set register A through A+C-2 to return values of calling A with the
    // values of A+1 until A+b-1
    Call: ABC = Register, Integer, Integer; 
    // return R(A)(R(A+1), ..., R(A+B-1))
    // Call the function A with values A+1 until A+B-1, and leave the return
    // values on top of the stack(?)
    // ::TODO:: is C used?
    TailCall: ABC = Register, Integer, Integer;
    Return: AB = Register, Integer; // return R(A), ... ,R(A+B-2) (see note)

    ForLoop: AsBx = Register, SInteger; // R(A)+=R(A+2); if R(A) <?= R(A+1) { pc += sBx; R(A+3)=R(A) }
    ForPrep: AsBx = Register, SInteger; // R(A)-=R(A+2); pc += sBx

    // set register A+3 through A+2+C to return values of call R(A) for R(A+1) and R(A+2)
    // i think this means you can only have two arguments to an iterator?
    TForCall: ABC = Register, Integer, Integer; // R(A+3), ... ,R(A+2+C) := R(A)(R(A+1), R(A+2))
    TForLoop: AsBx = Register, SInteger; // if R(A+1) ~= nil then { R(A)=R(A+!); pc += sBx }

    // ::TODO:: ask mailing list, what is this??
    SetList: ABC = Register, Integer, Integer; // R(A)[(C-1)*FPF+i] := R(A+i), 1 <= i <= B

    Closure: ABx = Register, Integer; // R(A) := closure(prototypes[Bx])
    
    VarArg: AB = Register, Integer; // R(A+1), ..., R(A+B-2) = vararg

    ExtraArg: Ax = Integer; // ExtraArg = Ax
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

    #[test]
    fn test_bxai() {
        {
            println!("{}", OFFSET_BX);
            let instr: Instruction = 0b000000000000000001_10000101_000001u32.try_into().unwrap();
            let instr_comp = Instruction::ABx {
                instruction: OpCode::LoadK,
                a: 0b10000101,
                bx: 0b000000000000000001,
            };
            assert_eq!(instr, instr_comp);
        }
    }
}
