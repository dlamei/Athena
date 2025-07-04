use std::time::Duration;

use atlas::{
    iso::{self, Iso2DConfig, Program},
    vm,
};
use compiler::jit2::{self, F64X2};
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use utils::Intrvl;

fn jit_native_vs_interpreter(c: &mut Criterion) {
    let jit = jit2::JIT::init();

    let mut vm = vm::VM::with_instr_table(vm::IntrvlInstrTable);

    let f1_intrvl = jit.compile_2intrvl_intrvl("f_intrvl", &Program::Dense2.bytecode());

    let op_code = Program::Dense2.opcode();

    let intrvl_x = F64X2(-0.1, 0.1);
    let intrvl_y = F64X2(-0.1, 0.1);

    let mut out = F64X2::ZERO;

    c.bench_function("intrvl_native", |b| {
        b.iter(|| f1_intrvl(intrvl_x, intrvl_y, &mut out))
    });
    c.bench_function("intrvl_interpreter", |b| {
        b.iter(|| {
            vm.reg[1] = Intrvl::new(intrvl_x.0, intrvl_x.1);
            vm.reg[2] = Intrvl::new(intrvl_y.0, intrvl_y.1);

            vm.eval(black_box(&op_code))
        })
    });

    // c.benchmark_group(BenchmarkId::new("native_intrvl"), &(xs, ys), |b, &
}

criterion_group!(benches, jit_native_vs_interpreter,);
criterion_main!(benches);

// criterion_group!(
//     benches,
//     // tree_graph_2d_collapse,
// );
// criterion_main!(benches);
