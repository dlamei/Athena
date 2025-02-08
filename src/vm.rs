use paste::paste;

pub type Opcode = u64;
pub type Address = usize;

pub type float = f64;
const PI: float = std::f64::consts::PI;
const HALF_PI: float = std::f64::consts::FRAC_PI_2;
const E: float = std::f64::consts::E;

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

const STACK_SIZE: usize = 1024;

const REGISTER_COUNT: usize = 16;

pub trait InstrTable {
    fn nop(vm: &mut Self, t: &InstrTape) {}
    fn add(vm: &mut Self, t: &InstrTape);
    fn sub(vm: &mut Self, t: &InstrTape);
    fn mul(vm: &mut Self, t: &InstrTape);
    fn div(vm: &mut Self, t: &InstrTape);
    fn pow(vm: &mut Self, t: &InstrTape);
    fn sin(vm: &mut Self, t: &InstrTape);
    fn cos(vm: &mut Self, t: &InstrTape);
    fn tan(vm: &mut Self, t: &InstrTape);
    fn out(vm: &mut Self, t: &InstrTape);
    fn mov(vm: &mut Self, t: &InstrTape);
    fn psh(vm: &mut Self, t: &InstrTape);
    fn pop(vm: &mut Self, t: &InstrTape);
    fn ext(vm: &mut Self, t: &InstrTape) {
        log::debug!("exit")
    }

    // fn next_op(vm: &mut Self, t: &InstrTape);

    fn build_table() -> [Instr<Self>; op::NUM_OPS] {
        let mut table: [Instr<Self>; op::NUM_OPS] = [Self::nop; op::NUM_OPS];
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

pub mod machines {
    use super::*;

    pub struct VmF32 {
        pub instr_table: [Instr<Self>; op::NUM_OPS],
        pub reg: [float; REGISTER_COUNT],
        pub stack: [float; STACK_SIZE],
        pub sp: Address,
        pub pc: usize,
    }

    impl VmF32 {

        pub fn eval(&mut self, bin: &[Opcode]) {
            self.pc = 0;
            self.sp = 0;
            let t = InstrTape { bin };
            let instr = op::get_op(t.fetch(self.pc));
            (self.instr_table[instr as usize])(self, &t)
        }

        pub fn new() -> Self {
            Self {
                stack: [float::NAN; STACK_SIZE],
                reg: [float::NAN; REGISTER_COUNT],
                sp: 0,
                instr_table: Self::build_table(),
                pc: 0,
            }
        }

        #[inline(always)]
        fn unary_arg(&mut self, t: &InstrTape) -> (float, usize) {
            let op = t.fetch(self.pc);
            let (_, l, _, out, imm) = op::decode(op);
            let lhs = if l == 0 {
                // float::from_bits(imm)
                op::float_from_imm(imm)
            } else {
                self.reg[l as usize]
            };

            (lhs, out as usize)
        }

        #[inline(always)]
        fn binop_arg(&mut self, t: &InstrTape) -> (float, float, usize) {
            let op = t.fetch(self.pc);
            let (_, l, r, out, imm) = op::decode(op);
            let lhs = if l == 0 {
                // float::from_bits(imm)
                op::float_from_imm(imm)
            } else {
                self.reg[l as usize]
            };

            let rhs = if r == 0 {
                // float::from_bits(imm)
                op::float_from_imm(imm)
            } else {
                self.reg[r as usize]
            };

            (lhs, rhs, out as usize)
        }

        #[inline(always)]
        fn next_op(vm: &mut Self, t: &InstrTape) {
            vm.pc += 1;
            //let instr = op::get_op(t.fetch(self.pc));
            let (op, lhs, rhs, out, imm) = op::decode(t.fetch(vm.pc));
            // let imm = float::from_bits(imm);
            let imm = op::float_from_imm(imm);
            // log::trace!("{}[{lhs}, {rhs}, {imm}] -> reg[{out}]", op::op_to_str(op));
            (vm.instr_table[op as usize])(vm, t)
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
    }

    impl InstrTable for VmF32 {

    fn add(vm: &mut Self, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        vm.reg[out] = lhs + rhs;
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        Self::next_op(vm, t)
    }

    fn sub(vm: &mut Self, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        vm.reg[out] = lhs - rhs;
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        Self::next_op(vm, t)
    }

    fn mul(vm: &mut Self, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        vm.reg[out] = lhs * rhs;
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        Self::next_op(vm, t)
    }

    fn div(vm: &mut Self, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        vm.reg[out] = lhs / rhs;
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        Self::next_op(vm, t)
    }

    fn pow(vm: &mut Self, t: &InstrTape) {
        let (lhs, rhs, out) = vm.binop_arg(t);
        vm.reg[out] = lhs.powf(rhs);
        log::trace!("    {} -> reg[{out}]", vm.reg[out]);
        Self::next_op(vm, t)
    }

    fn sin(vm: &mut Self, t: &InstrTape) {
        let (val, out) = vm.unary_arg(t);
        vm.reg[out] = val.sin();
        Self::next_op(vm, t);
    }

    fn cos(vm: &mut Self, t: &InstrTape) {
        let (val, out) = vm.unary_arg(t);
        vm.reg[out] = val.cos();
        Self::next_op(vm, t);
    }

    fn tan(vm: &mut Self, t: &InstrTape) {
        let (val, out) = vm.unary_arg(t);
        vm.reg[out] = val.tan();
        Self::next_op(vm, t);
    }

    fn out(vm: &mut Self, t: &InstrTape) {
        let (val, _) = vm.unary_arg(t);
        println!("{val}");
        Self::next_op(vm, t);
    }

    fn mov(vm: &mut Self, t: &InstrTape) {
        let (val, out) = vm.unary_arg(t);
        log::trace!("   {val} -> {out}");
        vm.reg[out as usize] = val;
        Self::next_op(vm, t);
    }

    fn psh(vm: &mut Self, t: &InstrTape) {
        let (val, _) = vm.unary_arg(t);
        vm.stack_push(val);
        log::trace!("   {} -> stack[{}]", vm.stack[vm.sp], vm.sp);
        Self::next_op(vm, t);
    }

    fn pop(vm: &mut Self, t: &InstrTape) {
        let (_, out) = vm.unary_arg(t);
        vm.reg[out] = vm.stack_pop();
        Self::next_op(vm, t);
    }

    }

    #[derive(Debug, Clone)]
    pub struct VmRange {
        pub instr_table: [Instr<Self>; op::NUM_OPS],
        pub reg: [Range; REGISTER_COUNT],
        pub stack: [Range; STACK_SIZE],
        pub sp: Address,
        pub pc: usize,
    }

    impl VmRange {

        pub fn new() -> Self {
            Self {
                stack: [Range::UNDEF; STACK_SIZE],
                reg: [Range::UNDEF; REGISTER_COUNT],
                sp: 0,
                instr_table: Self::build_table(),
                pc: 0,
            }
        }

        pub fn eval(&mut self, bin: &[Opcode]) {
            self.pc = 0;
            self.sp = 0;
            let t = InstrTape { bin };
            let instr = op::get_op(t.fetch(self.pc));
            (self.instr_table[instr as usize])(self, &t)
        }

        #[inline(always)]
        fn binop_arg(vm: &mut Self, t: &InstrTape) -> (Range, Range, usize) {
            let op = t.fetch(vm.pc);
            let (_, l, r, out, imm) = op::decode(op);

            let lhs = if l == 0 {
                // let imm = float::from_bits(imm);
                let imm = op::float_from_imm(imm);
                Range::new(imm, imm)
            } else {
                vm.reg[l as usize]
            };

            let rhs = if r == 0 {
                // let imm = float::from_bits(imm);
                let imm = op::float_from_imm(imm);
                Range::new(imm, imm)
            } else {
                vm.reg[r as usize]
            };

            (lhs, rhs, out as usize)
        }

        #[inline(always)]
        fn unary_arg(vm: &mut Self, t: &InstrTape) -> (Range, usize) {
            let op = t.fetch(vm.pc);
            let (_, l, _, out, imm) = op::decode(op);
            let lhs = if l == 0 {
                // let imm = float::from_bits(imm);
                let imm = op::float_from_imm(imm);
                Range::new(imm, imm)
            } else {
                vm.reg[l as usize]
            };

            (lhs, out as usize)
        }


        #[inline(always)]
        fn next_op(vm: &mut Self, t: &InstrTape) {
            vm.pc += 1;
            //let instr = op::get_op(t.fetch(self.pc));
            let (op, lhs, rhs, out, imm) = op::decode(t.fetch(vm.pc));
            // let imm = float::from_bits(imm);
            let imm = op::float_from_imm(imm);
            log::trace!("{}[{lhs}, {rhs}, {imm}] -> reg[{out}]", op::op_to_str(op));
            (vm.instr_table[op as usize])(vm, t)
        }

        pub fn stack_range_push(&mut self, f: Range) {
            self.sp += 1;
            if self.sp < self.stack.len() {
                self.stack[self.sp] = f;
            } else {
                panic!("stack overflow");
            }
        }

        pub fn stack_range_pop(&mut self) -> Range {
            if self.sp < self.stack.len() {
                let f = self.stack[self.sp];
                self.sp -= 1;
                f
            } else {
                panic!("stack overflow");
            }
        }
    }

    impl InstrTable for VmRange {
        fn add(vm: &mut Self, t: &InstrTape) {
            let (a, b, out) = Self::binop_arg(vm, t);
            let c = (a.l + b.l, a.u + b.u).into();
            vm.reg[out] = c;
            log::debug!("add({a}, {b}) = {c}");
            log::trace!("    {} -> reg[{out}]", vm.reg[out]);
            Self::next_op(vm, t)
        }

        fn sub(vm: &mut Self, t: &InstrTape) {
            let (a, b, out) = Self::binop_arg(vm, t);
            let c = (a.l - b.u, a.u - b.l).into();
            vm.reg[out] = c;
            log::debug!("sub({a}, {b}) = {c}");
            log::trace!("    {} -> reg[{out}]", vm.reg[out]);
            Self::next_op(vm, t)
        }

        fn mul(vm: &mut Self, t: &InstrTape) {
            let (a, b, out) = Self::binop_arg(vm, t);
            let c = Range::from_tuple(min_max_4(a.l * b.l, a.l * b.u, a.u * b.l, a.u * b.u));
            vm.reg[out] = c;
            log::debug!("mul({a}, {b}) = {c}");
            log::trace!("    {} -> reg[{out}]", vm.reg[out]);
            Self::next_op(vm, t)
        }

        fn div(vm: &mut Self, t: &InstrTape) {
            let (a, b, out) = Self::binop_arg(vm, t);
            // let c = if b.contains_zero() {
            //     Range::UNDEF
            // } else {
            //     Range::from_tuple(min_max_4(a.l / b.l, a.l / b.u, a.u / b.l, a.u / b.u))
            // };
            let c = Range::from_tuple(min_max_4(a.l / b.l, a.l / b.u, a.u / b.l, a.u / b.u));

            vm.reg[out] = c;
            log::debug!("div({a}, {b}) = {c}");
            log::trace!("    {} -> reg[{out}]", vm.reg[out]);
            Self::next_op(vm, t)
        }

        fn pow(vm: &mut Self, t: &InstrTape) {
            let (a, b, out) = Self::binop_arg(vm, t);

            if !(a.is_finite() || b.is_finite()) {}

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
                            // return Self::ext(vm, t);
                    }
                }
            } else if a.u < 0.0 {
                // a < 0
                if b.is_const_int() {
                    let b = b.l;

                    Range::from_tuple(min_max_2(a.u.powf(b), a.l.powf(b)))
                } else {
                    Range::UNDEF
                        // return Self::ext(vm, t)
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
                        // return Self::ext(vm, t)
                }
            };

            log::debug!("pow({a}, {b}) = {c}");
            vm.reg[out] = c;
            log::trace!("    {} -> reg[{out}]", vm.reg[out]);
            Self::next_op(vm, t)

                // let c = if a.l >= 0.0 || b.l.floor() == b.l && b.u.floor() == b.u {
                //     Range::from_tuple(min_max_4(
                //         a.l.powf(b.l),
                //         a.l.powf(b.u),
                //         a.u.powf(b.l),
                //         a.u.powf(b.u),
                //     ))
                // } else {
                //     Range::UNDEF
                // };
        }

        fn sin(vm: &mut Self, t: &InstrTape) {
            let (a, out) = Self::unary_arg(vm, t);

            if a.is_inf() {
                vm.reg[out] = (-1.0, 1.0).into();
                Self::next_op(vm, t);
                return;
            }

            let k = (((a.l - HALF_PI) / PI).ceil() * PI + HALF_PI).min(a.u);
            let l = (k + PI).min(a.u);
            let b = min_max_4(k.sin(), l.sin(), a.l.sin(), a.u.sin()).into();

            vm.reg[out] = b;
            log::trace!("    {} -> reg[{out}]", vm.reg[out]);
            Self::next_op(vm, t);
        }

        fn cos(vm: &mut Self, t: &InstrTape) {
            let (a, out) = Self::unary_arg(vm, t);

            if a.is_inf() {
                vm.reg[out] = (-1.0, 1.0).into();
                Self::next_op(vm, t);
                return;
            }

            let k = ((a.l / PI).ceil() * PI).min(a.u);
            let l = (k + PI).min(a.u);
            let b = min_max_4(k.cos(), l.cos(), a.l.cos(), a.u.cos()).into();

            vm.reg[out] = b;
            log::trace!("    {} -> reg[{out}]", vm.reg[out]);
            Self::next_op(vm, t);
        }

        fn tan(vm: &mut Self, t: &InstrTape) {
            todo!()
        }

        fn out(vm: &mut Self, t: &InstrTape) {
            let (val, _) = Self::unary_arg(vm, t);
            println!("{val}");
            Self::next_op(vm, t);
        }

        fn mov(vm: &mut Self, t: &InstrTape) {
            let (a, out) = Self::unary_arg(vm, t);
            log::trace!("   {a} -> {out}");
            vm.reg[out as usize] = a;
            Self::next_op(vm, t);
        }

        fn psh(vm: &mut Self, t: &InstrTape) {
            let (val, _) = Self::unary_arg(vm, t);
            vm.stack_range_push(val);
            log::trace!("   {} -> stack[{}]", vm.stack[vm.sp], vm.sp);
            Self::next_op(vm, t);
        }

        fn pop(vm: &mut Self, t: &InstrTape) {
            let (_, out) = Self::unary_arg(vm, t);
            vm.reg[out] = vm.stack_range_pop();
            Self::next_op(vm, t);
        }
    }

}

/*
#[derive(Debug, Clone, Copy)]
pub struct RangeEvalInstrTable;

impl RangeEvalInstrTable {
    #[inline(always)]
    fn binop_arg(vm: &mut VM, t: &InstrTape) -> (Range, Range, usize) {
        let op = t.fetch(vm.pc);
        let (_, l, r, out, imm) = op::decode(op);

        let lhs = if l == 0 {
            // let imm = float::from_bits(imm);
            let imm = op::float_from_imm(imm);
            Range::new(imm, imm)
        } else {
            vm.registers_range[l as usize]
        };

        let rhs = if r == 0 {
            // let imm = float::from_bits(imm);
            let imm = op::float_from_imm(imm);
            Range::new(imm, imm)
        } else {
            vm.registers_range[r as usize]
        };

        (lhs, rhs, out as usize)
    }

    #[inline(always)]
    fn unary_arg(vm: &mut VM, t: &InstrTape) -> (Range, usize) {
        let op = t.fetch(vm.pc);
        let (_, l, _, out, imm) = op::decode(op);
        let lhs = if l == 0 {
            // let imm = float::from_bits(imm);
            let imm = op::float_from_imm(imm);
            Range::new(imm, imm)
        } else {
            vm.registers_range[l as usize]
        };

        (lhs, out as usize)
    }
}

impl InstrTable for RangeEvalInstrTable {
    fn add(vm: &mut VM, t: &InstrTape) {
        let (a, b, out) = Self::binop_arg(vm, t);
        let c = (a.l + b.l, a.u + b.u).into();
        vm.registers_range[out] = c;
        log::debug!("add({a}, {b}) = {c}");
        log::trace!("    {} -> reg[{out}]", vm.registers_range[out]);
        Self::next_op(vm, t)
    }

    fn sub(vm: &mut VM, t: &InstrTape) {
        let (a, b, out) = Self::binop_arg(vm, t);
        let c = (a.l - b.u, a.u - b.l).into();
        vm.registers_range[out] = c;
        log::debug!("sub({a}, {b}) = {c}");
        log::trace!("    {} -> reg[{out}]", vm.registers_range[out]);
        Self::next_op(vm, t)
    }

    fn mul(vm: &mut VM, t: &InstrTape) {
        let (a, b, out) = Self::binop_arg(vm, t);
        let c = Range::from_tuple(min_max_4(a.l * b.l, a.l * b.u, a.u * b.l, a.u * b.u));
        vm.registers_range[out] = c;
        log::debug!("mul({a}, {b}) = {c}");
        log::trace!("    {} -> reg[{out}]", vm.registers_range[out]);
        Self::next_op(vm, t)
    }

    fn div(vm: &mut VM, t: &InstrTape) {
        let (a, b, out) = Self::binop_arg(vm, t);
        // let c = if b.contains_zero() {
        //     Range::UNDEF
        // } else {
        //     Range::from_tuple(min_max_4(a.l / b.l, a.l / b.u, a.u / b.l, a.u / b.u))
        // };
        let c = Range::from_tuple(min_max_4(a.l / b.l, a.l / b.u, a.u / b.l, a.u / b.u));

        vm.registers_range[out] = c;
        log::debug!("div({a}, {b}) = {c}");
        log::trace!("    {} -> reg[{out}]", vm.registers_range[out]);
        Self::next_op(vm, t)
    }

    fn pow(vm: &mut VM, t: &InstrTape) {
        let (a, b, out) = Self::binop_arg(vm, t);

        if !(a.is_finite() || b.is_finite()) {}

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
                    // return Self::ext(vm, t);
                }
            }
        } else if a.u < 0.0 {
            // a < 0
            if b.is_const_int() {
                let b = b.l;

                Range::from_tuple(min_max_2(a.u.powf(b), a.l.powf(b)))
            } else {
                Range::UNDEF
                // return Self::ext(vm, t)
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
                // return Self::ext(vm, t)
            }
        };

        log::debug!("pow({a}, {b}) = {c}");
        vm.registers_range[out] = c;
        log::trace!("    {} -> reg[{out}]", vm.registers_range[out]);
        Self::next_op(vm, t)

        // let c = if a.l >= 0.0 || b.l.floor() == b.l && b.u.floor() == b.u {
        //     Range::from_tuple(min_max_4(
        //         a.l.powf(b.l),
        //         a.l.powf(b.u),
        //         a.u.powf(b.l),
        //         a.u.powf(b.u),
        //     ))
        // } else {
        //     Range::UNDEF
        // };
    }

    fn sin(vm: &mut VM, t: &InstrTape) {
        let (a, out) = Self::unary_arg(vm, t);

        if a.is_inf() {
            vm.registers_range[out] = (-1.0, 1.0).into();
            Self::next_op(vm, t);
            return;
        }

        let k = (((a.l - HALF_PI) / PI).ceil() * PI + HALF_PI).min(a.u);
        let l = (k + PI).min(a.u);
        let b = min_max_4(k.sin(), l.sin(), a.l.sin(), a.u.sin()).into();

        vm.registers_range[out] = b;
        log::trace!("    {} -> reg[{out}]", vm.registers_range[out]);
        Self::next_op(vm, t);
    }

    fn cos(vm: &mut VM, t: &InstrTape) {
        let (a, out) = Self::unary_arg(vm, t);

        if a.is_inf() {
            vm.registers_range[out] = (-1.0, 1.0).into();
            Self::next_op(vm, t);
            return;
        }

        let k = ((a.l / PI).ceil() * PI).min(a.u);
        let l = (k + PI).min(a.u);
        let b = min_max_4(k.cos(), l.cos(), a.l.cos(), a.u.cos()).into();

        vm.registers_range[out] = b;
        log::trace!("    {} -> reg[{out}]", vm.registers_range[out]);
        Self::next_op(vm, t);
    }

    fn tan(vm: &mut VM, t: &InstrTape) {
        todo!()
    }

    fn out(vm: &mut VM, t: &InstrTape) {
        let (val, _) = Self::unary_arg(vm, t);
        println!("{val}");
        Self::next_op(vm, t);
    }

    fn mov(vm: &mut VM, t: &InstrTape) {
        let (a, out) = Self::unary_arg(vm, t);
        log::trace!("   {a} -> {out}");
        vm.registers_range[out as usize] = a;
        Self::next_op(vm, t);
    }

    fn psh(vm: &mut VM, t: &InstrTape) {
        let (val, _) = Self::unary_arg(vm, t);
        vm.stack_range_push(val);
        log::trace!("   {} -> stack[{}]", vm.stack_range[vm.sp], vm.sp);
        Self::next_op(vm, t);
    }

    fn pop(vm: &mut VM, t: &InstrTape) {
        let (_, out) = Self::unary_arg(vm, t);
        vm.registers_range[out] = vm.stack_range_pop();
        Self::next_op(vm, t);
    }

    fn next_op(vm: &mut VM, t: &InstrTape) {
        vm.pc += 1;
        //let instr = op::get_op(t.fetch(self.pc));
        let (op, lhs, rhs, out, imm) = op::decode(t.fetch(vm.pc));
        // let imm = float::from_bits(imm);
        let imm = op::float_from_imm(imm);
        log::trace!("{}[{lhs}, {rhs}, {imm}] -> reg[{out}]", op::op_to_str(op));
        (vm.instr_table_range[op as usize])(vm, t)
    }
}
*/

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
    // not a single point is defined
    pub const UNDEF: Self = Self {
        l: float::NAN,
        u: float::NAN,
    };
    // entire possible range
    pub const F32: Self = Self {
        l: float::NEG_INFINITY,
        u: float::INFINITY,
    };
    // no single continous range
    // pub const NON_CONTINUOUS: Self = Self {
    //     l: float::INFINITY,
    //     u: float::NEG_INFINITY,
    // };

    pub fn new(l: float, h: float) -> Self {
        debug_assert!(l <= h || (l.is_nan() && h.is_nan()), "{l} <= {h}");
        Range { l, u: h }
    }

    pub fn from_tuple(b: (float, float)) -> Self {
        Self::new(b.0, b.1)
    }

    pub fn is_inf(&self) -> bool {
        //self == &Self::F32
        self.l.is_infinite() || self.u.is_infinite()
    }

    pub const fn is_finite(&self) -> bool {
        self.l.is_finite() && self.u.is_finite()
    }

    pub const fn is_const(&self) -> bool {
        (self.l - self.u) < float::EPSILON
    }

    pub fn is_const_int(&self) -> bool {
        self.is_const() && self.l.fract() == 0.0
    }

    pub const fn is_pos(&self) -> bool {
        self.l > 0.0 && self.u > 0.0
    }

    pub const fn is_neg(&self) -> bool {
        self.l < 0.0 && self.u < 0.0
    }

    // pub fn is_non_continuous(&self) -> bool {
    //     self == &Self::NON_CONTINUOUS
    // }

    pub fn is_undef(&self) -> bool {
        self == &Self::UNDEF
    }

    pub const fn contains_zero(&self) -> bool {
        self.l <= 0.0 && self.u >= 0.0
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

        let mut vm_f32 = machines::VmF32::new();
        vm_f32.eval(&code);

        let mut vm_range = machines::VmRange::new();
        vm_range.eval(&code);

        assert_eq!(vm_f32.stack[1], vm_range.stack[1].l);
        assert_eq!(vm_f32.stack[1], vm_range.stack[1].u);
    }

    #[test]
    fn pow() {
        let pow = [op::POW_LHS_RHS(1, 2, 1), op::EXT(0)];

        let mut vm = machines::VmRange::new();

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
