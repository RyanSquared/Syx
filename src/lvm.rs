// Set up VM instructions
#![allow(dead_code)]

const SIZE_C: u8 = 9;
const SIZE_B: u8 = 9;
const SIZE_BX: u8 = SIZE_C + SIZE_B;
const SIZE_A: u8 = 8;
const SIZE_AX: u8 = SIZE_C + SIZE_B + SIZE_A;

const SIZE_OP: u8 = 6;

pub enum Instruction {
    ABC {
        instruction: u8, // 6
        a: u8, // 8
        b: u16, // 9
        c: u16, // 9
    },
    ABx {
        instruction: u8, // 6
        a: u8, // 8
        bx: u32, // 18
    },
    AsBx {
        instruction: u8, // 6
        a: u8, // 8
        sbx: i32, // 1 + 17
    },
    Ax {
        instruction: u8, // 6
        ax: u32, // 26
    },
}

#[repr(C)]
enum OpCode {
OP_MOVE, // A B R(A) := R(B)
OP_LOADK, // A Bx R(A) := Kst(Bx)
OP_LOADKX, // A  R(A) := Kst(extra arg)
OP_LOADBOOL, // A B C R(A) := (Bool)B; if (C) pc++
OP_LOADNIL, // A B R(A), R(A+1), ..., R(A+B) := nil
OP_GETUPVAL, // A B R(A) := UpValue[B]

OP_GETTABUP, // A B C R(A) := UpValue[B][RK(C)]
OP_GETTABLE, // A B C R(A) := R(B)[RK(C)]

OP_SETTABUP, // A B C UpValue[A][RK(B)] := RK(C)
OP_SETUPVAL, // A B UpValue[B] := R(A)
OP_SETTABLE, // A B C R(A)[RK(B)] := RK(C)

OP_NEWTABLE, // A B C R(A) := {} (size = B,C)

OP_SELF, // A B C R(A+1) := R(B); R(A) := R(B)[RK(C)]

OP_ADD, // A B C R(A) := RK(B) + RK(C)
OP_SUB, // A B C R(A) := RK(B) - RK(C)
OP_MUL, // A B C R(A) := RK(B) * RK(C)
OP_MOD, // A B C R(A) := RK(B) % RK(C)
OP_POW, // A B C R(A) := RK(B) ^ RK(C)
OP_DIV, // A B C R(A) := RK(B) / RK(C)
OP_IDIV, // A B C R(A) := RK(B) // RK(C)
OP_BAND, // A B C R(A) := RK(B) & RK(C)
OP_BOR, // A B C R(A) := RK(B) | RK(C)
OP_BXOR, // A B C R(A) := RK(B) ~ RK(C)
OP_SHL, // A B C R(A) := RK(B) << RK(C)
OP_SHR, // A B C R(A) := RK(B) >> RK(C)
OP_UNM, // A B R(A) := -R(B)
OP_BNOT, // A B R(A) := ~R(B)
OP_NOT, // A B R(A) := not R(B) 
OP_LEN, // A B R(A) := length of R(B)

OP_CONCAT, // A B C R(A) := R(B).. ... ..R(C)

OP_JMP, // A sBx pc+=sBx; if (A) close all upvalues >= R(A - 1)
OP_EQ, // A B C if ((RK(B) == RK(C)) ~= A) then pc++
OP_LT, // A B C if ((RK(B) <  RK(C)) ~= A) then pc++
OP_LE, // A B C if ((RK(B) <= RK(C)) ~= A) then pc++

OP_TEST, // A C if not (R(A) <=> C) then pc++
OP_TESTSET, // A B C if (R(B) <=> C) then R(A) := R(B) else pc++

OP_CALL, // A B C R(A), ... ,R(A+C-2) := R(A)(R(A+1), ... ,R(A+B-1))
OP_TAILCALL, // A B C return R(A)(R(A+1), ... ,R(A+B-1))
OP_RETURN, // A B return R(A), ... ,R(A+B-2) (see note)

OP_FORLOOP, // A sBx R(A)+=R(A+2); if R(A) <?= R(A+1) then { pc+=sBx; R(A+3)=R(A) }
OP_FORPREP, // A sBx R(A)-=R(A+2); pc+=sBx

OP_TFORCALL, // A C R(A+3), ... ,R(A+2+C) := R(A)(R(A+1), R(A+2));
OP_TFORLOOP, // A sBx if R(A+1) ~= nil then { R(A)=R(A+1); pc += sBx }

OP_SETLIST, // A B C R(A)[(C-1)*FPF+i] := R(A+i), 1 <= i <= B

OP_CLOSURE, // A Bx R(A) := closure(KPROTO[Bx])

OP_VARARG, // A B R(A), R(A+1), ..., R(A+B-2) = vararg

OP_EXTRAARG // Ax extra (larger) argument for previous opcode
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

impl Into<Instruction> for u32 {
    fn into(self) -> Instruction {
        let _enum = unsafe { ::std::mem::transmute::<u32, OpCode>(self as u32) };
        match _enum {
            | OpCode::OP_MOVE => { // ABC
                Instruction::ABC {
                    instruction: (self & (0b111111 << (32 - SIZE_OP))) as u8,
                    a: (self & (0b11111111 << (32 - SIZE_A - SIZE_OP))) as u8,
                    b: (self & (0b111111111 << (32 - SIZE_OP - SIZE_A - SIZE_B))) as u16,
                    c: (self & (0b111111111 <<
                                (32 - SIZE_OP - SIZE_A - SIZE_B - SIZE_C))) as u16,
                }
            },
            _ => panic!("bad opcode"),
        }
    }
}
