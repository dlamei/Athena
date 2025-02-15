use std::{fmt, marker::PhantomData, rc::Rc};

use paste::paste;

pub type Opcode = u64;
pub type Address = usize;

pub type float = f64;
const PI: float = std::f64::consts::PI;
const HALF_PI: float = std::f64::consts::FRAC_PI_2;
const TWO_PI: float = 2.0 * std::f64::consts::PI;
const E: float = std::f64::consts::E;

// use log as log2;
// mod log {
//     macro_rules! trace {
//         ($($tt:tt)*) => {
//             println!($($tt)*);
//         }
//     }
//     macro_rules! debug {
//         ($($tt:tt)*) => {
//             println!($($tt)*);
//         }
//     }
//     macro_rules! info {
//         ($($tt:tt)*) => {
//             println!($($tt)*);
//         }
//     }
//     pub(crate) use trace;
//     pub(crate) use debug;
//     pub(crate) use info;
// }

//TODO: use macros for constructors, use u32 for instruction and allow f64 literal?

/// module for all opcodes
///
/// opcodes are 64 unsinged bit integers, with the following layout: \
///
/// [ a b c d e_e_e_e]
///
/// a) 8 bits operator, e.g ADD, MOV, ... \
/// b) 8 bits lhs register \
/// c) 8 bits rhs register \
/// d) 8 bits out register \
/// e) 32 bits immediate value embedded within the instruction \
///
///
/// for unary operators the lhs register is used as input \
/// if either lhs or rhs are used and are set to zero use imm value instead
///
pub mod op {
    pub use super::*;

    macro_rules! ops {

        (_def: [], $code:expr) => {
            pub const NUM_OPS: usize = $code;
        };

        (_def: [$op0:ident $($op:ident)*], $code:expr) => {
            pub const $op0: u8 = $code;
            ops!(_def: [$($op)*], $code + 1);
        };

        ($op0:ident $(,$op:ident)* $(,)?) => {
            ops!(_def: [$op0 $($op)*], 0);
        };
    }

    ops! {
        OP_NOP,

        OP_ADD,
        OP_SUB,
        OP_MUL,
        OP_DIV,
        OP_POW,
        OP_SIN,
        OP_COS,
        OP_TAN,

        // print value of lhs reg
        OP_OUT,
        // move data to out reg
        OP_MOV,
        // pop from stack and move into out reg
        OP_POP,
        // push lhs reg value to the stack
        OP_PSH,

        OP_EXT,
    }

    #[inline(always)]
    const fn build_opcode(op: u8, lhs: u8, rhs: u8, out: u8, imm: u32) -> Opcode {
        let opcode =
            ((out as u64) << 24) | ((rhs as u64) << 16) | ((lhs as u64) << 8) | (op as u64);

        opcode | ((imm as u64) << 32)
    }

    #[inline(always)]
    const fn build_opcode_float(op: u8, lhs: u8, rhs: u8, out: u8, imm: float) -> Opcode {
        build_opcode(op, lhs, rhs, out, float_to_imm(imm))
    }

    macro_rules! binop_opcode {
        ($OP:ident) => {
            paste! {
                #[allow(non_snake_case)]
                pub const fn [<$OP _LHS_RHS>](lhs: u8, rhs: u8, out: u8) -> Opcode {
                    build_opcode_float([<OP_ $OP>], lhs, rhs, out, 0.0)
                }

                #[allow(non_snake_case)]
                pub const fn [<$OP _IMM_RHS>](lhs: float, rhs: u8, out: u8) -> Opcode {
                    build_opcode_float([<OP_ $OP>], 0, rhs, out, lhs)
                }

                #[allow(non_snake_case)]
                pub const fn [<$OP _LHS_IMM>](lhs: u8, rhs: float, out: u8) -> Opcode {
                    build_opcode_float([<OP_ $OP>], lhs, 0, out, rhs)
                }

                #[allow(non_snake_case)]
                pub const fn [<$OP _IMM_IMM>](v: float, out: u8) -> Opcode {
                    build_opcode_float([<OP_ $OP>], 0, 0, out, v)
                }
            }
        };
    }

    macro_rules! unary_opcode {
        ($OP:ident) => {
            paste! {
                #[allow(non_snake_case)]
                pub const fn [<$OP>] (lhs: u8, out: u8) -> Opcode {
                    build_opcode_float([<OP_ $OP>], lhs, 0, out, 0.0)
                }

                #[allow(non_snake_case)]
                pub const fn [<$OP _IMM>](val: float, out: u8) -> Opcode {
                    build_opcode_float([<OP_ $OP>], 0, 0, out, val)
                }

            }
        };
    }

    #[allow(non_snake_case)]
    pub const fn OUT(out: u8) -> Opcode {
        build_opcode_float(OP_OUT, out, 0, 0, 0.0)
    }

    #[allow(non_snake_case)]
    pub const fn PSH(lhs: u8) -> Opcode {
        build_opcode_float(OP_PSH, lhs, 0, 0, 0.0)
    }

    #[allow(non_snake_case)]
    pub const fn PSH_IMM(imm: float) -> Opcode {
        build_opcode_float(OP_PSH, 0, 0, 0, imm)
    }

    #[allow(non_snake_case)]
    pub const fn POP(out: u8) -> Opcode {
        build_opcode_float(OP_POP, 0, 0, out, 0.0)
    }

    #[allow(non_snake_case)]
    pub const fn EXT(exit_code: u32) -> Opcode {
        build_opcode(OP_EXT, 0, 0, 0, exit_code)
    }

    binop_opcode!(ADD);
    binop_opcode!(SUB);
    binop_opcode!(MUL);
    binop_opcode!(DIV);
    binop_opcode!(POW);

    unary_opcode!(SIN);
    unary_opcode!(COS);
    unary_opcode!(TAN);
    unary_opcode!(MOV);

    pub const fn EXP(lhs: u8, out: u8) -> Opcode {
        POW_LHS_IMM(lhs, E, out)
    }

    //#[allow(non_snake_case)]
    //#[inline(always)]
    //pub const fn LIT_F32(v: float) -> Opcode {
    //    (float::to_bits(v) as u64) << 32 | LIT
    //}

    //#[inline(always)]
    //pub const fn get_data_float(op: Opcode) -> float {
    //    debug_assert!((op & LIT) == LIT);
    //    float::from_bits(get_data(op))
    //}

    #[inline(always)]
    pub const fn get_op(op: Opcode) -> u8 {
        op as u8
    }

    // #[inline(always)]
    // pub const fn set_op(op: &mut Opcode, instr: u8) {
    //     *nth_byte_mut(op, 0) = instr;
    // }

    #[inline(always)]
    pub const fn get_lhs(op: Opcode) -> Address {
        (op >> 8) as u8 as usize
    }

    // #[inline(always)]
    // pub const fn set_lhs(op: &mut Opcode, reg: u8) {
    //     *nth_byte_mut(op, 1) = reg;
    // }

    #[inline(always)]
    pub const fn get_rhs(op: Opcode) -> Address {
        (op >> 16) as u8 as usize
    }

    // #[inline(always)]
    // pub const fn set_rhs(op: &mut Opcode, reg: u8) {
    //     *nth_byte_mut(op, 2) = reg;
    // }

    #[inline(always)]
    pub const fn get_out(op: Opcode) -> Address {
        (op >> 24) as u8 as usize
    }

    // #[inline(always)]
    // pub const fn set_out(op: &mut Opcode, reg: u8) {
    //     *nth_byte_mut(op, 3) = reg;
    // }

    #[inline(always)]
    pub const fn get_imm(op: Opcode) -> u32 {
        (op >> 32) as u32
    }

    #[inline(always)]
    pub const fn float_from_imm(imm: u32) -> float {
        f32::from_bits(imm) as float
    }

    #[inline(always)]
    pub const fn float_to_imm(f: float) -> u32 {
        f32::to_bits(f as f32)
    }

    // #[inline(always)]
    // pub const fn set_imm(op: &mut Opcode, data: u32) {
    //     *nth_u32_mut(op, 1) = data;
    // }

    /// returns (instr, lhs, rhs, out, data)
    #[inline(always)]
    pub const fn decode(op: Opcode) -> (u8, usize, usize, usize, u32) {
        let instr = get_op(op);
        let lhs = get_lhs(op);
        let rhs = get_rhs(op);
        let out = get_out(op);
        let data = get_imm(op);
        (instr, lhs, rhs, out, data)
    }

    pub const fn op_to_str(op: u8) -> &'static str {
        match op {
            OP_MOV => "MOV",
            OP_ADD => "ADD",
            OP_SUB => "SUB",
            OP_MUL => "MUL",
            OP_DIV => "DIV",
            OP_POW => "POW",
            OP_SIN => "SIN",
            OP_COS => "COS",
            OP_TAN => "TAN",
            OP_OUT => "OUT",
            OP_NOP => "NOP",
            OP_EXT => "EXT",
            OP_POP => "POP",
            OP_PSH => "PSH",
            _ => "UNKNOWN",
        }
    }

    pub const fn is_binary(op: u8) -> bool {
        match op {
            OP_ADD | OP_SUB | OP_MUL | OP_DIV | OP_POW => true,
            _ => false,
        }
    }
}

pub fn instr_to_str(instr: u64) -> String {
    let (op, lhs, rhs, out, imm) = op::decode(instr);

    let imm = op::float_from_imm(imm);
    let op_str = op::op_to_str(op);

    let lhs_str = if lhs == 0 {
        format!("{imm}f")
    } else {
        format!("{lhs}r")
    };
    let rhs_str = if rhs == 0 {
        format!("{imm}f")
    } else {
        format!("{rhs}r")
    };

    if op::is_binary(op) {
        format!("{op_str}({lhs_str}, {rhs_str}) -> {out}r")
    } else {
        format!("{op_str}({lhs_str}) -> {out}r")
    }
}

pub fn dbg_bytecode(code: &[Opcode]) {
    for instr in code {
        println!("{}", instr_to_str(*instr));
    }
}

pub type Instr<VM> = fn(vm: &mut VM, tape: &InstrTape);

/*
pub trait InstrSet {
    fn op_add(rt: &mut Runtime);
    fn op_sub(rt: &mut Runtime);
    fn op_mul(rt: &mut Runtime);
    fn op_div(rt: &mut Runtime);
    fn op_pow(rt: &mut Runtime);
    fn op_sin(rt: &mut Runtime);
    fn op_cos(rt: &mut Runtime);
    fn op_out(rt: &mut Runtime);
    //fn op_lit(rt: &mut Runtime) {
    //    let opcode = rt.program.fetch(rt.pc - 1);
    //    let data = op::get_data(opcode);
    //    rt.push(data);
    //    rt.next_op();
    //}
    fn op_nop(rt: &mut Runtime) {
        rt.next_op();
    }
    fn op_ext(_: &mut Runtime) {}
}
*/

#[derive(Debug)]
pub struct InstrTape<'a> {
    pub bin: &'a [Opcode],
}

impl InstrTape<'_> {
    #[inline(always)]
    pub fn fetch(&self, pc: Address) -> Opcode {
        self.bin[pc]
    }
}

const STACK_SIZE: usize = 256;

const REGISTER_COUNT: usize = 16;

pub trait InstrTable<VM> {
    fn nop(vm: &mut VM, t: &InstrTape) {}
    fn add(vm: &mut VM, t: &InstrTape);
    fn sub(vm: &mut VM, t: &InstrTape);
    fn mul(vm: &mut VM, t: &InstrTape);
    fn div(vm: &mut VM, t: &InstrTape);
    fn pow(vm: &mut VM, t: &InstrTape);
    fn sin(vm: &mut VM, t: &InstrTape);
    fn cos(vm: &mut VM, t: &InstrTape);
    fn tan(vm: &mut VM, t: &InstrTape);
    fn out(vm: &mut VM, t: &InstrTape);
    fn mov(vm: &mut VM, t: &InstrTape);
    fn psh(vm: &mut VM, t: &InstrTape);
    fn pop(vm: &mut VM, t: &InstrTape);
    fn ext(vm: &mut VM, t: &InstrTape) {
        log::debug!("exit")
    }

    // fn next_op(vm: &mut VM, t: &InstrTape);

    fn build_table() -> [Instr<VM>; op::NUM_OPS] {
        let mut table: [Instr<VM>; op::NUM_OPS] = [Self::nop; op::NUM_OPS];
        table[op::OP_ADD as usize] = Self::add;
        table[op::OP_SUB as usize] = Self::sub;
        table[op::OP_MUL as usize] = Self::mul;
        table[op::OP_DIV as usize] = Self::div;
        table[op::OP_POW as usize] = Self::pow;
        table[op::OP_SIN as usize] = Self::sin;
        table[op::OP_COS as usize] = Self::cos;
        table[op::OP_TAN as usize] = Self::tan;
        table[op::OP_OUT as usize] = Self::out;
        table[op::OP_NOP as usize] = Self::nop;
        table[op::OP_MOV as usize] = Self::mov;
        table[op::OP_PSH as usize] = Self::psh;
        table[op::OP_POP as usize] = Self::pop;
        table[op::OP_EXT as usize] = Self::ext;
        table
    }
}

// TODO: 1024 simultaneous evaluations
// TODO:  size of stack?
// TODO: custom vms
/*
#[derive(Debug, Clone)]
pub struct VM {
    pub instr_table: [Instr; op::NUM_OPS],
    pub instr_table_range: [Instr; op::NUM_OPS],

    pub registers: [float; REGISTER_COUNT],
    pub stack: [float; STACK_SIZE],

    pub registers_range: [Range; REGISTER_COUNT],
    pub stack_range: [Range; STACK_SIZE],

    pub sp: Address,
    pub pc: usize,
}

impl VM {
    pub fn new() -> Self {
        let mut vm = Self {
            //program: InstrTape { bin },
            stack: [float::NAN; STACK_SIZE],
            registers: [float::NAN; REGISTER_COUNT],
            stack_range: [Range::UNDEF; STACK_SIZE],
            registers_range: [Range::UNDEF; REGISTER_COUNT],
            sp: 0,
            instr_table: F32EvalInstrTable::build_table(),
            instr_table_range: RangeEvalInstrTable::build_table(),
            pc: 0,
        };
        vm
    }

    pub fn eval(&mut self, bin: &[Opcode]) {
        self.pc = 0;
        self.sp = 0;
        let t = InstrTape { bin };
        let instr = op::get_op(t.fetch(self.pc));
        (self.instr_table[instr as usize])(self, &t)
    }

    pub fn eval_range(&mut self, bin: &[Opcode]) {
        self.pc = 0;
        self.sp = 0;
        let t = InstrTape { bin };
        let instr = op::get_op(t.fetch(self.pc));
        (self.instr_table_range[instr as usize])(self, &t)
    }

    pub fn stack_push(&mut self, f: float) {
        self.sp += 1;
        if self.sp < self.stack.len() {
            self.stack[self.sp] = f;
        } else {
            panic!("stack overflow");
        }
    }

    pub fn stack_pop(&mut self) -> float {
        if self.sp < self.stack.len() {
            let f = self.stack[self.sp];
            self.sp -= 1;
            f
        } else {
            panic!("stack overflow");
        }
    }

    pub fn stack_range_push(&mut self, f: Range) {
        self.sp += 1;
        if self.sp < self.stack_range.len() {
            self.stack_range[self.sp] = f;
        } else {
            panic!("stack overflow");
        }
    }

    pub fn stack_range_pop(&mut self) -> Range {
        if self.sp < self.stack_range.len() {
            let f = self.stack_range[self.sp];
            self.sp -= 1;
            f
        } else {
            panic!("stack overflow");
        }
    }

    /// return (input value, out register)
    #[inline(always)]
    fn unary_arg(&mut self, t: &InstrTape) -> (float, usize) {
        let op = t.fetch(self.pc);
        let (_, l, _, out, imm) = op::decode(op);
        let lhs = if l == 0 {
            // float::from_bits(imm)
            op::float_from_imm(imm)
        } else {
            self.registers[l as usize]
        };

        (lhs, out as usize)
    }

    /// return (lhs value, rhs value, out register)
    #[inline(always)]
    fn binop_arg(&mut self, t: &InstrTape) -> (float, float, usize) {
        let op = t.fetch(self.pc);
        let (_, l, r, out, imm) = op::decode(op);
        let lhs = if l == 0 {
            // float::from_bits(imm)
            op::float_from_imm(imm)
        } else {
            self.registers[l as usize]
        };

        let rhs = if r == 0 {
            // float::from_bits(imm)
            op::float_from_imm(imm)
        } else {
            self.registers[r as usize]
        };

        (lhs, rhs, out as usize)
    }
}
*/

/*
#[derive(Debug, Clone, Copy)]
pub struct F32EvalInstrTable;

impl InstrTable for F32EvalInstrTable {
    fn add(vm: &mut VM, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        vm.registers[out] = lhs + rhs;
        log::trace!("    {} -> reg[{out}]", vm.registers[out]);
        Self::next_op(vm, t)
    }

    fn sub(vm: &mut VM, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        vm.registers[out] = lhs - rhs;
        log::trace!("    {} -> reg[{out}]", vm.registers[out]);
        Self::next_op(vm, t)
    }

    fn mul(vm: &mut VM, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        vm.registers[out] = lhs * rhs;
        log::trace!("    {} -> reg[{out}]", vm.registers[out]);
        Self::next_op(vm, t)
    }

    fn div(vm: &mut VM, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        vm.registers[out] = lhs / rhs;
        log::trace!("    {} -> reg[{out}]", vm.registers[out]);
        Self::next_op(vm, t)
    }

    fn pow(vm: &mut VM, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        vm.registers[out] = lhs.powf(rhs);
        log::trace!("    {} -> reg[{out}]", vm.registers[out]);
        Self::next_op(vm, t)
    }

    fn sin(vm: &mut VM, t: &InstrTape) {
        let (val, out) = vm.unary_arg(t);
        vm.registers[out] = val.sin();
        Self::next_op(vm, t);
    }

    fn cos(vm: &mut VM, t: &InstrTape) {
        let (val, out) = vm.unary_arg(t);
        vm.registers[out] = val.cos();
        Self::next_op(vm, t);
    }

    fn tan(vm: &mut VM, t: &InstrTape) {
        let (val, out) = vm.unary_arg(t);
        vm.registers[out] = val.tan();
        Self::next_op(vm, t);
    }

    fn out(vm: &mut VM, t: &InstrTape) {
        let (val, _) = vm.unary_arg(t);
        println!("{val}");
        Self::next_op(vm, t);
    }

    fn mov(vm: &mut VM, t: &InstrTape) {
        let (val, out) = vm.unary_arg(t);
        log::trace!("   {val} -> {out}");
        vm.registers[out as usize] = val;
        Self::next_op(vm, t);
    }

    fn psh(vm: &mut VM, t: &InstrTape) {
        let (val, _) = vm.unary_arg(t);
        vm.stack_push(val);
        log::trace!("   {} -> stack[{}]", vm.stack[vm.sp], vm.sp);
        Self::next_op(vm, t);
    }

    fn pop(vm: &mut VM, t: &InstrTape) {
        let (_, out) = vm.unary_arg(t);
        vm.registers[out] = vm.stack_pop();
        Self::next_op(vm, t);
    }

    fn next_op(vm: &mut VM, t: &InstrTape) {
        vm.pc += 1;
        //let instr = op::get_op(t.fetch(self.pc));
        let (op, lhs, rhs, out, imm) = op::decode(t.fetch(vm.pc));
        // let imm = float::from_bits(imm);
        let imm = op::float_from_imm(imm);
        // log::trace!("{}[{lhs}, {rhs}, {imm}] -> reg[{out}]", op::op_to_str(op));
        (vm.instr_table[op as usize])(vm, t)
    }
}
*/

pub trait VmWord: Clone + fmt::Debug + PartialEq {
    type Data: Default;
    fn from_imm(imm: u32) -> Self;
    fn uninit() -> Self;
}

pub struct VM<WORD: VmWord> {
    pub instr_table: [Instr<Self>; op::NUM_OPS],
    // TODO: do something about reg[0]
    pub reg: [WORD; REGISTER_COUNT],
    pub stack: [WORD; STACK_SIZE],
    pub sp: Address,
    pub pc: usize,
    pub data: WORD::Data,
}

impl<WORD: VmWord> VM<WORD> {
    pub fn clear_memory(&mut self) {
        self.reg = vec![WORD::uninit(); REGISTER_COUNT].try_into().unwrap();
        self.stack = vec![WORD::uninit(); STACK_SIZE].try_into().unwrap();
    }

    fn undef_instr(&mut self, _: &InstrTape) {
        panic!("instruction not found")
    }

    pub fn with_instr_table<T: InstrTable<Self>>(table: T) -> Self {
        let mut vm = Self::new();
        vm.set_instr_table(table);
        vm
    }

    pub fn new() -> Self {
        Self {
            instr_table: [Self::undef_instr; op::NUM_OPS],
            reg: vec![WORD::uninit(); REGISTER_COUNT].try_into().unwrap(),
            stack: vec![WORD::uninit(); STACK_SIZE].try_into().unwrap(),
            sp: 0,
            pc: 0,
            data: Default::default(),
        }
    }

    #[inline]
    pub fn call<I: IntoIterator<Item = WORD>>(&mut self, args: I, bin: &[Opcode]) -> WORD {
        for (i, arg) in args.into_iter().enumerate() {
            self.reg[i + 1] = arg;
        }
        // self.reg[1] = x;
        // self.reg[2] = y;
        // self.reg[3] = z;
        self.eval(bin);
        self.reg[1].clone()
    }

    pub fn eval(&mut self, bin: &[Opcode]) {
        self.pc = 0;
        self.sp = 0;
        let t = InstrTape { bin };
        let instr = op::get_op(t.fetch(self.pc));
        (self.instr_table[instr as usize])(self, &t)
    }

    pub fn set_instr_table<T: InstrTable<Self>>(&mut self, _instr_table: T) {
        self.instr_table = T::build_table();
    }

    #[inline(always)]
    fn unary_arg(&mut self, t: &InstrTape) -> (WORD, usize) {
        let op = t.fetch(self.pc);
        let (_, l, _, out, imm) = op::decode(op);
        let lhs = if l == 0 {
            // float::from_bits(imm)
            // op::float_from_imm(imm)
            VmWord::from_imm(imm)
        } else {
            self.reg[l as usize].clone()
        };

        (lhs, out as usize)
    }

    #[inline(always)]
    fn binop_arg(&mut self, t: &InstrTape) -> (WORD, WORD, usize) {
        let op = t.fetch(self.pc);
        let (_, l, r, out, imm) = op::decode(op);
        let lhs = if l == 0 {
            // float::from_bits(imm)
            // op::float_from_imm(imm)
            VmWord::from_imm(imm)
        } else {
            self.reg[l as usize].clone()
        };

        let rhs = if r == 0 {
            // float::from_bits(imm)
            // op::float_from_imm(imm)
            VmWord::from_imm(imm)
        } else {
            self.reg[r as usize].clone()
        };

        (lhs, rhs, out as usize)
    }

    pub fn stack_push(&mut self, f: WORD) {
        self.sp += 1;
        if self.sp < self.stack.len() {
            self.stack[self.sp] = f;
        } else {
            panic!("stack overflow");
        }
    }

    pub fn stack_pop(&mut self) -> WORD {
        if self.sp < self.stack.len() {
            let f = self.stack[self.sp].clone();
            self.sp -= 1;
            f
        } else {
            panic!("stack overflow");
        }
    }

    #[inline(always)]
    pub fn next(&mut self, t: &InstrTape) {
        self.pc += 1;
        //let instr = op::get_op(t.fetch(self.pc));
        let (op, lhs, rhs, out, imm) = op::decode(t.fetch(self.pc));
        // let imm = float::from_bits(imm);
        let imm = op::float_from_imm(imm);
        log::trace!("{}[{lhs}, {rhs}, {imm}] -> reg[{out}]", op::op_to_str(op));
        (self.instr_table[op as usize])(self, t)
    }
}

impl VmWord for f64 {
    type Data = ();

    fn from_imm(imm: u32) -> Self {
        op::float_from_imm(imm)
    }

    fn uninit() -> Self {
        f64::NAN
    }
}

pub struct F64InstrTable;

impl InstrTable<VM<f64>> for F64InstrTable {
    fn add(vm: &mut VM<f64>, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        vm.reg[out] = lhs + rhs;
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn sub(vm: &mut VM<f64>, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        vm.reg[out] = lhs - rhs;
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn mul(vm: &mut VM<f64>, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        vm.reg[out] = lhs * rhs;
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn div(vm: &mut VM<f64>, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        vm.reg[out] = lhs / rhs;
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn pow(vm: &mut VM<f64>, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        vm.reg[out] = lhs.powf(rhs);
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn sin(vm: &mut VM<f64>, t: &InstrTape) {
        let (val, out) = vm.unary_arg(t);
        vm.reg[out] = val.sin();
        vm.next(t);
    }

    fn cos(vm: &mut VM<f64>, t: &InstrTape) {
        let (val, out) = vm.unary_arg(t);
        vm.reg[out] = val.cos();
        vm.next(t);
    }

    fn tan(vm: &mut VM<f64>, t: &InstrTape) {
        let (val, out) = vm.unary_arg(t);
        vm.reg[out] = val.tan();
        vm.next(t);
    }

    fn out(vm: &mut VM<f64>, t: &InstrTape) {
        let (val, _) = vm.unary_arg(t);
        println!("{val}");
        vm.next(t);
    }

    fn mov(vm: &mut VM<f64>, t: &InstrTape) {
        let (val, out) = vm.unary_arg(t);
        log::trace!("   {val} -> {out}");
        vm.reg[out as usize] = val;
        vm.next(t);
    }

    fn psh(vm: &mut VM<f64>, t: &InstrTape) {
        let (val, _) = vm.unary_arg(t);
        vm.stack_push(val);
        log::trace!("   {} -> stack[{}]", vm.stack[vm.sp], vm.sp);
        vm.next(t);
    }

    fn pop(vm: &mut VM<f64>, t: &InstrTape) {
        let (_, out) = vm.unary_arg(t);
        vm.reg[out] = vm.stack_pop();
        vm.next(t);
    }

    fn nop(vm: &mut VM<f64>, t: &InstrTape) {
        vm.next(t);
    }

    fn ext(vm: &mut VM<f64>, _: &InstrTape) {
        log::trace!("EXT");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FDeriv {
    pub val: f64,
    pub grad: f64,
}

impl FDeriv {
    pub fn var(val: f64) -> Self {
        Self { val, grad: 1.0 }
    }

    pub fn cnst(val: f64) -> Self {
        Self { val, grad: 0.0 }
    }
}

impl VmWord for FDeriv {
    type Data = ();

    fn from_imm(imm: u32) -> Self {
        Self::cnst(op::float_from_imm(imm))
    }

    fn uninit() -> Self {
        Self {
            val: f64::NAN,
            grad: f64::NAN,
        }
    }
}

pub struct FDerivInstrTable;

impl InstrTable<VM<FDeriv>> for FDerivInstrTable {
    fn add(vm: &mut VM<FDeriv>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = FDeriv {
            val: a.val + b.val,
            grad: a.grad + b.grad,
        };
        vm.reg[out] = c;
        vm.next(t);
    }

    fn sub(vm: &mut VM<FDeriv>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = FDeriv {
            val: a.val - b.val,
            grad: a.grad - b.grad,
        };
        vm.reg[out] = c;
        vm.next(t);
    }

    fn mul(vm: &mut VM<FDeriv>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = FDeriv {
            val: a.val * b.val,
            grad: a.val * b.grad + a.grad * b.val,
        };
        vm.reg[out] = c;
        vm.next(t);
    }

    fn div(vm: &mut VM<FDeriv>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let (da, db) = (a.grad, b.grad);
        let (a, b) = (a.val, b.val);
        let c = FDeriv {
            val: a / b,
            grad: (b * da - a * db) / b.powf(2.0),
        };
        vm.reg[out] = c;
        vm.next(t);
    }

    fn pow(vm: &mut VM<FDeriv>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let (da, db) = (a.grad, b.grad);
        let (a, b) = (a.val, b.val);
        let c = FDeriv {
            val: a.powf(b),
            grad: b * a.powf(b - 1.0) * da + a.powf(b) * a.ln() * db,
        };
        vm.reg[out] = c;
        vm.next(t);
    }

    fn sin(vm: &mut VM<FDeriv>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        let c = FDeriv {
            val: a.val.sin(),
            grad: a.val.cos() * a.grad,
        };
        vm.reg[out] = c;
        vm.next(t);
    }

    fn cos(vm: &mut VM<FDeriv>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        let c = FDeriv {
            val: a.val.cos(),
            grad: -a.val.sin() * a.grad,
        };
        vm.reg[out] = c;
        vm.next(t);
    }

    fn tan(vm: &mut VM<FDeriv>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        let c = FDeriv {
            val: a.val.tan(),
            grad: a.grad * 1.0 / a.val.cos().powf(2.0),
        };
        vm.reg[out] = c;
        vm.next(t);
    }

    fn out(vm: &mut VM<FDeriv>, t: &InstrTape) {
        let (a, _) = vm.unary_arg(t);
        println!("{a:?}");
        vm.next(t);
    }

    fn mov(vm: &mut VM<FDeriv>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        vm.reg[out] = a;
        vm.next(t);
    }

    fn psh(vm: &mut VM<FDeriv>, t: &InstrTape) {
        let (a, _) = vm.unary_arg(t);
        vm.stack_push(a);
        vm.next(t);
    }

    fn pop(vm: &mut VM<FDeriv>, t: &InstrTape) {
        let (_, out) = vm.unary_arg(t);
        vm.reg[out] = vm.stack_pop();
        vm.next(t);
    }
}

impl VmWord for Range {
    type Data = ();

    fn from_imm(imm: u32) -> Self {
        let imm = op::float_from_imm(imm);
        Range::new(imm, imm)
    }

    fn uninit() -> Self {
        Range::UNDEF
    }
}

pub struct RangeInstrTable;

impl InstrTable<VM<Range>> for RangeInstrTable {
    fn add(vm: &mut VM<Range>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = Range::of_add(a, b);
        vm.reg[out] = c;
        log::debug!("add({a}, {b}) = {c}");
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t)
    }

    fn sub(vm: &mut VM<Range>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = Range::of_sub(a, b);
        vm.reg[out] = c;
        log::debug!("sub({a}, {b}) = {c}");
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t)
    }

    fn mul(vm: &mut VM<Range>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = Range::of_mul(a, b);
        vm.reg[out] = c;
        log::debug!("mul({a}, {b}) = {c}");
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t)
    }

    fn div(vm: &mut VM<Range>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = Range::of_div(a, b);

        vm.reg[out] = c;
        log::debug!("div({a}, {b}) = {c}");
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t)
    }

    fn pow(vm: &mut VM<Range>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = Range::of_pow(a, b);
        log::debug!("pow({a}, {b}) = {c}");
        vm.reg[out] = c;
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t)
    }

    fn sin(vm: &mut VM<Range>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        let b = Range::of_sin(a);
        vm.reg[out] = b;
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t)
    }

    fn cos(vm: &mut VM<Range>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        let b = Range::of_cos(a);
        vm.reg[out] = b;
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t)
    }

    fn tan(vm: &mut VM<Range>, t: &InstrTape) {
        todo!()
    }

    fn out(vm: &mut VM<Range>, t: &InstrTape) {
        let (val, _) = vm.unary_arg(t);
        println!("{val}");
        vm.next(t)
    }

    fn mov(vm: &mut VM<Range>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        log::trace!("   {a} -> {out}");
        vm.reg[out as usize] = a;
        vm.next(t)
    }

    fn psh(vm: &mut VM<Range>, t: &InstrTape) {
        let (val, _) = vm.unary_arg(t);
        vm.stack_push(val);
        log::trace!("   {} -> stack[{}]", vm.stack[vm.sp], vm.sp);
        vm.next(t)
    }

    fn pop(vm: &mut VM<Range>, t: &InstrTape) {
        let (_, out) = vm.unary_arg(t);
        vm.reg[out] = vm.stack_pop();
        vm.next(t)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RangeDeriv {
    pub val: Range,
    pub grad: Range,
}

impl RangeDeriv {
    pub fn var(val: Range) -> Self {
        Self {
            val,
            grad: Range::new_const(1.0),
        }
    }

    pub fn cnst(val: Range) -> Self {
        Self {
            val,
            grad: Range::new_const(0.0),
        }
    }

    pub fn add_deriv(self, other: Self) -> Self {
        let (a, b) = (self, other);
        Self {
            val: a.val.add(b.val),
            grad: a.grad.add(b.grad),
        }
    }

    pub fn sub_deriv(self, other: Self) -> Self {
        let (a, b) = (self, other);
        Self {
            val: a.val.sub(b.val),
            grad: a.grad.sub(b.grad),
        }
    }

    pub fn mul_deriv(self, other: Self) -> Self {
        let (a, b) = (self, other);
        Self {
            val: a.val.mul(b.val),
            grad: a.val.mul(b.grad).add(a.grad.mul(a.val)),
        }
    }

    pub fn div_deriv(self, other: Self) -> Self {
        let (a, b) = (self, other);
        let (da, db) = (a.grad, b.grad);
        let (a, b) = (a.val, b.val);
        Self {
            val: a.div(b),
            grad: b.mul(da).sub(a.mul(db)).div(b.pow(Range::new_const(2.0))),
        }
    }

    pub fn pow_deriv(self, other: Self) -> Self {
        let (a, b) = (self, other);
        let (da, db) = (a.grad, b.grad);
        let (a, b) = (a.val, b.val);
        Self {
            val: a.pow(b),
            grad: b
                .mul(a.pow(b.sub(Range::ONE)))
                .mul(da)
                .add(a.pow(b).mul(a.ln()).mul(db)),
        }
    }

    pub fn sin_deriv(self) -> Self {
        Self {
            val: self.val.sin(),
            grad: self.val.cos().mul(self.grad),
        }
    }

    pub fn cos_deriv(self) -> Self {
        Self {
            val: self.val.sin(),
            grad: Range::MINUS_ONE.mul(self.val.sin()).mul(self.grad),
        }
    }

    pub fn tan_deriv(self) -> Self {
        todo!()
    }
}

impl VmWord for RangeDeriv {
    type Data = ();

    fn from_imm(imm: u32) -> Self {
        Self::cnst(Range::new_const(op::float_from_imm(imm)))
    }

    fn uninit() -> Self {
        Self {
            val: Range::UNDEF,
            grad: Range::UNDEF,
        }
    }
}

pub struct RangeDerivInstrTable;

impl InstrTable<VM<RangeDeriv>> for RangeDerivInstrTable {
    fn add(vm: &mut VM<RangeDeriv>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = a.add_deriv(b);
        vm.reg[out] = c;
        vm.next(t);
    }

    fn sub(vm: &mut VM<RangeDeriv>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = a.sub_deriv(b);
        vm.reg[out] = c;
        vm.next(t);
    }

    fn mul(vm: &mut VM<RangeDeriv>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = a.mul_deriv(b);
        vm.reg[out] = c;
        vm.next(t);
    }

    fn div(vm: &mut VM<RangeDeriv>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = a.div_deriv(b);
        vm.reg[out] = c;
        vm.next(t);
    }

    fn pow(vm: &mut VM<RangeDeriv>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = a.pow_deriv(b);
        vm.reg[out] = c;
        vm.next(t);
    }

    fn sin(vm: &mut VM<RangeDeriv>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        let c = a.sin_deriv();
        vm.reg[out] = c;
        vm.next(t);
    }

    fn cos(vm: &mut VM<RangeDeriv>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        let c = a.cos_deriv();
        vm.reg[out] = c;
        vm.next(t);
    }

    fn tan(vm: &mut VM<RangeDeriv>, t: &InstrTape) {
        todo!()
    }

    fn out(vm: &mut VM<RangeDeriv>, t: &InstrTape) {
        let (a, _) = vm.unary_arg(t);
        println!("{a:?}");
        vm.next(t);
    }

    fn mov(vm: &mut VM<RangeDeriv>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        vm.reg[out] = a;
        vm.next(t);
    }

    fn psh(vm: &mut VM<RangeDeriv>, t: &InstrTape) {
        let (a, _) = vm.unary_arg(t);
        vm.stack_push(a);
        vm.next(t);
    }

    fn pop(vm: &mut VM<RangeDeriv>, t: &InstrTape) {
        let (_, out) = vm.unary_arg(t);
        vm.reg[out] = vm.stack_pop();
        vm.next(t);
    }
}

pub struct F64VecInstrTable;

impl VM<F64Vec> {
    pub fn set_vec_size(&mut self, size: usize) {
        self.data = size;
    }

    pub fn take_reg(&mut self, indx: usize) -> Vec<f64> {
        let res = self.reg[indx].clone();

        self.clear_memory();

        match res {
            F64Vec::Vec(vec) => {
                if let Ok(vec) = Rc::try_unwrap(vec.clone()) {
                    return vec;
                } else {
                    (*vec).clone()
                }
            }
            F64Vec::Imm(imm) => vec![imm; self.data],
        }
    }
    pub fn take_stack(&mut self, indx: usize) -> Vec<f64> {
        let res = self.stack[indx].clone();

        self.clear_memory();

        match res {
            F64Vec::Vec(vec) => {
                if let Ok(vec) = Rc::try_unwrap(vec.clone()) {
                    return vec;
                } else {
                    (*vec).clone()
                }
            }
            F64Vec::Imm(imm) => vec![imm; self.data],
        }
    }
}

impl InstrTable<VM<F64Vec>> for F64VecInstrTable {
    fn add(vm: &mut VM<F64Vec>, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        vm.reg[out] = lhs.add(&rhs);
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn sub(vm: &mut VM<F64Vec>, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        vm.reg[out] = lhs.sub(&rhs);
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn mul(vm: &mut VM<F64Vec>, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        vm.reg[out] = lhs.mul(&rhs);
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn div(vm: &mut VM<F64Vec>, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        vm.reg[out] = lhs.div(&rhs);
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn pow(vm: &mut VM<F64Vec>, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        vm.reg[out] = lhs.pow(&rhs);
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn sin(vm: &mut VM<F64Vec>, t: &InstrTape) {
        let (lhs, out) = vm.unary_arg(t);
        vm.reg[out] = lhs.sin();
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn cos(vm: &mut VM<F64Vec>, t: &InstrTape) {
        let (lhs, out) = vm.unary_arg(t);
        vm.reg[out] = lhs.cos();
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn tan(vm: &mut VM<F64Vec>, t: &InstrTape) {
        let (lhs, out) = vm.unary_arg(t);
        vm.reg[out] = lhs.tan();
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn out(vm: &mut VM<F64Vec>, t: &InstrTape) {
        let (val, _) = vm.unary_arg(t);
        log::trace!("{val}");
        vm.next(t)
    }

    fn mov(vm: &mut VM<F64Vec>, t: &InstrTape) {
        let (val, out) = vm.unary_arg(t);
        vm.reg[out as usize] = val.clone();
        log::trace!("   {val} -> {out}");
        vm.next(t);
    }

    fn psh(vm: &mut VM<F64Vec>, t: &InstrTape) {
        let (val, _) = vm.unary_arg(t);
        vm.stack_push(val);
        log::trace!("   {} -> stack[{}]", vm.stack[vm.sp], vm.sp);
        vm.next(t);
    }

    fn pop(vm: &mut VM<F64Vec>, t: &InstrTape) {
        let (_, out) = vm.unary_arg(t);
        vm.reg[out] = vm.stack_pop();
        vm.next(t);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum F64Vec {
    Vec(Rc<Vec<f64>>),
    Imm(f64),
}

impl VmWord for F64Vec {
    type Data = usize;

    fn from_imm(imm: u32) -> Self {
        let v = op::float_from_imm(imm);
        F64Vec::Imm(v)
    }

    fn uninit() -> Self {
        F64Vec::Imm(f64::NAN)
    }
}

impl fmt::Display for F64Vec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            F64Vec::Vec(vec) => write!(f, "{vec:?}"),
            F64Vec::Imm(i) => write!(f, "{i}"),
        }
    }
}

impl Default for F64Vec {
    fn default() -> Self {
        0.0.into()
    }
}

impl From<f64> for F64Vec {
    fn from(value: f64) -> Self {
        F64Vec::Imm(value)
    }
}

impl From<Vec<f64>> for F64Vec {
    fn from(value: Vec<f64>) -> Self {
        Self::Vec(value.into())
    }
}

trait ExplicitCopy: Copy {
    #[inline(always)]
    fn copy(&self) -> Self {
        *self
    }
}

impl<T: Copy> ExplicitCopy for T {}

macro_rules! op_imm_vec {
    ($lhs:ident : $imm:expr, $rhs:ident : $vec:expr => $body:block) => {{
        let vec = $vec;
        // let mut res = vec![0.0; vec.len()];

        let $lhs = $imm.copy();
        /*
        for i in 0..vec.len() {
        let $rhs = vec[i].copy();
        res[i] = $body;
        }
        */
        let res: Vec<_> = vec
            .iter()
            .map(|r| {
                let $rhs = r.copy();
                $body
            })
            .collect();

        F64Vec::Vec(res.into())
    }};
}

macro_rules! op_vec_imm {
    ($lhs:ident: $vec:expr, $rhs:ident: $imm:expr => $body:block) => {{
        let vec = $vec;
        // let mut res = vec![0.0; vec.len()];

        let $rhs = $imm.copy();

        /*
        for i in 0..vec.len() {
        let $lhs = vec[i].copy();
        res[i] = $body;
        }
        */
        let res: Vec<_> = vec
            .iter()
            .map(|l| {
                let $lhs = l.copy();
                $body
            })
            .collect();

        F64Vec::Vec(res.into())
    }};
}

macro_rules! op_vec_vec {
    ($lhs:ident: $vec1:expr, $rhs:ident: $vec2:expr => $body:block) => {{
        let vec1 = $vec1;
        let vec2 = $vec2;
        debug_assert!(vec1.len() == vec2.len());
        // let mut res = vec![0.0; vec1.len()];

        /*
        for i in 0..vec1.len() {
        let $lhs = vec1[i].copy();
        let $rhs = vec2[i].copy();
        res[i] = $body;
        }
        */

        let res: Vec<_> = vec1
            .iter()
            .zip(vec2.iter())
            .map(|(l, r)| {
                let $lhs = l.copy();
                let $rhs = r.copy();
                $body
            })
            .collect();

        F64Vec::Vec(res.into())
    }};
}

macro_rules! f64_vec_op {
    ($lhs:ident: $fvec1:expr, $rhs:ident: $fvec2:expr => $body:block) => {{
        match ($fvec1, $fvec2) {
            (F64Vec::Vec(v1), F64Vec::Vec(v2)) => op_vec_vec!($lhs: v1,$rhs: v2 => $body),
            (F64Vec::Vec(v), F64Vec::Imm(i)) => op_vec_imm!($lhs: v, $rhs: i => $body),
            (F64Vec::Imm(i), F64Vec::Vec(v)) => op_imm_vec!($lhs: i, $rhs: v => $body),
            (F64Vec::Imm(i1), F64Vec::Imm(i2)) => {
                let $lhs = i1.copy();
                let $rhs = i2.copy();
                F64Vec::Imm($body)
            }
        }
    }};

    ($val:ident: $fvec:expr => $body:block) => {{
        match $fvec {
            F64Vec::Vec(vec) => {
                // let mut res = vec![0.0; vec.len()];
                // for i in 0..vec.len() {
                //     let $val = vec[i].copy();
                //     res[i] = $body;
                // }

                let res: Vec<_> = vec.iter()
                    .map(|v| {
                        let $val = v.copy();
                        $body
                    }).collect();

                F64Vec::Vec(res.into())
            }
            F64Vec::Imm(i) => {
                let $val = i.copy();
                F64Vec::Imm($body)
            }
        }
    }};
}

impl F64Vec {
    #[inline(always)]
    pub fn add(&self, other: &Self) -> Self {
        f64_vec_op!(lhs: self, rhs: other => { lhs + rhs })
    }
    #[inline(always)]
    pub fn sub(&self, other: &Self) -> Self {
        f64_vec_op!(lhs: self, rhs: other => { lhs - rhs })
    }
    #[inline(always)]
    pub fn mul(&self, other: &Self) -> Self {
        f64_vec_op!(lhs: self, rhs: other => { lhs * rhs })
    }
    #[inline(always)]
    pub fn div(&self, other: &Self) -> Self {
        f64_vec_op!(lhs: self, rhs: other => { lhs / rhs })
    }
    #[inline(always)]
    pub fn pow(&self, other: &Self) -> Self {
        f64_vec_op!(lhs: self, rhs: other => { lhs.powf(rhs) })
    }
    #[inline(always)]
    pub fn sin(&self) -> Self {
        f64_vec_op!(v: self => { v.sin() })
    }
    #[inline(always)]
    pub fn cos(&self) -> Self {
        f64_vec_op!(v: self => { v.cos() })
    }
    #[inline(always)]
    pub fn tan(&self) -> Self {
        f64_vec_op!(v: self => { v.tan() })
    }
}

#[inline(always)]
const fn min_max_2(a: float, b: float) -> (float, float) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

#[inline(always)]
const fn min_max_4(a: float, b: float, c: float, d: float) -> (float, float) {
    let (a, b) = min_max_2(a, b);
    let (c, d) = min_max_2(c, d);
    (min_max_2(a, c).0, min_max_2(b, d).1)
}

#[derive(Debug, Default, PartialEq, Copy, Clone)]
pub struct Range {
    pub l: float,
    pub u: float,
}

impl std::fmt::Display for Range {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.l, self.u)
    }
}

impl From<(float, float)> for Range {
    fn from(value: (float, float)) -> Self {
        Range::new(value.0, value.1)
    }
}

impl Range {
    pub const NULL: Self = Self {
        l: float::INFINITY,
        u: float::NEG_INFINITY,
    };

    pub const UNDEF: Self = Self {
        l: float::NAN,
        u: float::NAN,
    };
    // entire possible range
    pub const INF: Self = Self {
        l: float::NEG_INFINITY,
        u: float::INFINITY,
    };

    pub const ONE: Self = Self::new_const(1.0);
    pub const MINUS_ONE: Self = Self::new_const(-1.0);

    // no single continous range
    // pub const NON_CONTINUOUS: Self = Self {
    //     l: float::INFINITY,
    //     u: float::NEG_INFINITY,
    // };

    #[inline(always)]
    pub fn new(l: float, u: float) -> Self {
        debug_assert!(l <= u || (l.is_nan() && u.is_nan()), "{l} <= {u}");
        Range { l, u }
    }

    #[inline(always)]
    pub const fn new_const(v: float) -> Self {
        Range { l: v, u: v }
    }

    #[inline(always)]
    pub fn add(self, other: Self) -> Self {
        Self::of_add(self, other)
    }
    #[inline(always)]
    pub fn sub(self, other: Self) -> Self {
        Self::of_sub(self, other)
    }
    #[inline(always)]
    pub fn mul(self, other: Self) -> Self {
        Self::of_mul(self, other)
    }
    #[inline(always)]
    pub fn div(self, other: Self) -> Self {
        Self::of_div(self, other)
    }
    #[inline(always)]
    pub fn pow(self, other: Self) -> Self {
        Self::of_pow(self, other)
    }
    #[inline(always)]
    pub fn sin(self) -> Self {
        Self::of_sin(self)
    }
    #[inline(always)]
    pub fn cos(self) -> Self {
        Self::of_cos(self)
    }
    #[inline(always)]
    pub fn ln(self) -> Self {
        Self::of_ln(self)
    }

    #[inline(always)]
    pub fn of_sin(a: Range) -> Self {
        if a.is_undef() {
            return Self::UNDEF;
        } else if a.dist() >= TWO_PI {
            return (-1.0, 1.0).into();
        }

        let l = a.l.rem_euclid(TWO_PI);
        let u = a.u.rem_euclid(TWO_PI);

        let mut min = a.l.sin();
        let mut max = a.u.sin();

        if min > max {
            (min, max) = (max, min);
        }

        if l <= u {
            if l <= HALF_PI && u >= HALF_PI {
                max = 1.0;
            } else if l <= 3.0 * HALF_PI && u >= 3.0 * HALF_PI {
                min = -1.0;
            }
        } else {
            min = -1.0;
            max = 1.0;
        }

        (min, max).into()

        // let (u, l) = (a.u, a.l);
        // if l.is_nan() || u.is_nan() {
        //     Self::UNDEF
        // } else if l.is_infinite() || u.is_infinite() {
        //     Range::new(-1.0, 1.0)
        // } else {
        //     let m = (((l - HALF_PI) / PI).ceil() * PI + HALF_PI).min(u);
        //     let n = (m + PI).min(u);
        //     min_max_4(m.sin(), n.sin(), l.sin(), u.sin()).into()
        // }
    }

    #[inline(always)]
    pub fn of_cos(a: Range) -> Self {
        if a.is_undef() {
            return Self::UNDEF;
        } else if a.dist() >= TWO_PI {
            return (-1.0, 1.0).into();
        }

        let l = a.l.rem_euclid(TWO_PI);
        let u = a.u.rem_euclid(TWO_PI);

        let mut min = a.l.cos();
        let mut max = a.u.cos();

        if min > max {
            (min, max) = (max, min);
        }

        if l <= u {
            if l == 0.0 {
                max = 1.0;
            }
            if l <= PI && u >= PI {
                min = -1.0;
            }
        } else {
            max = 1.0;
            if !(u < PI && PI < l) {
                min = -1.0;
            }
        }

        (min, max).into()
    }

    #[inline(always)]
    pub fn of_add(a: Range, b: Range) -> Self {
        (a.l + b.l, a.u + b.u).into()
    }

    #[inline(always)]
    pub fn of_sub(a: Range, b: Range) -> Self {
        (a.l - b.u, a.u - b.l).into()
    }

    #[inline(always)]
    pub fn of_mul(a: Range, b: Range) -> Self {
        min_max_4(a.l * b.l, a.l * b.u, a.u * b.l, a.u * b.u).into()
    }

    #[inline(always)]
    pub fn of_div(a: Range, b: Range) -> Self {
        min_max_4(a.l / b.l, a.l / b.u, a.u / b.l, a.u / b.u).into()
    }

    #[inline(always)]
    pub fn of_pow(a: Range, b: Range) -> Self {
        if a.is_undef() || b.is_undef() {
            return Self::UNDEF;
        }

        let c = if a.l >= 0.0 {
            // a >= 0
            if a.l > 0.0 {
                // a > 0
                Range::from_tuple(min_max_4(
                    a.l.powf(b.l),
                    a.u.powf(b.u),
                    a.u.powf(b.l),
                    a.l.powf(b.u),
                ))
            } else {
                if !b.contains_zero() {
                    Range::new(0.0, a.u.powf(b.u))
                } else {
                    Range::UNDEF
                }
            }
        } else if a.u < 0.0 {
            // a < 0
            if b.is_const_int() {
                let b = b.l;

                Range::from_tuple(min_max_2(a.u.powf(b), a.l.powf(b)))
            } else {
                Range::UNDEF
            }
        } else {
            if b.is_const_int() {
                let b = b.l;

                if (b % 2.0).abs() < float::EPSILON {
                    Range::new(0.0, a.l.abs().max(a.u).powf(b))
                } else {
                    Range::new(a.l.powf(b), a.l.abs().max(a.u).powf(b))
                }
            } else {
                Range::UNDEF
            }
        };
        c
    }

    #[inline(always)]
    pub fn of_ln(a: Range) -> Self {
        if a.is_undef() || a.l <= 0.0 {
            return a;
        }

        (a.l.ln(), a.u.ln()).into()
    }

    //     #[inline(always)]
    //     pub const fn in_range(&self, r: Range) -> bool {
    //         self.l > r.l && self.u < r.u
    //     }

    #[inline(always)]
    pub fn from_tuple(b: (float, float)) -> Self {
        Self::new(b.0, b.1)
    }

    #[inline(always)]
    pub fn is_inf(&self) -> bool {
        //self == &Self::F32
        // self.l.is_infinite() || self.u.is_infinite()
        self.l == float::NEG_INFINITY && self.u == float::INFINITY
    }

    #[inline(always)]
    pub const fn is_finite(&self) -> bool {
        self.l.is_finite() && self.u.is_finite()
    }

    #[inline(always)]
    pub const fn is_const(&self) -> bool {
        (self.l - self.u) < float::EPSILON
    }

    #[inline(always)]
    pub fn is_const_int(&self) -> bool {
        self.is_const() && self.l.fract() == 0.0
    }

    #[inline(always)]
    pub const fn is_pos(&self) -> bool {
        self.l > 0.0 && self.u > 0.0
    }

    #[inline(always)]
    pub const fn is_neg(&self) -> bool {
        self.l < 0.0 && self.u < 0.0
    }

    // pub fn is_non_continuous(&self) -> bool {
    //     self == &Self::NON_CONTINUOUS
    // }

    #[inline(always)]
    pub fn is_undef(&self) -> bool {
        self.l.is_nan() || self.u.is_nan() || self == &Self::NULL
    }

    #[inline(always)]
    pub fn is_null(&self) -> bool {
        self == &Self::NULL
    }

    #[inline(always)]
    pub const fn contains_zero(&self) -> bool {
        self.l <= 0.0 && self.u >= 0.0
    }

    #[inline(always)]
    pub fn dist(&self) -> float {
        (self.l.powf(2.0) + self.u.powf(2.0)).sqrt()
    }
}

//pub fn run() {
//    let code = [
//        op::ADD_LHS_RHS(1, 2, 3),
//        op::MOV(3, 1),
//        op::OUT(1),
//        op::PSH(1),
//        op::EXT(0),
//    ];

//    let mut vm = machines::VmF32::new();
//    vm.reg[1] = 2.0;
//    vm.reg[2] = 3.0;
//    vm.eval(&code);

//    //op::MOV_IMM(2.0, 1),
//    //op::MOV_IMM(3.0, 2),
//    vm.registers_range[1] = (2.0, 2.5).into();
//    vm.registers_range[2] = (3.0, 3.0).into();
//    vm.eval_range(&code);
//}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basic() {
        let code = [
            op::MOV_IMM(2.0, 1),
            op::MOV_IMM(3.0, 2),
            op::ADD_LHS_RHS(1, 2, 3),
            op::POW_LHS_IMM(3, 2.0, 1),
            op::DIV_IMM_RHS(100.0, 1, 1),
            op::SUB_LHS_IMM(1, 4.0, 1),
            op::COS(1, 1),
            op::PSH(1),
            op::EXT(0),
        ];

        let mut vm_f32 = VM::<f64>::new();
        vm_f32.set_instr_table(F64InstrTable);
        vm_f32.eval(&code);

        let mut vm_range = VM::<Range>::new();
        vm_range.set_instr_table(RangeInstrTable);
        vm_range.eval(&code);

        let mut vm_f32_vec = VM::<F64Vec>::new();
        vm_f32_vec.set_instr_table(F64VecInstrTable);
        vm_f32_vec.data = 1;
        vm_f32_vec.eval(&code);

        let res = vm_f32.stack[1];
        assert_eq!(res, vm_range.stack[1].l);
        assert_eq!(res, vm_range.stack[1].u);
        assert_eq!(res, vm_f32_vec.take_stack(1)[0]);

        let code = [
            op::MOV_IMM(5.0, 1),
            op::MOV_IMM(6.0, 2),
            op::MOV_IMM(7.0, 3),
            op::SIN(1, 4),            // sin(x)
            op::SIN(2, 5),            // sin(y)
            op::SIN(3, 6),            // sin(z)
            op::COS(1, 1),            // cos(x)
            op::COS(2, 2),            // cos(y)
            op::COS(3, 3),            // cos(z)
            op::MUL_LHS_RHS(6, 1, 1), // sin(z)*cos(x)
            op::MUL_LHS_RHS(5, 3, 3), // sin(y)*cos(z)
            op::MUL_LHS_RHS(4, 2, 2), // sin(x)*cos(y)
            op::ADD_LHS_RHS(2, 1, 1),
            op::ADD_LHS_RHS(3, 1, 1),
            op::PSH(1),
            op::EXT(0),
        ];

        let mut vm_f32 = VM::<f64>::new();
        vm_f32.set_instr_table(F64InstrTable);
        vm_f32.eval(&code);

        let mut vm_range = VM::<Range>::new();
        vm_range.set_instr_table(RangeInstrTable);
        vm_range.eval(&code);

        let mut vm_f32_vec = VM::<F64Vec>::new();
        vm_f32_vec.set_instr_table(F64VecInstrTable);
        vm_f32_vec.data = 1;
        vm_f32_vec.eval(&code);

        let res = vm_f32.stack[1];
        assert_eq!(res, vm_range.stack[1].l);
        assert_eq!(res, vm_range.stack[1].u);
        assert_eq!(res, vm_f32_vec.take_stack(1)[0]);
    }

    #[test]
    fn pow() {
        let pow = [op::POW_LHS_RHS(1, 2, 1), op::EXT(0)];

        let mut vm = VM::<Range>::new();
        vm.set_instr_table(RangeInstrTable);

        vm.reg[1] = (-3.0, -2.0).into();
        vm.reg[2] = (2.0, 2.0).into();
        vm.eval(&pow);
        assert_eq!(vm.reg[1], (4.0, 9.0).into());

        vm.reg[1] = (-2.0, 3.0).into();
        vm.reg[2] = (3.0, 3.0).into();
        vm.eval(&pow);
        assert_eq!(vm.reg[1], (-8.0, 27.0).into());

        vm.reg[1] = (0.5, 2.0).into();
        vm.reg[2] = (0.5, 2.0).into();
        vm.eval(&pow);
        assert_eq!(vm.reg[1], (0.25, 4.0).into());
    }
}
