use std::collections::HashMap;
use std::marker::PhantomData;

use atl_macro::jit_fn;
use cranelift_codegen::ir::AbiParam;
use cranelift_codegen::settings::Configurable;
use cranelift_codegen::{
    ir::{
        self, InstBuilder,
        condcodes::{self, IntCC as IntCondCode},
    },
    isa,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable as JITVar};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};
use paste::paste;
use rustc_hash::FxHashMap;

use wide::{self, f64x2, f64x4};

use crate::vm::ExplicitCopy;

pub type Reg = u8;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Oprnd {
    Reg(Reg),
    Imm(f64),
}

impl Oprnd {
    pub fn reg(reg: impl Into<Reg>) -> Self {
        Self::Reg(reg.into())
    }
    pub fn imm(imm: impl Into<f64>) -> Self {
        Self::Imm(imm.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum BinOp {
    ADD,
    SUB,
    MUL,
    DIV,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum UnOp {
    MOV,

    SIN,
    COS,
    TAN,
}

impl UnOp {
    fn c_fn_name(&self) -> Option<&'static str> {
        match self {
            UnOp::SIN => "sin",
            UnOp::COS => "cos",
            UnOp::TAN => "tan",
            _ => return None,
        }
        .into()
    }
}

macro_rules! bytecode {
    (@oprnd: reg($val:literal)) => { crate::jit::Oprnd::Reg($val.into()) };
    (@oprnd: imm($val:literal)) => { crate::jit::Oprnd::Imm($val.into()) };
    (@oprnd: $reg:literal) => { crate::jit::Oprnd::Reg($reg.into()) };

    (@instr: $op: ident [$($loprnd_typ: ident)? $(($lval:literal))? $($lreg:literal)?, $($roprnd_typ: ident)? $(($rval:literal))? $($rreg:literal)? ] -> $dst:literal) => {
        crate::jit::Instr::BinOp {
            op: crate::jit::BinOp::$op,
            lhs: crate::jit::bytecode!(@oprnd: $($loprnd_typ)? $(($lval))? $($lreg)?),
            rhs: crate::jit::bytecode!(@oprnd: $($roprnd_typ)? $(($rval))? $($rreg)?),
            dst: $dst,
        }
    };

    (@instr: $op: ident [$($oprnd_typ: ident)? $(($val:literal))? $($reg:literal)?] -> $dst:literal) => {
        crate::jit::Instr::UnOp {
            op: crate::jit::UnOp::$op,
            val: crate::jit::bytecode!(@oprnd: $($oprnd_typ)? $(($val))? $($reg)?),
            dst: $dst,
        }
    };

    ($op:ident [$($oprnds:tt)*] -> $dst:literal $(,)?) => {
        crate::jit::bytecode!(@instr: $op[$($oprnds)*] -> $dst)
    };


    ($($op:ident [$($oprnds:tt)*] -> $dst:literal $(,)?)+) => {
        [ $( crate::jit::bytecode!(@instr: $op[$($oprnds)*] -> $dst) , )+]
        // crate::jit::bytecode!($($rest)+)
    };
}
pub(crate) use bytecode;

macro_rules! oprnd_val {
    ($oprnd:expr, $reg_vars: expr, $fn_ctx: expr) => {
        match $oprnd {
            Oprnd::Reg(reg) => $fn_ctx.use_var($reg_vars[reg.copy() as usize]),
            Oprnd::Imm(imm) => $fn_ctx.ins().f64const(imm.copy()),
        }
    };
}

macro_rules! set_reg {
    ($val:expr, $reg: expr, $reg_vars: expr, $fn_ctx: expr) => {
        let reg_var = $reg_vars[$reg.copy() as usize];
        $fn_ctx.def_var(reg_var, $val.copy())
    };
}

macro_rules! make_sig {

    (@param: # $p:expr) => {
        $p
    };
    (@param: $p:expr) => {
        paste!(cranelift_codegen::ir::types::$p)
    };

    ($mod:expr, ($($p_typ:ident)? $(#$p_var:expr)? $(,$($pr_typ:ident)? $(#$pr_var:expr)?)* $(,)?) -> ($($ret_typ:ident)? $(#$ret_var:expr)?)) => {{
        #[allow(unused_mut)]
        let mut sig = $mod.make_signature();
        $(sig.params.push(ir::AbiParam::new(make_sig!(@param: #$p_var)));)?
        $(sig.params.push(ir::AbiParam::new(make_sig!(@param: $p_typ)));)?
        $($(sig.params.push(ir::AbiParam::new(make_sig!(@param: $pr_typ)));)?)*
        $($(sig.params.push(ir::AbiParam::new(make_sig!(@param: #$pr_var)));)?)*
        $(sig.returns.push(ir::AbiParam::new(make_sig!(@param: #$ret_var)));)?
        $(sig.returns.push(ir::AbiParam::new(make_sig!(@param: $ret_typ)));)?
        sig
    }};
}

macro_rules! extrn_sig {

    (@param: # $p:expr) => {
        $p
    };
    (@param: $p:expr) => {
        paste!(cranelift_codegen::ir::types::$p)
    };

    (($($p_typ:ident)? $(#$p_var:expr)? $(,$($pr_typ:ident)? $(#$pr_var:expr)?)* $(,)?) -> 
     ($($ret_typ:ident)? $(#$ret_var:expr)? $(,$($retr_typ:ident)? $(#$retr_var:expr)?)* $(,)?)) => {{
     //($($ret_typ:ident)? $(#$ret_var:expr)?)) => 
        #[allow(unused_mut)]
        let mut params = vec![];
        #[allow(unused_mut)]
        let mut returns = vec![];

        $(params.push(ir::AbiParam::new(make_sig!(@param: #$p_var)));)?
        $(params.push(ir::AbiParam::new(make_sig!(@param: $p_typ)));)?
        $($(params.push(ir::AbiParam::new(make_sig!(@param: $pr_typ)));)?)*
        $($(params.push(ir::AbiParam::new(make_sig!(@param: #$pr_var)));)?)*

        $(returns.push(ir::AbiParam::new(make_sig!(@param: #$ret_var)));)?
        $(returns.push(ir::AbiParam::new(make_sig!(@param: $ret_typ)));)?
        $($(returns.push(ir::AbiParam::new(make_sig!(@param: $retr_typ)));)?)*
        $($(returns.push(ir::AbiParam::new(make_sig!(@param: #$retr_var)));)?)*

        let mut sig = ir::Signature::new(isa::CallConv::triple_default(&target_lexicon::Triple::host()));
        sig.params = params;
        sig.returns = returns;
        sig
    }};
}


macro_rules! decl_var {
    ($fb:expr, $idx:tt: $typ:ident) => {{
        paste! {
        let var = cranelift_frontend::Variable::from_u32($idx);
        let typ = cranelift_codegen::ir::types::[<$typ:upper>];
        let val = $fb.ins().[<$typ:lower const>]($val);
        $fb.declare_var(var, typ);
        }
    }};

    ($fb:expr, $idx:tt: $typ:ident [$const_ty:ident]) => {{
        paste! {
        let var = cranelift_frontend::Variable::from_u32($idx);
        let typ = cranelift_codegen::ir::types::[<$typ:upper>];
        let val = $fb.ins().[<$const_ty:lower const>]($val);
        $fb.declare_var(var, typ);
        }
    }};
}

macro_rules! init_var {
    ($fb:expr, $idx:tt: $typ:ident [$const_ty:ident] = $val:literal) => {{
        paste! {
        let var = cranelift_frontend::Variable::from_u32($idx);
        let typ = cranelift_codegen::ir::types::[<$typ:upper>];
        let val = $fb.ins().[<$const_ty:lower const>](typ, $val);
        $fb.declare_var(var, typ);
        $fb.def_var(var, val);
        var
        }
    }};
    ($fb:expr, $idx:tt: $typ:ident = $val:literal) => {{
        paste! {
        let var = cranelift_frontend::Variable::from_u32($idx);
        let typ = cranelift_codegen::ir::types::[<$typ:upper>];
        let val = $fb.ins().[<$typ:lower const>]($val);
        $fb.declare_var(var, typ);
        $fb.def_var(var, val);
        var
        }
    }};
}

macro_rules! rep_count {
    (@ident: ) => ( 0 );
    (@ident: $x:ident $($xs:ident)*) => ( 1 + rep_count!(@ident: $($xs)*) );
}

macro_rules! extern_c_fns {
    //($($c_fn:ident $(,)?)*) => {
    ($($c_fn:ident),* $(,)?) => {
        struct ExternCFnTable {
            $($c_fn: cranelift_module::FuncId,)*
        }

        impl ExternCFnTable {

            fn import<F>(mut import_fn: F) -> Self
            where F: FnMut(&'static str) -> cranelift_module::FuncId
            {
                Self {
                    $($c_fn: import_fn(stringify!($c_fn)),)*
                }
            }

        }
    }
}

extern_c_fns!(sin, cos, tan);

pub trait AsJITType {
    const TYPE: ir::Type;
}
impl AsJITType for i32 {
    const TYPE: ir::Type = ir::types::I32;
}
impl AsJITType for i64 {
    const TYPE: ir::Type = ir::types::I64;
}
impl AsJITType for f32 {
    const TYPE: ir::Type = ir::types::F32;
}
impl AsJITType for f64 {
    const TYPE: ir::Type = ir::types::F64;
}

pub trait IRSignature {
    fn ir_signature(module: &JITModule) -> ir::Signature;
}

#[jit_fn]
fn print_f64(f: f64) {
    println!("{f}");
}

#[inline]
fn pack_f64x2(l: f64x2, u: f64x2) -> f64x4 {
    let [ll, lu] = l.to_array();
    let [ul, uu] = u.to_array();
    f64x4::new([ll, lu, ul, uu])
}

#[inline]
fn unpack_f64x4(val: f64x4) -> (f64x2, f64x2) {
    let [ll, lu, ul, uu] = val.to_array();
    (f64x2::new([ll, lu]), f64x2::new([ul, uu]))
}

fn sin_f64x2x4(l: f64x2, u: f64x2) -> (f64x2, f64x2) {
    let val = pack_f64x2(l, u);
    unpack_f64x4(val.sin())
}

fn div_f64x2x2(lhsl: f64x2, lhsu: f64x2, rhsl: f64x2, rhsu: f64x2) -> (f64x2, f64x2) {
    let lhs = pack_f64x2(lhsl, lhsu);
    let rhs = pack_f64x2(rhsl, rhsu);
    unpack_f64x4(lhs / rhs)
}

impl ExternCFnTable {
    fn unop_id(&self, unop: UnOp) -> FuncId {
        match unop {
            UnOp::SIN => self.sin,
            UnOp::COS => self.cos,
            UnOp::TAN => self.tan,
            UnOp::MOV => panic!("not a c function: {unop:?}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Instr {
    UnOp {
        op: UnOp,
        val: Oprnd,
        dst: Reg,
    },
    BinOp {
        op: BinOp,
        lhs: Oprnd,
        rhs: Oprnd,
        dst: Reg,
    },
}

#[unsafe(no_mangle)]
extern "C" fn print_hello() {
    println!("Hello World");
}

struct FnTable(HashMap<String, FuncId>);

impl FnTable {
    fn get(&self, name: &str) -> FuncId {
        self.0[name]
    }

    fn get_unop(&self, unop: UnOp) -> FuncId {
        let name = match unop {
            UnOp::SIN => "sin",
            UnOp::COS => "cos",
            UnOp::TAN => "tan",
            UnOp::MOV => panic!("not a function: {unop:?}"),
        };
        self.get(name)
    }

    fn insert(&mut self, name: &str, id: FuncId) {
        self.0.insert(name.to_string(), id);
    }

    fn decl_in_func(
        &self,
        module: &mut JITModule,
        func: &mut ir::Function,
    ) -> HashMap<String, ir::FuncRef> {
        self.0
            .iter()
            .map(|(name, id)| (name.to_owned(), module.declare_func_in_func(*id, func)))
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExternFnPtr {
    Rust(*const u8),
    C(&'static str),
}


type ExternFnSig = ir::Signature;

#[derive(Debug, Clone, PartialEq)]
pub struct ExternFn {
    ptr: ExternFnPtr,
    sig: ExternFnSig,
    name: String,
}

impl ExternFn {
    fn c_fn(name: &'static str, sig: &ExternFnSig) -> Self {
        Self {
            ptr: ExternFnPtr::C(name),
            sig: sig.clone(),
            name: name.into(),
        }
    }

    fn rust(name: &str, ptr: *const u8, sig: &ExternFnSig) -> Self {
        Self {
            ptr: ExternFnPtr::Rust(ptr),
            sig: sig.clone(),
            name: name.into(),
        }
    }
}

pub struct JITCompiler {
    module: JITModule,
    ctx: cranelift_codegen::Context,
    fn_ctx: FunctionBuilderContext,
    // c_fn_table: HashMap<,
    glob_fn_table: FnTable,
}

fn simd_bit_width() -> usize {
    if is_x86_feature_detected!("avx512f") {
        512
    } else if is_x86_feature_detected!("avx") {
        256
    } else if is_x86_feature_detected!("sse2") {
        128
    } else {
        0
    }
}

impl JITCompiler {
    // fn unop_sig(module: &mut JITModule) -> ir::Signature {
    //     make_sig!(module: (F64) -> F64)
    //     // let mut sig = module.make_signature();
    //     // sig.params.push(ir::AbiParam::new(ir::types::F64));
    //     // sig.returns.push(ir::AbiParam::new(ir::types::F64));
    //     // sig
    // }

    // fn binop_sig(module: &mut JITModule) -> ir::Signature {
    //     make_sig!(module: (F64, F64) -> F64)
    //     // let mut sig = module.make_signature();
    //     // sig.params.push(ir::AbiParam::new(ir::types::F64));
    //     // sig.params.push(ir::AbiParam::new(ir::types::F64));
    //     // sig.returns.push(ir::AbiParam::new(ir::types::F64));
    //     // sig
    // }

    pub fn init() -> Self {
        // let isa = {
        //     use cranelift_codegen::settings;
        //     let mut flag_builder = settings::builder();

        //     // On at least AArch64, "colocated" calls use shorter-range relocations,
        //     // which might not reach all definitions; we can't handle that here, so
        //     // we require long-range relocation types.
        //     flag_builder.set("use_colocated_libcalls", "false").unwrap();
        //     flag_builder.set("is_pic", "false").unwrap();
        //     let mut isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
        //         panic!("host machine is not supported: {msg}");
        //     });
        //     isa_builder.enable("has_sse42").unwrap();
        //     isa_builder.enable("has_avx").unwrap();
        //     isa_builder.enable("has_avx512f").unwrap();
        //     isa_builder.enable("has_avx512dq").unwrap();
        //     isa_builder.enable("has_avx512vl").unwrap();
        //     let isa = isa_builder.finish(settings::Flags::new(flag_builder)).unwrap();
        //     isa
        // };

        // let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());

        let unop_sig = extrn_sig!((F64) -> (F64));
        let imports = [
            ExternFn::c_fn("sin", &unop_sig),
            ExternFn::c_fn("cos", &unop_sig),
            ExternFn::c_fn("tan", &unop_sig),

            ExternFn::rust("print_f64", print_f64 as *const u8, &extrn_sig!((F64) -> ())),
        ];
            //ExternFn::rust("sin_f64x2x4", sin_f64x2x4 as *const u8, &extrn_sig!((F64X2, F64X2) -> (F64X2, F64X2))),

        let mut builder = JITBuilder::with_flags(
            &[
                // ("opt_level", "speed_and_size")
                // ("has_avx512f", "true"),
                #[cfg(debug_assertions)]
                ("enable_verifier", "true"),
            ],
            cranelift_module::default_libcall_names(),
        )
        .unwrap();
        //builder.symbol("print_f64", print_f64 as *const u8);

        builder.symbol("sin_f64x2x4", sin_f64x2x4 as *const u8);
        builder.symbol("div_f64x2x2", div_f64x2x2 as *const u8);

        imports.iter().for_each(|ex_fn| {
            if let ExternFnPtr::Rust(ptr) = ex_fn.ptr {
                builder.symbol(&ex_fn.name, ptr);
            }
        });

        let mut module = JITModule::new(builder);
        let ctx = module.make_context();
        let fn_ctx = FunctionBuilderContext::new();

        let mut glob_fn_table = FnTable(HashMap::default());


        imports.into_iter().for_each(|mut ex_fn| {
            ex_fn.sig.call_conv = module.isa().default_call_conv();

            glob_fn_table.insert(
                &ex_fn.name,
                module
                    .declare_function(&ex_fn.name, Linkage::Import, &ex_fn.sig)
                    .unwrap(),
            );
            
        });

        {
            let name = "sin_f64x2x4";
            let mut sig = module.make_signature();

            let ptr_ty = module.target_config().pointer_type();
            let mut sret_param = AbiParam::new(ptr_ty);
            sret_param.purpose = ir::ArgumentPurpose::StructReturn;
            sig.params.push(sret_param);

            sig.params.push(AbiParam::new(ir::types::F64X2));
            sig.params.push(AbiParam::new(ir::types::F64X2));
            let id = module.declare_function(name, Linkage::Import, &sig)
                .unwrap();
            glob_fn_table.insert(name, id);
        }
        {
            let name = "div_f64x2x2";
            let mut sig = module.make_signature();

            let ptr_ty = module.target_config().pointer_type();
            let mut sret_param = AbiParam::new(ptr_ty);
            sret_param.purpose = ir::ArgumentPurpose::StructReturn;
            sig.params.push(sret_param);

            sig.params.push(AbiParam::new(ir::types::F64X2));
            sig.params.push(AbiParam::new(ir::types::F64X2));
            sig.params.push(AbiParam::new(ir::types::F64X2));
            sig.params.push(AbiParam::new(ir::types::F64X2));
            let id = module.declare_function(name, Linkage::Import, &sig)
                .unwrap();
            glob_fn_table.insert(name, id);
        }


        // let unop_sig = make_sig!(module, (F64) -> (F64));

        // ["sin", "cos", "tan"].into_iter().for_each(|name| {
        //     let id = module
        //         .declare_function(name, Linkage::Import, &unop_sig)
        //         .expect("import: {name}");
        //     glob_fn_table.insert(name, id);
        // });
        // let c_fn_table = ExternCFnTable::import(|name| {
        //     module.declare_function(name, Linkage::Import, &unop_sig)
        //         .expect("import: {name}")
        // });



        // let name = "print_f64";
        // glob_fn_table.insert(
        //     name,
        //     module
        //         .declare_function(name, Linkage::Import, &sig)
        //         .unwrap(),
        // );

        Self {
            module,
            ctx,
            fn_ctx,
            glob_fn_table,
        }
    }

    pub fn compile_for_f64(
        &mut self,
        fn_name: &str,
        bytecode: &[Instr],
    ) -> extern "C" fn(f64, f64) -> f64 {
        self.ctx.func.signature = make_sig!(self.module, (F64, F64) -> (F64));

        let mut fb = FunctionBuilder::new(&mut self.ctx.func, &mut self.fn_ctx);
        let entry = fb.create_block();
        fb.append_block_params_for_function_params(entry);
        fb.switch_to_block(entry);
        fb.seal_block(entry);

        let loc_fns = self
            .glob_fn_table
            .decl_in_func(&mut self.module, &mut fb.func);

        let mut regs = vec![];
        for i in 0..16 {
            let v = init_var!(fb, i: F64 = 0.0);
            regs.push(v);
        }

        let param = fb.block_params(entry)[0];
        fb.def_var(regs[0], param);
        let param = fb.block_params(entry)[1];
        fb.def_var(regs[1], param);

        for instr in bytecode {
            match *instr {
                Instr::UnOp { op, val, dst } => {
                    let val = oprnd_val!(val, regs, fb);
                    match op {
                        UnOp::MOV => {
                            fb.def_var(regs[dst as usize], val);
                            // continue;
                        }
                        c_op => {
                            let name = c_op.c_fn_name().unwrap();
                            let fn_ref = loc_fns[name];
                            let call = fb.ins().call(fn_ref, &[val]);
                            let res = fb.inst_results(call)[0];
                            fb.def_var(regs[dst as usize], res);
                        } // UnOp::SIN => "sin",
                          // UnOp::COS => "cos",
                          // UnOp::TAN => "tan",
                    };
                    // let fn_id = self.glob_fn_table.get_unop(op);
                }
                Instr::BinOp { op, lhs, rhs, dst } => {
                    let lhs = oprnd_val!(lhs, regs, fb);
                    let rhs = oprnd_val!(rhs, regs, fb);
                    let res = match op {
                        BinOp::ADD => fb.ins().fadd(lhs, rhs),
                        BinOp::SUB => fb.ins().fsub(lhs, rhs),
                        BinOp::MUL => fb.ins().fmul(lhs, rhs),
                        BinOp::DIV => fb.ins().fdiv(lhs, rhs),
                    };
                    fb.def_var(regs[dst as usize], res);
                }
            }
        }

        let ret = fb.use_var(regs[0]);

        fb.ins().return_(&[ret]);
        fb.finalize();

        let fn_id = self
            .module
            .declare_function(fn_name, Linkage::Local, &self.ctx.func.signature)
            .unwrap();
        self.module.define_function(fn_id, &mut self.ctx).unwrap();
        self.module.finalize_definitions().unwrap();
        self.module.clear_context(&mut self.ctx);

        let fn_ptr = self.module.get_finalized_function(fn_id);

        unsafe { std::mem::transmute(fn_ptr) }
    }

    // TODO: config kernel, slice, name, flags, etc...

    fn supported_f64_simd_typ(&self) -> ir::Type {
        let n_lanes = self.module.isa().dynamic_vector_bytes(ir::types::F64) / 8;
        match n_lanes {
            1 => ir::types::F64,
            2 => ir::types::F64X2,
            4 => ir::types::F64X4,
            8 => ir::types::F64X8,
            _ => panic!(),
        }
    }

    pub fn compile_for_f64x2xn(
        &mut self,
        name: &str,
        bytecode: &[Instr],
    ) -> extern "C" fn(*const f64, *const f64, *mut f64, i64) {
        let vec_ty = ir::types::F64X2;
        let i64_ty = ir::types::I64;
        let f64_ty = ir::types::F64;

        let mut variable_id = 0;
        let mut new_var = || {
            variable_id += 1;
            JITVar::from_u32(variable_id)
        };

        // two f64x2 at once (f64x4 not well supported currently)
        let step_size = 2 * vec_ty.lane_count();
        let ptr_ty = self.module.target_config().pointer_type();

        let sig = make_sig!(self.module, (#ptr_ty, #ptr_ty, #ptr_ty, I64) -> ());
        self.ctx.func.signature = sig.clone();

        let mut fb = FunctionBuilder::new(&mut self.ctx.func, &mut self.fn_ctx);
        let loc_fns = self
            .glob_fn_table
            .decl_in_func(&mut self.module, &mut fb.func);

        // entry(void *a, void *b, void *out, i64 len);
        let entry = fb.create_block();
        fb.append_block_params_for_function_params(entry);
        fb.switch_to_block(entry);
        fb.seal_block(entry);

        // void *a;
        let a_base_ptr = fb.block_params(entry)[0];
        // void *b;
        let b_base_ptr = fb.block_params(entry)[1];
        // void *out;
        let out_base_ptr = fb.block_params(entry)[2];
        // i64 len;
        let len = fb.block_params(entry)[3];

        let zero_f64 = fb.ins().f64const(0.0);
        let zero_i64 = fb.ins().iconst(ir::types::I64, 0);
        let one_f64 = fb.ins().f64const(1.0);
        let zero_f64x2 = fb.ins().splat(vec_ty, zero_f64);
        let step_size_i64 = fb.ins().iconst(ir::types::I64, step_size as i64);

        let mut regs = vec![];
        for i in 0..16 {
            let l = new_var();
            let u = new_var();
            fb.declare_var(l, vec_ty);
            fb.declare_var(u, vec_ty);
            fb.def_var(l, zero_f64x2);
            fb.def_var(u, zero_f64x2);

            regs.push((l, u));
        }

        let loop_head = fb.create_block();
        let loop_body = fb.create_block();
        let loop_exit = fb.create_block();

        // i64 indx = 0;
        let indx_var = new_var();
        fb.declare_var(indx_var, i64_ty);
        fb.def_var(indx_var, zero_i64);

        // void *a_ptr = a_base_ptr;
        let a_ptr_var = new_var();
        fb.declare_var(a_ptr_var, ptr_ty);
        fb.def_var(a_ptr_var, a_base_ptr);

        // void *b_ptr = a_base_ptr;
        let b_ptr_var = new_var();
        fb.declare_var(b_ptr_var, ptr_ty);
        fb.def_var(b_ptr_var, b_base_ptr);

        // void *out_ptr = out_base_ptr;
        let out_ptr_var = new_var();
        fb.declare_var(out_ptr_var, ptr_ty);
        fb.def_var(out_ptr_var, out_base_ptr);

        //TODO: len not div by step_size
        fb.ins().jump(loop_head, &[]);

        // LOOP HEAD //
        {
            fb.switch_to_block(loop_head);
            let indx = fb.use_var(indx_var);
            let indx_next = fb.ins().iadd_imm(indx, step_size as i64);
            // set indx = indx_next at end of loop

            // if indx_next > len  { jmp LOOP_EXIT } else { jmp LOOP_BODY }
            let cmp = fb
                .ins()
                .icmp(IntCondCode::UnsignedGreaterThan, indx_next, len);
            fb.ins().brif(cmp, loop_exit, &[], loop_body, &[]);
        }

        // LOOP BODY //
        {
            fb.switch_to_block(loop_body);
            // let fn_ref = loc_fns["print_hello"];
            // let call = fb.ins().call(fn_ref, &[]);
            // i64 lower_addr = indx * step_size;
            // i64 upper_addr = lower_addr + 1;

            // is_aligned flag?
            let load_flags = ir::MemFlags::new();
            let offset_l = 0;
            let offset_u = (vec_ty.lane_count() * 8) as i32;
            let stride = (8 * step_size) as i64;

            let a_ptr = fb.use_var(a_ptr_var);
            let a_l = fb.ins().load(vec_ty, load_flags, a_ptr, offset_l);
            let a_u = fb.ins().load(vec_ty, load_flags, a_ptr, offset_u);

            let b_ptr = fb.use_var(b_ptr_var);
            let b_l = fb.ins().load(vec_ty, load_flags, b_ptr, offset_l);
            let b_u = fb.ins().load(vec_ty, load_flags, b_ptr, offset_u);

            fb.def_var(regs[0].0, a_l);
            fb.def_var(regs[0].1, a_u);
            fb.def_var(regs[1].0, b_l);
            fb.def_var(regs[1].1, b_u);

            let use_oprnd = |oprnd: Oprnd, fb: &mut FunctionBuilder| match oprnd {
                Oprnd::Reg(indx) => {
                    let indx = indx as usize;
                    let (l, u) = (regs[indx].0, regs[indx].1);
                    (fb.use_var(l), fb.use_var(u))
                }
                Oprnd::Imm(imm) => {
                    let imm_f = fb.ins().f64const(imm);
                    let imm_v = fb.ins().splat(vec_ty, imm_f);
                    (imm_v, imm_v)
                }
            };

            let res_slot = fb.create_sized_stack_slot(ir::StackSlotData {
                kind: ir::StackSlotKind::ExplicitSlot,
                size: stride as u32,
                align_shift: 16,
            });
            // let slot = fb.create_stack_slot(StackSlotData {
            //     kind:   StackSlotKind::ExplicitSlot,
            //     size:   8,      // bytes for one f64
            //     offset: None,
            // });


            for instr in bytecode {
                match *instr {
                    Instr::UnOp { op, val, dst } => {
                        let (val_l, val_u) = use_oprnd(val, &mut fb);
                        let dst = dst as usize;
                        match op {
                            UnOp::MOV => {
                                fb.def_var(regs[dst].0, val_l);
                                fb.def_var(regs[dst].1, val_u);
                            }
                            UnOp::SIN => {
                                let fn_ref = loc_fns["sin_f64x2x4"];
                                let res_addr = fb.ins().stack_addr(ptr_ty, res_slot, 0);
                                let _ = fb.ins().call(fn_ref, &[res_addr, val_l, val_u]);

                                let res_l = fb.ins().load(vec_ty, ir::MemFlags::new(), res_addr, offset_l);
                                let res_u = fb.ins().load(vec_ty, ir::MemFlags::new(), res_addr, offset_u);
                                fb.def_var(regs[dst].0, res_l);
                                fb.def_var(regs[dst].1, res_u);
                            }
                            c_op => {
                                todo!()
                            }
                        }
                    }
                    Instr::BinOp { op, lhs, rhs, dst } => {
                        let dst = dst as usize;
                        let (lhs_l, lhs_u) = use_oprnd(lhs, &mut fb);
                        let (rhs_l, rhs_u) = use_oprnd(rhs, &mut fb);
                        let (res_l, res_u) = match op {
                            BinOp::ADD => {
                                let res_l = fb.ins().fadd(lhs_l, rhs_l);
                                let res_u = fb.ins().fadd(lhs_u, rhs_u);
                                (res_l, res_u)
                            }
                            BinOp::SUB => {
                                let res_l = fb.ins().fsub(lhs_l, rhs_l);
                                let res_u = fb.ins().fsub(lhs_u, rhs_u);
                                (res_l, res_u)
                            }
                            BinOp::MUL => {
                                let res_l = fb.ins().fmul(lhs_l, rhs_l);
                                let res_u = fb.ins().fmul(lhs_u, rhs_u);
                                (res_l, res_u)
                            }
                            BinOp::DIV => {
                                let fn_ref = loc_fns["div_f64x2x2"];
                                let res_addr = fb.ins().stack_addr(ptr_ty, res_slot, 0);
                                let _ = fb.ins().call(fn_ref, &[res_addr, lhs_l, lhs_u, rhs_l, rhs_u]);

                                let res_l = fb.ins().load(vec_ty, ir::MemFlags::new(), res_addr, offset_l);
                                let res_u = fb.ins().load(vec_ty, ir::MemFlags::new(), res_addr, offset_u);
                                (res_l, res_u)
                                // let res_l = fb.ins().fdiv(lhs_l, rhs_l);
                                // let res_u = fb.ins().fdiv(lhs_u, rhs_u);
                                // (res_l, res_u)
                            }
                        };
                        fb.def_var(regs[dst].0, res_l);
                        fb.def_var(regs[dst].1, res_u);
                    }
                }
            }

            let res_l = fb.use_var(regs[0].0);
            let res_u = fb.use_var(regs[0].1);

            let load_flags = ir::MemFlags::new();
            let out_ptr = fb.use_var(out_ptr_var);
            fb.ins().store(load_flags, res_l, out_ptr, offset_l);
            fb.ins().store(load_flags, res_u, out_ptr, offset_u);

            let new_a_ptr = fb.ins().iadd_imm(a_ptr, stride);
            let new_b_ptr = fb.ins().iadd_imm(b_ptr, stride);
            let new_out_ptr = fb.ins().iadd_imm(out_ptr, stride);

            fb.def_var(a_ptr_var, new_a_ptr);
            fb.def_var(b_ptr_var, new_b_ptr);
            fb.def_var(out_ptr_var, new_out_ptr);

            // indx = indx + step_size;
            let indx = fb.use_var(indx_var);
            let indx_next = fb.ins().iadd_imm(indx, step_size as i64);
            fb.def_var(indx_var, indx_next);
            fb.ins().jump(loop_head, &[]);
            fb.seal_block(loop_body);
            fb.seal_block(loop_head);
        }

        // LOOP EXIT //
        {
            fb.switch_to_block(loop_exit);
            fb.ins().return_(&[]);
            fb.seal_all_blocks();
            fb.finalize();
        }

        let fn_id = self
            .module
            .declare_function(name, Linkage::Local, &sig)
            .unwrap();
        self.ctx.func.name = ir::UserFuncName::user(0, fn_id.as_u32());

        self.module.define_function(fn_id, &mut self.ctx).unwrap();
        self.module.clear_context(&mut self.ctx);
        self.module.finalize_definitions().unwrap();

        let fn_ptr = self.module.get_finalized_function(fn_id);
        unsafe { std::mem::transmute(fn_ptr) }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand;

    #[test]
    fn compile_f64() {
        let program = bytecode! {
            ADD[0, 1] -> 0,
            ADD[imm(0.1), 0] -> 0,
        };

        let mut jit = JITCompiler::init();
        let f = jit.compile_for_f64("jit_fn", &program);
        // let f: extern "C" fn(f64, f64) -> f64 = unsafe { std::mem::transmute(fn_ptr) };
        let res = f(2.0, 3.0);
        assert_eq!(res, 5.1);
    }

    #[test]
    fn compile_f64x2() {
        unsafe {
            std::env::set_var("RUST_BACKTRACE", "1");
        }
        let program = bytecode! {
            SIN[0] -> 0,
            // ADD[0, 1] -> 0,
            ADD[imm(0.1), 0] -> 0,
        };

        let mut jit = JITCompiler::init();
        let f_f64 = jit.compile_for_f64("jit_fn", &program);
        let f_f64x2 = jit.compile_for_f64x2xn("jit_fn_vec", &program);

        let a = [0.0; 1028].map(|_| rand::random());
        let b = [0.0; 1028].map(|_| rand::random());

        let mut out = [0.0; 1028];
        f_f64x2(a.as_ptr(), b.as_ptr(), out.as_mut_ptr(), out.len() as i64);

        for ((x, y), o) in a.into_iter().zip(b).zip(out) {
            assert!(f_f64(x, y) - o < f32::EPSILON as f64);
        }
    }
}
