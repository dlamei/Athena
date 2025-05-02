use compiler::jit;

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use rayon::prelude::*;

fn jit_basic_op(c: &mut Criterion) {
    let prgrm = compiler::bytecode! {
        ADD[0, 1] -> 0,
        SUB[1, 0] -> 1,
        MUL[0, 1] -> 0,
        DIV[1, 0] -> 1,
        ADD[0, 1] -> 0,
        SUB[1, 0] -> 1,
        MUL[0, 1] -> 0,
        DIV[1, 0] -> 1,
        MOV[1] -> 0,
    };

    let mut jit = jit::JITCompiler::init();
    let config = jit::CompConfig::default();
    let f1 = jit.compile_for_f64("f64", &prgrm, &config).fn_ptr;
    let f2 = jit.compile_for_f64x2("f64x2", &prgrm, &config).fn_ptr;
    let f3 = jit.compile_for_f64x2x4("f64x2x4", &prgrm, &config).fn_ptr;
    // let f4 = jit.compile_for_f64x2xn("f64x2xn", &prgrm, &config);

    const N: usize = 1024;
    let xs: [f64; N] = rand::random();
    let ys: [f64; N] = rand::random();

    c.bench_with_input(BenchmarkId::new("f64_basic", N), &(xs, ys), |b, &params| {
        b.iter(|| {
            for i in 0..N {
                let x = params.0[i];
                let y = params.1[i];
                let _ = f1(x, y);
            }
        })
    });
    c.bench_with_input(
        BenchmarkId::new("f64x2_basic", N),
        &(xs, ys),
        |b, &params| {
            b.iter(|| {
                for i in (0..N).step_by(2) {
                    let x: [f64; 2] = params.0[i..i + 2].try_into().unwrap();
                    let y: [f64; 2] = params.1[i..i + 2].try_into().unwrap();
                    let mut o = [0.0; 2];
                    let _ = f2(&mut o, x, y);
                }
            })
        },
    );
    c.bench_with_input(
        BenchmarkId::new("f64x8_basic", N),
        &(xs, ys),
        |b, &params| {
            b.iter(|| {
                for i in (0..N).step_by(8) {
                    let x: [f64; 8] = params.0[i..i + 8].try_into().unwrap();
                    let y: [f64; 8] = params.1[i..i + 8].try_into().unwrap();
                    let mut o = [0.0; 8];
                    let _ = f3(&x, &y, &mut o);
                }
            })
        },
    );
}

fn jit_trig_op(c: &mut Criterion) {
    let prgrm = compiler::bytecode! {
        SIN[0] -> 1,
        COS[1] -> 0,
        TAN[0] -> 1,
        SIN[1] -> 0,
        COS[0] -> 1,
        TAN[1] -> 0,
    };

    let mut jit = jit::JITCompiler::init();
    let config = jit::CompConfig::default();
    let f1 = jit.compile_for_f64("f64", &prgrm, &config).fn_ptr;
    let f2 = jit.compile_for_f64x2("f64x2", &prgrm, &config).fn_ptr;
    let f3 = jit.compile_for_f64x2x4("f64x2x4", &prgrm, &config).fn_ptr;

    const N: usize = 1024;
    let xs: [f64; N] = rand::random();
    let ys: [f64; N] = rand::random();

    c.bench_with_input(BenchmarkId::new("f64_trig", N), &(xs, ys), |b, &params| {
        b.iter(|| {
            for i in 0..N {
                let x = params.0[i];
                let y = params.1[i];
                let _ = f1(x, y);
            }
        })
    });
    c.bench_with_input(
        BenchmarkId::new("f64x2_trig", N),
        &(xs, ys),
        |b, &params| {
            b.iter(|| {
                for i in (0..N).step_by(2) {
                    let x: [f64; 2] = params.0[i..i + 2].try_into().unwrap();
                    let y: [f64; 2] = params.1[i..i + 2].try_into().unwrap();
                    let mut o = [0.0; 2];
                    let _ = f2(&mut o, x, y);
                }
            })
        },
    );
    c.bench_with_input(
        BenchmarkId::new("f64x8_trig", N),
        &(xs, ys),
        |b, &params| {
            b.iter(|| {
                for i in (0..N).step_by(8) {
                    let x: [f64; 8] = params.0[i..i + 8].try_into().unwrap();
                    let y: [f64; 8] = params.1[i..i + 8].try_into().unwrap();
                    let mut o = [0.0; 8];
                    let _ = f3(&x, &y, &mut o);
                }
            })
        },
    );
}

fn jit_all_op(c: &mut Criterion) {
    let prgrm = compiler::bytecode! {
        ADD[imm(1.0), 0] -> 0,
        DIV[imm(1.0), 1] -> 1,
        SIN[0] -> 2,
        TAN[1] -> 3,
        ADD[2, 3] -> 2,
        COS[2] -> 2,
        MUL[0, 1] -> 4,
        SIN[4] -> 4,
        COS[0] -> 5,
        ADD[4, 5] -> 4,
        SIN[4] -> 4,
        ADD[2, 4] -> 0,
        DIV[imm(1.0), 0] -> 0,
        DIV[imm(1.0), 1] -> 1,
        SIN[0] -> 2,
        TAN[1] -> 3,
        ADD[2, 3] -> 2,
        SIN[2] -> 2,
        SUB[0, 1] -> 4,
        SIN[4] -> 4,
        COS[0] -> 5,
        ADD[4, 5] -> 4,
        SIN[4] -> 4,
        DIV[2, 4] -> 0,
    };

    let mut jit = jit::JITCompiler::init();
    let config = jit::CompConfig::default();
    let f1 = jit.compile_for_f64("f64", &prgrm, &config).fn_ptr;
    let f2 = jit.compile_for_f64x2("f64x2", &prgrm, &config).fn_ptr;
    let f3 = jit.compile_for_f64x2x4("f64x2x4", &prgrm, &config).fn_ptr;
    let f4 = jit.compile_for_f64x2xn("f64x2xn", &prgrm, &config).fn_ptr;

    const N: usize = 1024 * 1024;
    let xs: Vec<_> = rand::random_iter().take(N).collect();
    let ys: Vec<_> = rand::random_iter().take(N).collect();

    c.bench_with_input(BenchmarkId::new("f64_all", N), &(&xs, &ys), |b, &params| {
        b.iter(|| {
            for i in 0..N {
                let x = params.0[i];
                let y = params.1[i];
                let _ = f1(x, y);
            }
        })
    });
    c.bench_with_input(
        BenchmarkId::new("f64x2_all", N),
        &(&xs, &ys),
        |b, &params| {
            b.iter(|| {
                for i in (0..N).step_by(2) {
                    let x: [f64; 2] = params.0[i..i + 2].try_into().unwrap();
                    let y: [f64; 2] = params.1[i..i + 2].try_into().unwrap();
                    let mut o = [0.0; 2];
                    let _ = f2(&mut o, x, y);
                }
            })
        },
    );
    c.bench_with_input(
        BenchmarkId::new("f64x8_all", N),
        &(&xs, &ys),
        |b, &params| {
            b.iter(|| {
                for i in (0..N).step_by(8) {
                    let x: [f64; 8] = params.0[i..i + 8].try_into().unwrap();
                    let y: [f64; 8] = params.1[i..i + 8].try_into().unwrap();
                    let mut o = [0.0; 8];
                    let _ = f3(&x, &y, &mut o);
                }
            })
        },
    );
    c.bench_with_input(
        BenchmarkId::new("f64xn_all", N),
        &(&xs, &ys),
        |b, &params| {
            b.iter(|| {
                let mut out = vec![0.0; N];
                f4(xs.as_ptr(), ys.as_ptr(), out.as_mut_ptr(), N as i64);
            })
        },
    );
}

fn jit_all_op_par(c: &mut Criterion) {
    let prgrm = compiler::bytecode! {
        ADD[imm(1.0), 0] -> 0,
        DIV[imm(1.0), 1] -> 1,
        SIN[0] -> 2,
        TAN[1] -> 3,
        ADD[2, 3] -> 2,
        COS[2] -> 2,
        MUL[0, 1] -> 4,
        SIN[4] -> 4,
        COS[0] -> 5,
        ADD[4, 5] -> 4,
        SIN[4] -> 4,
        ADD[2, 4] -> 0,
        DIV[imm(1.0), 0] -> 0,
        DIV[imm(1.0), 1] -> 1,
        SIN[0] -> 2,
        TAN[1] -> 3,
        ADD[2, 3] -> 2,
        SIN[2] -> 2,
        SUB[0, 1] -> 4,
        SIN[4] -> 4,
        COS[0] -> 5,
        ADD[4, 5] -> 4,
        SIN[4] -> 4,
        DIV[2, 4] -> 0,
    };
    let prgrm = compiler::bytecode! [
        DIV[imm(1.0), 0] -> 0,
        DIV[imm(1.0), 1] -> 1,
        SIN[0] -> 2,
        SIN[1] -> 3,
        ADD[2, 3] -> 2,
        SIN[2] -> 2,
        MUL[0, 1] -> 4,
        SIN[4] -> 4,
        SIN[0] -> 5,
        ADD[4, 5] -> 4,
        SIN[4] -> 4,
        SUB[2, 4] -> 0,
    ];

    let mut jit = jit::JITCompiler::init();
    let config = jit::CompConfig::default();
    let f1 = jit.compile_for_f64("f64", &prgrm, &config).fn_ptr;
    let f2 = jit.compile_for_f64x2("f64x2", &prgrm, &config).fn_ptr;
    let f3 = jit.compile_for_f64x2x4("f64x2x4", &prgrm, &config).fn_ptr;

    const N: usize = 1024 * 1024;
    let xs: Vec<_> = rand::random_iter().take(N).collect();
    let ys: Vec<_> = rand::random_iter().take(N).collect();

    c.bench_with_input(
        BenchmarkId::new("f64_all_par", N),
        &(&xs, &ys),
        |b, &params| {
            b.iter(|| {
                (0..N).into_par_iter().for_each(|i| {
                    let x = params.0[i];
                    let y = params.1[i];
                    let _ = f1(x, y);
                })
            })
        },
    );
    c.bench_with_input(
        BenchmarkId::new("f64x2_all_par", N),
        &(&xs, &ys),
        |b, &params| {
            b.iter(|| {
                (0..N).into_par_iter().step_by(2).for_each(|i| {
                    let x: [f64; 2] = params.0[i..i + 2].try_into().unwrap();
                    let y: [f64; 2] = params.1[i..i + 2].try_into().unwrap();
                    let mut o = [0.0; 2];
                    let _ = f2(&mut o, x, y);
                })
            })
        },
    );
    c.bench_with_input(
        BenchmarkId::new("f64x8_all_par", N),
        &(&xs, &ys),
        |b, &params| {
            b.iter(|| {
                (0..N).into_par_iter().step_by(8).for_each(|i| {
                    let x: [f64; 8] = params.0[i..i + 8].try_into().unwrap();
                    let y: [f64; 8] = params.1[i..i + 8].try_into().unwrap();
                    let mut o = [0.0; 8];
                    let _ = f3(&x, &y, &mut o);
                })
            })
        },
    );
}

criterion_group!(
    benches,
    // jit_basic_op,
    // jit_trig_op,
    jit_all_op,
    jit_all_op_par,
);
criterion_main!(benches);
