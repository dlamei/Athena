use std::{
    fmt,
    ops::{self},
    rc::Rc,
};

use paste::paste;

use utils::{ExplicitCopy, Intrvl};

pub type Opcode = u64;
pub type Address = usize;

const UNROLL_STEP: usize = 4;

pub type float = f64;
const PI: float = std::f64::consts::PI;
const HALF_PI: float = std::f64::consts::FRAC_PI_2;
const THREE_HALVES_PI: float = 3.0 * HALF_PI;
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
                pub const fn [<$OP _REG_REG>](lhs: u8, rhs: u8, out: u8) -> Opcode {
                    build_opcode_float([<OP_ $OP>], lhs, rhs, out, 0.0)
                }

                #[allow(non_snake_case)]
                pub const fn [<$OP _IMM_REG>](lhs: float, rhs: u8, out: u8) -> Opcode {
                    build_opcode_float([<OP_ $OP>], 0, rhs, out, lhs)
                }

                #[allow(non_snake_case)]
                pub const fn [<$OP _REG_IMM>](lhs: u8, rhs: float, out: u8) -> Opcode {
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

    #[allow(non_snake_case)]
    pub const fn EXP(lhs: u8, out: u8) -> Opcode {
        POW_REG_IMM(lhs, E, out)
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

#[derive(Debug, Clone)]
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

    fn skip_to_end(&mut self) {
        self.pc = usize::MAX;
    }

    fn reg(&self, reg: usize) -> &WORD {
        &self.reg[reg]
    }

    fn reg_mut(&mut self, reg: usize) -> &mut WORD {
        &mut self.reg[reg]
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
            // self.reg[l as usize].clone()
            self.reg(l).clone()
        };

        (lhs, out)
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
            // self.reg[l as usize].clone()
            self.reg(l).clone()
        };

        let rhs = if r == 0 {
            // float::from_bits(imm)
            // op::float_from_imm(imm)
            VmWord::from_imm(imm)
        } else {
            //self.reg[r as usize].clone()
            self.reg(r).clone()
        };

        (lhs, rhs, out)
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
        if self.pc == usize::MAX {
            log::trace!("abort!");
            self.instr_table[op::OP_EXT as usize](self, t);
            return;
        }

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
        *vm.reg_mut(out) = lhs + rhs;
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn sub(vm: &mut VM<f64>, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        *vm.reg_mut(out) = lhs - rhs;
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn mul(vm: &mut VM<f64>, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        *vm.reg_mut(out) = lhs * rhs;
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn div(vm: &mut VM<f64>, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        *vm.reg_mut(out) = lhs / rhs;
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn pow(vm: &mut VM<f64>, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        *vm.reg_mut(out) = lhs.powf(rhs);
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn sin(vm: &mut VM<f64>, t: &InstrTape) {
        let (val, out) = vm.unary_arg(t);
        *vm.reg_mut(out) = val.sin();
        vm.next(t);
    }

    fn cos(vm: &mut VM<f64>, t: &InstrTape) {
        let (val, out) = vm.unary_arg(t);
        *vm.reg_mut(out) = val.cos();
        vm.next(t);
    }

    fn tan(vm: &mut VM<f64>, t: &InstrTape) {
        let (val, out) = vm.unary_arg(t);
        *vm.reg_mut(out) = val.tan();
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
        *vm.reg_mut(out) = val;
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
        *vm.reg_mut(out) = vm.stack_pop();
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
pub struct F64Deriv {
    pub val: f64,
    pub grad: f64,
}

impl F64Deriv {
    pub fn var(val: f64) -> Self {
        Self { val, grad: 1.0 }
    }

    pub fn cnst(val: f64) -> Self {
        Self { val, grad: 0.0 }
    }
}

impl VmWord for F64Deriv {
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

pub struct F64DerivInstrTable;

impl InstrTable<VM<F64Deriv>> for F64DerivInstrTable {
    fn add(vm: &mut VM<F64Deriv>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = F64Deriv {
            val: a.val + b.val,
            grad: a.grad + b.grad,
        };
        *vm.reg_mut(out) = c;
        vm.next(t);
    }

    fn sub(vm: &mut VM<F64Deriv>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = F64Deriv {
            val: a.val - b.val,
            grad: a.grad - b.grad,
        };
        *vm.reg_mut(out) = c;
        vm.next(t);
    }

    fn mul(vm: &mut VM<F64Deriv>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = F64Deriv {
            val: a.val * b.val,
            grad: a.val * b.grad + a.grad * b.val,
        };
        *vm.reg_mut(out) = c;
        vm.next(t);
    }

    fn div(vm: &mut VM<F64Deriv>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let (da, db) = (a.grad, b.grad);
        let (a, b) = (a.val, b.val);
        let c = F64Deriv {
            val: a / b,
            grad: (b * da - a * db) / b.powf(2.0),
        };
        *vm.reg_mut(out) = c;
        vm.next(t);
    }

    fn pow(vm: &mut VM<F64Deriv>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let (da, db) = (a.grad, b.grad);
        let (a, b) = (a.val, b.val);
        let c = F64Deriv {
            val: a.powf(b),
            grad: b * a.powf(b - 1.0) * da + a.powf(b) * a.ln() * db,
        };
        *vm.reg_mut(out) = c;
        vm.next(t);
    }

    fn sin(vm: &mut VM<F64Deriv>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        let c = F64Deriv {
            val: a.val.sin(),
            grad: a.val.cos() * a.grad,
        };
        *vm.reg_mut(out) = c;
        vm.next(t);
    }

    fn cos(vm: &mut VM<F64Deriv>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        let c = F64Deriv {
            val: a.val.cos(),
            grad: -a.val.sin() * a.grad,
        };
        *vm.reg_mut(out) = c;
        vm.next(t);
    }

    fn tan(vm: &mut VM<F64Deriv>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        let c = F64Deriv {
            val: a.val.tan(),
            grad: a.grad * 1.0 / a.val.cos().powf(2.0),
        };
        *vm.reg_mut(out) = c;
        vm.next(t);
    }

    fn out(vm: &mut VM<F64Deriv>, t: &InstrTape) {
        let (a, _) = vm.unary_arg(t);
        println!("{a:?}");
        vm.next(t);
    }

    fn mov(vm: &mut VM<F64Deriv>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        *vm.reg_mut(out) = a;
        vm.next(t);
    }

    fn psh(vm: &mut VM<F64Deriv>, t: &InstrTape) {
        let (a, _) = vm.unary_arg(t);
        vm.stack_push(a);
        vm.next(t);
    }

    fn pop(vm: &mut VM<F64Deriv>, t: &InstrTape) {
        let (_, out) = vm.unary_arg(t);
        *vm.reg_mut(out) = vm.stack_pop();
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

impl VmWord for Intrvl {
    type Data = ();

    fn from_imm(imm: u32) -> Self {
        let imm = op::float_from_imm(imm);
        Intrvl::scalar(imm)
    }

    fn uninit() -> Self {
        Intrvl::UNDEF
    }
}

pub struct RangeInstrTable;

impl InstrTable<VM<Range>> for RangeInstrTable {
    fn add(vm: &mut VM<Range>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = Range::of_add(a, b);
        *vm.reg_mut(out) = c;
        log::debug!("add({a}, {b}) = {c}");
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t)
    }

    fn sub(vm: &mut VM<Range>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = Range::of_sub(a, b);
        *vm.reg_mut(out) = c;
        log::debug!("sub({a}, {b}) = {c}");
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t)
    }

    fn mul(vm: &mut VM<Range>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = Range::of_mul(a, b);
        *vm.reg_mut(out) = c;
        log::debug!("mul({a}, {b}) = {c}");
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t)
    }

    fn div(vm: &mut VM<Range>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = Range::of_div(a, b);

        *vm.reg_mut(out) = c;
        log::debug!("div({a}, {b}) = {c}");
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t)
    }

    fn pow(vm: &mut VM<Range>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = Range::of_pow(a, b);
        log::debug!("pow({a}, {b}) = {c}");
        *vm.reg_mut(out) = c;
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t)
    }

    fn sin(vm: &mut VM<Range>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        let b = Range::of_sin(a);
        *vm.reg_mut(out) = b;
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t)
    }

    fn cos(vm: &mut VM<Range>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        let b = Range::of_cos(a);
        *vm.reg_mut(out) = b;
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t)
    }

    fn tan(vm: &mut VM<Range>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        let b = Range::of_tan(a);
        *vm.reg_mut(out) = b;
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t)
    }

    fn out(vm: &mut VM<Range>, t: &InstrTape) {
        let (val, _) = vm.unary_arg(t);
        println!("{val}");
        vm.next(t)
    }

    fn mov(vm: &mut VM<Range>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        log::trace!("   {a} -> {out}");
        *vm.reg_mut(out) = a;
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
        *vm.reg_mut(out) = vm.stack_pop();
        vm.next(t)
    }
}

pub struct IntrvlInstrTable;

impl InstrTable<VM<Intrvl>> for IntrvlInstrTable {
    fn add(vm: &mut VM<Intrvl>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = Intrvl::add(a, b);
        *vm.reg_mut(out) = c;
        log::debug!("add({a}, {b}) = {c}");
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t)
    }

    fn sub(vm: &mut VM<Intrvl>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = Intrvl::sub(a, b);
        *vm.reg_mut(out) = c;
        log::debug!("sub({a}, {b}) = {c}");
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t)
    }

    fn mul(vm: &mut VM<Intrvl>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = Intrvl::mul(a, b);
        *vm.reg_mut(out) = c;
        log::debug!("mul({a}, {b}) = {c}");
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t)
    }

    fn div(vm: &mut VM<Intrvl>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = Intrvl::div(a, b);

        *vm.reg_mut(out) = c;
        log::debug!("div({a}, {b}) = {c}");
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t)
    }

    fn pow(vm: &mut VM<Intrvl>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = Intrvl::pow(a, b);
        log::debug!("pow({a}, {b}) = {c}");
        *vm.reg_mut(out) = c;
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t)
    }

    fn sin(vm: &mut VM<Intrvl>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        let b = Intrvl::sin(a);
        *vm.reg_mut(out) = b;
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t)
    }

    fn cos(vm: &mut VM<Intrvl>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        let b = Intrvl::cos(a);
        *vm.reg_mut(out) = b;
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t)
    }

    fn tan(vm: &mut VM<Intrvl>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        let b = Intrvl::tan(a);
        *vm.reg_mut(out) = b;
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t)
    }

    fn out(vm: &mut VM<Intrvl>, t: &InstrTape) {
        let (val, _) = vm.unary_arg(t);
        println!("{val}");
        vm.next(t)
    }

    fn mov(vm: &mut VM<Intrvl>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        log::trace!("   {a} -> {out}");
        *vm.reg_mut(out) = a;
        vm.next(t)
    }

    fn psh(vm: &mut VM<Intrvl>, t: &InstrTape) {
        let (val, _) = vm.unary_arg(t);
        vm.stack_push(val);
        log::trace!("   {} -> stack[{}]", vm.stack[vm.sp], vm.sp);
        vm.next(t)
    }

    fn pop(vm: &mut VM<Intrvl>, t: &InstrTape) {
        let (_, out) = vm.unary_arg(t);
        *vm.reg_mut(out) = vm.stack_pop();
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
            val: self.val.cos(),
            grad: Range::MINUS_ONE.mul(self.val.sin()).mul(self.grad),
        }
    }

    pub fn tan_deriv(self) -> Self {
        Self {
            val: self.val.tan(),
            grad: Range::ONE
                .div(self.val.cos().pow(Range::TWO))
                .mul(self.grad),
        }
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
        *vm.reg_mut(out) = c;
        vm.next(t);
    }

    fn sub(vm: &mut VM<RangeDeriv>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = a.sub_deriv(b);
        *vm.reg_mut(out) = c;
        vm.next(t);
    }

    fn mul(vm: &mut VM<RangeDeriv>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = a.mul_deriv(b);
        *vm.reg_mut(out) = c;
        vm.next(t);
    }

    fn div(vm: &mut VM<RangeDeriv>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = a.div_deriv(b);
        *vm.reg_mut(out) = c;
        vm.next(t);
    }

    fn pow(vm: &mut VM<RangeDeriv>, t: &InstrTape) {
        let (a, b, out) = vm.binop_arg(t);
        let c = a.pow_deriv(b);
        *vm.reg_mut(out) = c;
        vm.next(t);
    }

    fn sin(vm: &mut VM<RangeDeriv>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        let c = a.sin_deriv();
        *vm.reg_mut(out) = c;
        vm.next(t);
    }

    fn cos(vm: &mut VM<RangeDeriv>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        let c = a.cos_deriv();
        *vm.reg_mut(out) = c;
        vm.next(t);
    }

    fn tan(vm: &mut VM<RangeDeriv>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        let c = a.tan_deriv();
        *vm.reg_mut(out) = c;
        vm.next(t);
    }

    fn out(vm: &mut VM<RangeDeriv>, t: &InstrTape) {
        let (a, _) = vm.unary_arg(t);
        println!("{a:?}");
        vm.next(t);
    }

    fn mov(vm: &mut VM<RangeDeriv>, t: &InstrTape) {
        let (a, out) = vm.unary_arg(t);
        *vm.reg_mut(out) = a;
        vm.next(t);
    }

    fn psh(vm: &mut VM<RangeDeriv>, t: &InstrTape) {
        let (a, _) = vm.unary_arg(t);
        vm.stack_push(a);
        vm.next(t);
    }

    fn pop(vm: &mut VM<RangeDeriv>, t: &InstrTape) {
        let (_, out) = vm.unary_arg(t);
        *vm.reg_mut(out) = vm.stack_pop();
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
                    vec
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
                    vec
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
        *vm.reg_mut(out) = lhs.add(&rhs);
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn sub(vm: &mut VM<F64Vec>, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        *vm.reg_mut(out) = lhs.sub(&rhs);
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn mul(vm: &mut VM<F64Vec>, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        *vm.reg_mut(out) = lhs.mul(&rhs);
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn div(vm: &mut VM<F64Vec>, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        *vm.reg_mut(out) = lhs.div(&rhs);
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn pow(vm: &mut VM<F64Vec>, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        *vm.reg_mut(out) = lhs.pow(&rhs);
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn sin(vm: &mut VM<F64Vec>, t: &InstrTape) {
        let (lhs, out) = vm.unary_arg(t);
        *vm.reg_mut(out) = lhs.sin();
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn cos(vm: &mut VM<F64Vec>, t: &InstrTape) {
        let (lhs, out) = vm.unary_arg(t);
        *vm.reg_mut(out) = lhs.cos();
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn tan(vm: &mut VM<F64Vec>, t: &InstrTape) {
        let (lhs, out) = vm.unary_arg(t);
        *vm.reg_mut(out) = lhs.tan();
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
        *vm.reg_mut(out) = val.clone();
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
        *vm.reg_mut(out) = vm.stack_pop();
        vm.next(t);
    }
}

pub struct RangeVecInstrTable;

impl VM<RangeVec> {
    pub fn set_vec_size(&mut self, size: usize) {
        self.data = size;
    }

    pub fn take_reg(&mut self, indx: usize) -> Vec<Range> {
        let res = self.reg[indx].clone();

        self.clear_memory();

        match res {
            RangeVec::Vec(vec) => {
                if let Ok(vec) = Rc::try_unwrap(vec.clone()) {
                    vec
                } else {
                    (*vec).clone()
                }
            }
            RangeVec::Imm(imm) => vec![imm; self.data],
        }
    }
    pub fn take_stack(&mut self, indx: usize) -> Vec<Range> {
        let res = self.stack[indx].clone();

        self.clear_memory();

        match res {
            RangeVec::Vec(vec) => {
                if let Ok(vec) = Rc::try_unwrap(vec.clone()) {
                    vec
                } else {
                    (*vec).clone()
                }
            }
            RangeVec::Imm(imm) => vec![imm; self.data],
        }
    }
}

impl InstrTable<VM<RangeVec>> for RangeVecInstrTable {
    fn add(vm: &mut VM<RangeVec>, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        *vm.reg_mut(out) = lhs.add(&rhs);
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn sub(vm: &mut VM<RangeVec>, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        *vm.reg_mut(out) = lhs.sub(&rhs);
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn mul(vm: &mut VM<RangeVec>, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        *vm.reg_mut(out) = lhs.mul(&rhs);
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn div(vm: &mut VM<RangeVec>, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        *vm.reg_mut(out) = lhs.div(&rhs);
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn pow(vm: &mut VM<RangeVec>, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        *vm.reg_mut(out) = lhs.pow(&rhs);
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn sin(vm: &mut VM<RangeVec>, t: &InstrTape) {
        let (lhs, out) = vm.unary_arg(t);
        *vm.reg_mut(out) = lhs.sin();
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn cos(vm: &mut VM<RangeVec>, t: &InstrTape) {
        let (lhs, out) = vm.unary_arg(t);
        *vm.reg_mut(out) = lhs.cos();
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn tan(vm: &mut VM<RangeVec>, t: &InstrTape) {
        let (lhs, out) = vm.unary_arg(t);
        *vm.reg_mut(out) = lhs.tan();
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        vm.next(t);
    }

    fn out(vm: &mut VM<RangeVec>, t: &InstrTape) {
        let (val, _) = vm.unary_arg(t);
        log::trace!("{val}");
        vm.next(t)
    }

    fn mov(vm: &mut VM<RangeVec>, t: &InstrTape) {
        let (val, out) = vm.unary_arg(t);
        *vm.reg_mut(out) = val.clone();
        log::trace!("   {val} -> {out}");
        vm.next(t);
    }

    fn psh(vm: &mut VM<RangeVec>, t: &InstrTape) {
        let (val, _) = vm.unary_arg(t);
        vm.stack_push(val);
        log::trace!("   {} -> stack[{}]", vm.stack[vm.sp], vm.sp);
        vm.next(t);
    }

    fn pop(vm: &mut VM<RangeVec>, t: &InstrTape) {
        let (_, out) = vm.unary_arg(t);
        *vm.reg_mut(out) = vm.stack_pop();
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

macro_rules! op_imm_vec {
    ($typ:ident, $lhs:ident : $imm:expr, $rhs:ident : $vec:expr => $body:block) => {{
        let vec = $vec;
        // let mut res = vec![0.0; vec.len()];

        let $lhs = $imm.copy();
        let mut res = vec![Default::default(); vec.len()];
        let unrolled_len = vec.len() / UNROLL_STEP;
        let rem_len = vec.len() - unrolled_len * UNROLL_STEP;

        for j in 0..unrolled_len {
            for k in 0..UNROLL_STEP {
                let i = j * UNROLL_STEP + k;
                let $rhs = vec[i];
                res[i] = { $body };
            }
        }
        for i in 0..rem_len {
            let $rhs = vec[i];
            res[i] = { $body };
        }

        // let res: Vec<_> = vec
        //     .iter()
        //     .map(|r| {
        //         let $rhs = r.copy();
        //         $body
        //     })
        //     .collect();

        $typ::Vec(res.into())
    }};
}

macro_rules! op_vec_imm {
    ($typ:ident, $lhs:ident: $vec:expr, $rhs:ident: $imm:expr => $body:block) => {{
        let vec = $vec;
        // let mut res = vec![0.0; vec.len()];

        let $rhs = $imm.copy();
        let mut res = vec![Default::default(); vec.len()];
        let unrolled_len = vec.len() / UNROLL_STEP;
        let rem_len = vec.len() - unrolled_len * UNROLL_STEP;

        for j in 0..unrolled_len {
            for k in 0..UNROLL_STEP {
                let i = j * UNROLL_STEP + k;
                let $lhs = vec[i];
                res[i] = { $body };
            }
        }
        for i in 0..rem_len {
            let $lhs = vec[i];
            res[i] = { $body };
        }

        // let res: Vec<_> = vec
        //     .iter()
        //     .map(|l| {
        //         let $lhs = l.copy();
        //         $body
        //     })
        //     .collect();

        $typ::Vec(res.into())
    }};
}

macro_rules! op_vec_vec {
    ($typ:ident, $lhs:ident: $vec1:expr, $rhs:ident: $vec2:expr => $body:block) => {{
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
        let mut res = vec![Default::default(); vec1.len()];
        let unrolled_len = vec1.len() / UNROLL_STEP;
        let rem_len = vec1.len() - unrolled_len * UNROLL_STEP;

        for j in 0..unrolled_len {
            for k in 0..UNROLL_STEP {
                let i = UNROLL_STEP * j + k;
                let $lhs = $vec1[i];
                let $rhs = $vec2[i];
                res[i] = { $body };
            }
        }
        for i in 0..rem_len {
            let $lhs = $vec1[i];
            let $rhs = $vec2[i];
            res[i] = $body;
        }

        // let res: Vec<_> = vec1
        //     .iter()
        //     .zip(vec2.iter())
        //     .map(|(l, r)| {
        //         let $lhs = l.copy();
        //         let $rhs = r.copy();
        //         $body
        //     })
        //     .collect();

        $typ::Vec(res.into())
    }};
}

macro_rules! impl_vec_op {
    ($typ:ident, $lhs:ident: $fvec1:expr, $rhs:ident: $fvec2:expr => $body:block) => {{
        match ($fvec1, $fvec2) {
            ($typ::Vec(v1), $typ::Vec(v2)) => op_vec_vec!($typ, $lhs: v1,$rhs: v2 => $body),
            ($typ::Vec(v), $typ::Imm(i)) => op_vec_imm!($typ, $lhs: v, $rhs: i => $body),
            ($typ::Imm(i), $typ::Vec(v)) => op_imm_vec!($typ, $lhs: i, $rhs: v => $body),
            ($typ::Imm(i1), $typ::Imm(i2)) => {
                let $lhs = i1.copy();
                let $rhs = i2.copy();
                $typ::Imm($body)
            }
        }
    }};

    ($typ:ident, $val:ident: $fvec:expr => $body:block) => {{
        match $fvec {
            $typ::Vec(vec) => {
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

                $typ::Vec(res.into())
            }
            $typ::Imm(i) => {
                let $val = i.copy();
                $typ::Imm($body)
            }
        }
    }};
}

impl F64Vec {
    #[inline(always)]
    pub fn add(&self, other: &Self) -> Self {
        impl_vec_op!(F64Vec, lhs: self, rhs: other => { lhs + rhs })
    }
    #[inline(always)]
    pub fn sub(&self, other: &Self) -> Self {
        impl_vec_op!(F64Vec, lhs: self, rhs: other => { lhs - rhs })
    }
    #[inline(always)]
    pub fn mul(&self, other: &Self) -> Self {
        impl_vec_op!(F64Vec, lhs: self, rhs: other => { lhs * rhs })
    }
    #[inline(always)]
    pub fn div(&self, other: &Self) -> Self {
        impl_vec_op!(F64Vec, lhs: self, rhs: other => { lhs / rhs })
    }
    #[inline(always)]
    pub fn pow(&self, other: &Self) -> Self {
        impl_vec_op!(F64Vec, lhs: self, rhs: other => { lhs.powf(rhs) })
    }
    #[inline(always)]
    pub fn sin(&self) -> Self {
        impl_vec_op!(F64Vec, v: self => { v.sin() })
    }
    #[inline(always)]
    pub fn cos(&self) -> Self {
        impl_vec_op!(F64Vec, v: self => { v.cos() })
    }
    #[inline(always)]
    pub fn tan(&self) -> Self {
        impl_vec_op!(F64Vec, v: self => { v.tan() })
    }
}

#[inline(always)]
const fn min_max_2(a: float, b: float) -> (float, float) {
    if a < b { (a, b) } else { (b, a) }
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
    pub const TWO: Self = Self::new_const(2.0);

    // no single continous range
    // pub const NON_CONTINUOUS: Self = Self {
    //     l: float::INFINITY,
    //     u: float::NEG_INFINITY,
    // };

    #[inline(always)]
    pub fn new(l: float, u: float) -> Self {
        // let check = l <= u || (l.is_nan() && u.is_nan());
        // debug_assert!(check, "{l} <= {u}");
        // if !check {
        if l > u {
            let mid = (l + u) / 2.0;
            Range { u: mid, l: mid }
        } else {
            Range { l, u }
        }
        // }
        // Range { l, u }
    }

    #[inline(always)]
    pub fn imm(imm: float) -> Self {
        Range { l: imm, u: imm }
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
    pub fn tan(self) -> Self {
        Self::of_tan(self)
    }
    #[inline(always)]
    pub fn ln(self) -> Self {
        Self::of_ln(self)
    }

    // pub fn of_sin(a: Range) -> Self {
    //     if a.is_undef() {
    //         return Self::UNDEF;
    //     } else if a.dist() >= TWO_PI {
    //         return (-1.0, 1.0).into();
    //     }

    //     let l = a.l.rem_euclid(TWO_PI);
    //     let u = a.u.rem_euclid(TWO_PI);

    //     // Check if π/2 or 3π/2 are contained in the interval (accounting for wrapping)
    //     let contains_half_pi =
    //         (l <= HALF_PI && HALF_PI <= u) || (l > u && (HALF_PI >= l || HALF_PI <= u));
    //     let contains_three_halves_pi = (l <= THREE_HALVES_PI && THREE_HALVES_PI <= u)
    //         || (l > u && (THREE_HALVES_PI >= l || THREE_HALVES_PI <= u));

    //     let min = if contains_three_halves_pi {
    //         -1.0
    //     } else {
    //         l.sin().min(u.sin())
    //     };
    //     let max = if contains_half_pi {
    //         1.0
    //     } else {
    //         l.sin().max(u.sin())
    //     };

    //     (min, max).into()
    // }
    #[inline(always)]
    pub fn of_sin(a: Range) -> Self {
        if a.is_empty() {
            return Self::UNDEF;
        } else if a.dist() >= TWO_PI {
            return (-1.0, 1.0).into();
        }

        let l = a.l.rem_euclid(TWO_PI);
        let u = a.u.rem_euclid(TWO_PI);

        let contains_half_pi =
            (l <= HALF_PI && HALF_PI <= u) || (l > u && (HALF_PI >= l || HALF_PI <= u));
        let contains_three_halves_pi = (l <= THREE_HALVES_PI && THREE_HALVES_PI <= u)
            || (l > u && (THREE_HALVES_PI >= l || THREE_HALVES_PI <= u));

        // let min = if contains_three_halves_pi {
        //     -1.0
        // } else {
        //     l.cos().min(u.cos())
        // };
        // let max = if contains_half_pi {
        //     1.0
        // } else {
        //     l.cos().max(u.cos())
        // };

        if contains_three_halves_pi && contains_half_pi {
            (-1.0, 1.0)
        } else if contains_three_halves_pi && !contains_half_pi {
            (-1.0, l.sin().max(u.sin()))
        } else if !contains_three_halves_pi && contains_half_pi {
            (l.sin().min(u.sin()), 1.0)
        } else {
            let ls = l.sin();
            let us = u.sin();
            min_max_2(ls, us)
        }
        .into()
    }

    pub fn of_cos(a: Range) -> Self {
        if a.is_empty() {
            return Self::UNDEF;
        } else if a.dist() >= TWO_PI {
            return (-1.0, 1.0).into();
        }

        let l = a.l.rem_euclid(TWO_PI);
        let u = a.u.rem_euclid(TWO_PI);

        let contains_zero = (l <= 0.0 && 0.0 <= u) || (l > u && (0.0 >= l || 0.0 <= u));
        let contains_pi = (l <= PI && PI <= u) || (l > u && (PI >= l || PI <= u));

        let min = if contains_pi {
            -1.0
        } else {
            l.cos().min(u.cos())
        };
        let max = if contains_zero {
            1.0
        } else {
            l.cos().max(u.cos())
        };

        if contains_pi && contains_zero {
            (-1.0, 1.0)
        } else if contains_pi && !contains_zero {
            (-1.0, l.cos().max(u.cos()))
        } else if !contains_pi && contains_zero {
            (l.cos().min(u.cos()), 1.0)
        } else {
            let lc = l.cos();
            let uc = u.cos();
            min_max_2(lc, uc)
        }
        .into()
    }

    // pub fn of_tan(a: Range) -> Self {
    //     if a.is_undef() {
    //         return Range::UNDEF;
    //     }

    //     let res = a.sin().div(a.cos());
    //     println!("{a}: {res}");
    //     res
    // }

    pub fn of_tan(a: Range) -> Self {
        if a.is_empty() {
            return Self::UNDEF;
        } else if a.dist() >= PI {
            return Self::UNDEF; // Intervals >= π always contain an asymptote
        }

        let l = a.l.rem_euclid(PI);
        let u = a.u.rem_euclid(PI);

        // Check if π/2 is in the interval (accounting for wrapping)
        let contains_half_pi =
            (l <= HALF_PI && HALF_PI <= u) || (l > u && (HALF_PI >= l || HALF_PI <= u));

        if contains_half_pi {
            Self::UNDEF
        } else {
            let min = a.l.tan().min(a.u.tan());
            let max = a.l.tan().max(a.u.tan());
            (min, max).into()
        }
    }

    // #[inline(always)]
    // pub fn of_sin(a: Range) -> Self {
    //     if a.is_undef() {
    //         return Self::UNDEF;
    //     } else if a.dist() >= TWO_PI {
    //         return (-1.0, 1.0).into();
    //     }

    //     let mut l = a.l.rem(TWO_PI);
    //     let mut u = a.u.rem(TWO_PI);

    //     let mut min = a.l.sin();
    //     let mut max = a.u.sin();

    //     if min > max {
    //         (min, max) = (max, min);
    //     }

    //     if l <= u {
    //         if l <= HALF_PI && u >= HALF_PI {
    //             max = 1.0;
    //         } else if l <= 3.0 * HALF_PI && u >= 3.0 * HALF_PI {
    //             min = -1.0;
    //         }
    //     } else {
    //         min = -1.0;
    //         max = 1.0;
    //     }

    //     (min, max).into()
    // }

    // #[inline(always)]
    // pub fn of_cos(a: Range) -> Self {
    //     if a.is_undef() {
    //         return Self::UNDEF;
    //     } else if a.dist() >= TWO_PI {
    //         return (-1.0, 1.0).into();
    //     }

    //     let mut l = a.l.rem_euclid(TWO_PI);
    //     let u = a.u.rem_euclid(TWO_PI);

    //     // if l < u {
    //     //     l -= TWO_PI;
    //     // }

    //     let mut min = a.l.cos();
    //     let mut max = a.u.cos();

    //     if min > max {
    //         (min, max) = (max, min);
    //     }

    //     if l <= u {
    //         if l == 0.0 {
    //             max = 1.0;
    //         }
    //         if l <= PI && u >= PI {
    //             min = -1.0;
    //         }
    //     } else {
    //         max = 1.0;
    //         if !(u < PI && PI < l) {
    //             min = -1.0;
    //         }
    //     }

    //     (min, max).into()
    // }

    #[inline(always)]
    pub fn of_add(a: Range, b: Range) -> Self {
        if a.is_empty() || b.is_empty() {
            return Self::UNDEF;
        }
        (a.l + b.l, a.u + b.u).into()
    }

    #[inline(always)]
    pub fn of_sub(a: Range, b: Range) -> Self {
        if a.is_empty() || b.is_empty() {
            return Self::UNDEF;
        }
        (a.l - b.u, a.u - b.l).into()
    }

    #[inline(always)]
    pub fn of_mul(a: Range, b: Range) -> Self {
        if a.is_empty() || b.is_empty() {
            return Self::UNDEF;
        }
        let res = min_max_4(a.l * b.l, a.l * b.u, a.u * b.l, a.u * b.u);

        if res.0.is_nan() || res.1.is_nan() {
            return Self::UNDEF;
        }
        res.into()
    }

    #[inline(always)]
    pub fn of_div(a: Range, b: Range) -> Self {
        // 1 / b

        let denom = if !b.contains_zero() {
            (1.0 / b.u, 1.0 / b.l)
        } else if b.u == 0.0 {
            (f64::NEG_INFINITY, 1.0 / b.l)
        } else if b.l == 0.0 {
            (1.0 / b.u, f64::INFINITY)
        } else {
            return Range::UNDEF;
        };

        Self::of_mul(a, denom.into())

        // if a.is_undef() || b.is_undef() || (a.contains_zero() && b.contains_zero()) {
        //     return Self::UNDEF;
        // }
        // min_max_4(a.l / b.l, a.l / b.u, a.u / b.l, a.u / b.u).into()
    }

    #[inline(always)]
    pub fn of_pow(a: Range, b: Range) -> Self {
        if a.is_empty() || b.is_empty() {
            return Self::UNDEF;
        }

        if a.l >= 0.0 {
            // a >= 0
            if a.l > 0.0 {
                // a > 0
                Range::from_tuple(min_max_4(
                    a.l.powf(b.l),
                    a.u.powf(b.u),
                    a.u.powf(b.l),
                    a.l.powf(b.u),
                ))
            } else if !b.contains_zero() {
                Range::new(0.0, a.u.powf(b.u))
            } else {
                Range::UNDEF
            }
        } else if a.u < 0.0 {
            // a < 0
            if b.is_const_int() {
                let b = b.l;

                Range::from_tuple(min_max_2(a.u.powf(b), a.l.powf(b)))
            } else {
                Range::UNDEF
            }
        } else if b.is_const_int() {
            let b = b.l;

            if (b % 2.0).abs() < float::EPSILON {
                Range::new(0.0, a.l.abs().max(a.u).powf(b))
            } else {
                Range::new(a.l.powf(b), a.l.abs().max(a.u).powf(b))
            }
        } else {
            Range::UNDEF
        }
    }

    #[inline(always)]
    pub fn of_ln(a: Range) -> Self {
        if a.is_empty() || a.l <= 0.0 {
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
        (self.l - self.u).abs() < float::EPSILON
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
    pub fn is_empty(&self) -> bool {
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
        self.u - self.l
        // (self.l.powf(2.0) + self.u.powf(2.0)).sqrt()
    }
}

impl ops::Add for Range {
    type Output = Range;

    fn add(self, rhs: Self) -> Self::Output {
        Self::of_add(self, rhs)
    }
}
impl ops::Sub for Range {
    type Output = Range;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::of_sub(self, rhs)
    }
}
impl ops::Mul for Range {
    type Output = Range;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::of_mul(self, rhs)
    }
}
impl ops::Div for Range {
    type Output = Range;

    fn div(self, rhs: Self) -> Self::Output {
        Self::of_div(self, rhs)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RangeVec {
    Vec(Rc<Vec<Range>>),
    Imm(Range),
}

impl VmWord for RangeVec {
    type Data = usize;

    fn from_imm(imm: u32) -> Self {
        let v = op::float_from_imm(imm);
        RangeVec::Imm(Range::imm(v))
    }

    fn uninit() -> Self {
        RangeVec::Imm(Range::UNDEF)
    }
}

impl fmt::Display for RangeVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RangeVec::Vec(vec) => write!(f, "{vec:?}"),
            RangeVec::Imm(i) => write!(f, "{i}"),
        }
    }
}

impl Default for RangeVec {
    fn default() -> Self {
        0.0.into()
    }
}

impl From<Range> for RangeVec {
    fn from(value: Range) -> Self {
        RangeVec::Imm(value)
    }
}

impl From<float> for RangeVec {
    fn from(value: float) -> Self {
        Range::imm(value).into()
    }
}

impl From<Vec<Range>> for RangeVec {
    fn from(value: Vec<Range>) -> Self {
        Self::Vec(value.into())
    }
}

impl RangeVec {
    #[inline(always)]
    pub fn add(&self, other: &Self) -> Self {
        impl_vec_op!(RangeVec, lhs: self, rhs: other => { lhs + rhs })
    }
    #[inline(always)]
    pub fn sub(&self, other: &Self) -> Self {
        impl_vec_op!(RangeVec, lhs: self, rhs: other => { lhs - rhs })
    }
    #[inline(always)]
    pub fn mul(&self, other: &Self) -> Self {
        impl_vec_op!(RangeVec, lhs: self, rhs: other => { lhs * rhs })
    }
    #[inline(always)]
    pub fn div(&self, other: &Self) -> Self {
        impl_vec_op!(RangeVec, lhs: self, rhs: other => { lhs / rhs })
    }
    #[inline(always)]
    pub fn pow(&self, other: &Self) -> Self {
        impl_vec_op!(RangeVec, lhs: self, rhs: other => { lhs.pow(rhs) })
    }
    #[inline(always)]
    pub fn sin(&self) -> Self {
        impl_vec_op!(RangeVec, v: self => { v.sin() })
    }
    #[inline(always)]
    pub fn cos(&self) -> Self {
        impl_vec_op!(RangeVec, v: self => { v.cos() })
    }
    #[inline(always)]
    pub fn tan(&self) -> Self {
        impl_vec_op!(RangeVec, v: self => { v.tan() })
    }
}

pub mod simd {
    use super::*;
    use wide::f64x4;

    const SIMD_WIDTH: usize = 4;

    #[derive(Debug, Clone, PartialEq)]
    pub enum F64x4Vec {
        Vec(Rc<Vec<f64x4>>),
        Imm(f64x4),
    }

    impl VmWord for F64x4Vec {
        type Data = usize;

        fn from_imm(imm: u32) -> Self {
            let v = op::float_from_imm(imm);
            Self::Imm(f64x4::splat(v))
        }

        fn uninit() -> Self {
            Self::Imm(f64x4::ZERO)
        }
    }

    impl Default for F64x4Vec {
        fn default() -> Self {
            0.0.into()
        }
    }

    impl From<f64> for F64x4Vec {
        fn from(value: f64) -> Self {
            Self::Imm(f64x4::splat(value))
        }
    }

    impl From<&Vec<f64>> for F64x4Vec {
        fn from(vec: &Vec<f64>) -> Self {
            let mut simd_vec = Vec::with_capacity(vec.len().div_ceil(SIMD_WIDTH));

            let mut i = 0;
            while i + SIMD_WIDTH <= vec.len() {
                let v: [f64; SIMD_WIDTH] = vec[i..i + SIMD_WIDTH].try_into().unwrap();
                simd_vec.push(f64x4::new(v));
                i += SIMD_WIDTH;
            }

            if i < vec.len() {
                let mut remainder = [0.0; SIMD_WIDTH];

                for j in 0..(vec.len() - i) {
                    remainder[j] = vec[i + j];
                }

                simd_vec.push(f64x4::new(remainder));
            }

            Self::Vec(simd_vec.into())
        }
    }

    impl F64x4Vec {
        #[inline]
        pub fn add(&self, other: &Self) -> Self {
            impl_vec_op!(F64x4Vec, lhs: self, rhs: other => { lhs + rhs })
        }
        #[inline]
        pub fn sub(&self, other: &Self) -> Self {
            impl_vec_op!(F64x4Vec, lhs: self, rhs: other => { lhs - rhs })
        }
        #[inline]
        pub fn mul(&self, other: &Self) -> Self {
            impl_vec_op!(F64x4Vec, lhs: self, rhs: other => { lhs * rhs })
        }
        #[inline]
        pub fn div(&self, other: &Self) -> Self {
            impl_vec_op!(F64x4Vec, lhs: self, rhs: other => { lhs / rhs })
        }
        #[inline]
        pub fn pow(&self, other: &Self) -> Self {
            impl_vec_op!(F64x4Vec, lhs: self, rhs: other => { lhs.pow_f64x4(rhs) })
        }
        #[inline]
        pub fn sin(&self) -> Self {
            impl_vec_op!(F64x4Vec, v: self => { v.sin() })
        }
        #[inline]
        pub fn cos(&self) -> Self {
            impl_vec_op!(F64x4Vec, v: self => { v.cos() })
        }
        #[inline]
        pub fn tan(&self) -> Self {
            impl_vec_op!(F64x4Vec, v: self => { v.tan() })
        }

        pub fn len(&self) -> usize {
            match self {
                F64x4Vec::Vec(v) => v.len(),
                F64x4Vec::Imm(_) => 1,
            }
        }

        pub fn is_empty(&self) -> bool {
            self.len() == 0
        }

        pub fn to_vec(&self, orig_len: usize) -> Vec<f64> {
            let simd_vec_len = self.len();
            assert!(simd_vec_len == orig_len.div_ceil(SIMD_WIDTH));
            match self {
                F64x4Vec::Vec(simd_vec) => {
                    let mut result = Vec::with_capacity(simd_vec.len() * SIMD_WIDTH);

                    for chunk in simd_vec.iter() {
                        result.extend(chunk.to_array());
                    }

                    result.truncate(orig_len);
                    result
                }
                F64x4Vec::Imm(i) => i.to_array().to_vec(),
            }
        }

        // pub fn to_vec(self, len: usize) -> Vec<f64> {
        //     let simd_vec_len = self.len();
        //     assert!(simd_vec_len == len.div_ceil(SIMD_WIDTH));
        //     let simd_vec = match self {
        //         F64x4Vec::Vec(v) => v,
        //         F64x4Vec::Imm(i) => return i.to_array().to_vec(),
        //     };

        //     let mut vec: Vec<_> = simd_vec.iter().flat_map(|v| (*v).to_array()).collect();
        //     vec.remove(vec.len() - len);
        //     vec
        // }
    }

    impl VM<F64x4Vec> {
        pub fn set_vec_size(&mut self, size: usize) {
            self.data = size / SIMD_WIDTH;
        }

        pub fn take_reg(&mut self, indx: usize, size: usize) -> Vec<f64> {
            let res = self.reg[indx].clone();

            self.clear_memory();

            res.to_vec(size)
        }
        pub fn take_stack(&mut self, indx: usize) -> Vec<f64x4> {
            let res = self.stack[indx].clone();

            self.clear_memory();

            match res {
                F64x4Vec::Vec(vec) => {
                    if let Ok(vec) = Rc::try_unwrap(vec.clone()) {
                        vec
                    } else {
                        (*vec).clone()
                    }
                }
                F64x4Vec::Imm(imm) => vec![imm; self.data],
            }
        }
    }

    pub struct F64x4VecInstrTable;

    impl InstrTable<VM<F64x4Vec>> for F64x4VecInstrTable {
        fn add(vm: &mut VM<F64x4Vec>, t: &InstrTape) {
            let (lhs, rhs, out) = vm.binop_arg(t);
            *vm.reg_mut(out) = lhs.add(&rhs);
            vm.next(t);
        }

        fn sub(vm: &mut VM<F64x4Vec>, t: &InstrTape) {
            let (lhs, rhs, out) = vm.binop_arg(t);
            *vm.reg_mut(out) = lhs.sub(&rhs);
            vm.next(t);
        }

        fn mul(vm: &mut VM<F64x4Vec>, t: &InstrTape) {
            let (lhs, rhs, out) = vm.binop_arg(t);
            *vm.reg_mut(out) = lhs.mul(&rhs);
            vm.next(t);
        }

        fn div(vm: &mut VM<F64x4Vec>, t: &InstrTape) {
            let (lhs, rhs, out) = vm.binop_arg(t);
            *vm.reg_mut(out) = lhs.div(&rhs);
            vm.next(t);
        }

        fn pow(vm: &mut VM<F64x4Vec>, t: &InstrTape) {
            let (lhs, rhs, out) = vm.binop_arg(t);
            *vm.reg_mut(out) = lhs.pow(&rhs);
            vm.next(t);
        }

        fn sin(vm: &mut VM<F64x4Vec>, t: &InstrTape) {
            let (lhs, out) = vm.unary_arg(t);
            *vm.reg_mut(out) = lhs.sin();
            vm.next(t);
        }

        fn cos(vm: &mut VM<F64x4Vec>, t: &InstrTape) {
            let (lhs, out) = vm.unary_arg(t);
            *vm.reg_mut(out) = lhs.cos();
            vm.next(t);
        }

        fn tan(vm: &mut VM<F64x4Vec>, t: &InstrTape) {
            let (lhs, out) = vm.unary_arg(t);
            *vm.reg_mut(out) = lhs.tan();
            vm.next(t);
        }

        fn out(vm: &mut VM<F64x4Vec>, t: &InstrTape) {
            let (val, _) = vm.unary_arg(t);
            println!("{val:?}");
            vm.next(t)
        }

        fn mov(vm: &mut VM<F64x4Vec>, t: &InstrTape) {
            let (val, out) = vm.unary_arg(t);
            *vm.reg_mut(out) = val.clone();
            vm.next(t);
        }

        fn psh(vm: &mut VM<F64x4Vec>, t: &InstrTape) {
            let (val, _) = vm.unary_arg(t);
            vm.stack_push(val);
            vm.next(t);
        }

        fn pop(vm: &mut VM<F64x4Vec>, t: &InstrTape) {
            let (_, out) = vm.unary_arg(t);
            *vm.reg_mut(out) = vm.stack_pop();
            vm.next(t);
        }
    }
}

//pub fn run() {
//    let code = [
//        op::ADD_REG_REG(1, 2, 3),
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
            op::ADD_REG_REG(1, 2, 3),
            op::POW_REG_IMM(3, 2.0, 1),
            op::DIV_IMM_REG(100.0, 1, 1),
            op::SUB_REG_IMM(1, 4.0, 1),
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
            op::MUL_REG_REG(6, 1, 1), // sin(z)*cos(x)
            op::MUL_REG_REG(5, 3, 3), // sin(y)*cos(z)
            op::MUL_REG_REG(4, 2, 2), // sin(x)*cos(y)
            op::ADD_REG_REG(2, 1, 1),
            op::ADD_REG_REG(3, 1, 1),
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
        assert!((res - vm_range.stack[1].l).abs() <= f64::EPSILON);
        assert!((res - vm_range.stack[1].u).abs() <= f64::EPSILON);
        assert!((res - vm_f32_vec.take_stack(1)[0]).abs() <= f64::EPSILON);
    }

    #[test]
    fn pow() {
        let pow = [op::POW_REG_REG(1, 2, 1), op::EXT(0)];

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
