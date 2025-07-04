use compiler::{
    jit,
    jit2::{self, F64X2, Intrvl},
};

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use rayon::prelude::*;

fn jit_native_vs_interpreter(c: &mut Criterion) {
    let prgrm = compiler::bytecode! {
        DIV[imm(1.0), 0] -> 0,
        DIV[imm(1.0), 1] -> 1,
        SIN[0] -> 2,
        COS[1] -> 3,
        ADD[2, 3] -> 2,
        SIN[2] -> 2,
        MUL[0, 1] -> 4,
        SIN[4] -> 4,
        COS[0] -> 5,
        ADD[4, 5] -> 4,
        COS[4] -> 4,
        SUB[2, 4] -> 0,
    };

    let jit = jit2::JIT::init();

    let f1_intrvl = jit.compile_2intrvl_intrvl("f_intrvl", &prgrm);

    let intrvl_x = F64X2(-0.1, 0.1);
    let intrvl_y = F64X2(-0.1, 0.1);

    let mut out = F64X2::ZERO;

    c.bench_function("intrvl_native", |b| {
        b.iter(|| f1_intrvl(intrvl_x, intrvl_y, &mut out))
    });
    c.bench_function("intrvl_interpreter", |b| b.iter(|| f1_intrvl()));

    // c.benchmark_group(BenchmarkId::new("native_intrvl"), &(xs, ys), |b, &
}

criterion_group!(benches,);
criterion_main!(benches);
