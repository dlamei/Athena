pub mod config;
// pub mod expr;
pub mod expr;
// pub mod expr_old;
pub mod flat_deque;
pub mod real;

use std::time::Instant;

pub use expr::EvalMode;
// pub use expr::{Atom, Expr};
pub use expr::Expr;
pub use noctua_macros::{log_fn, noctua};

pub extern crate self as noctua;

use noctua as n;

use itertools::Itertools;

noctua_macros::setup_fn_log! {
    dbg: false,
}

fn test2() {
    let a = n!((-x - y)^3).simplify();
    println!("{:?}", a);
}

fn doc_simplify(e: Expr) {
    log::info!("");
    log::info!("simplify: {e:?}:");
    let res = e.clone().simplify();
    log::info!("simplify: {e:?} -> {res:?}");
    log::info!("");
}

fn test1() {
    let config = config::NoctuaConfig {
        default_eval_mode: EvalMode::frozen(),
    };

    let scope = config::ScopedConfig::install(config);

    // let mut a = [n!(a * x^2), n!(x^3)];
    let mut a = n!(3 + a * x ^ 2 + b * x + c).operands().to_vec();
    a.sort_by(Expr::canon_order);
    println!("{a:?}\n");

    let mut a = n!(3 + a * x ^ 2 + b * x + c)
        .flatten_root()
        .operands()
        .to_vec();
    a.sort_by(Expr::canon_order);
    println!("{a:?}\n");

    let mut a = n!(3 + a * x ^ 2 + b * x + c).flatten();
    a.sort_operands_by(Expr::canon_order);
    println!("{a:?}\n");

    let mut a = [
        n!(x),
        -n!(x),
        n!(-1 * x),
        n!(-2 * x),
        n!(y),
        n!(-1 * y),
        -n!(y),
    ];
    a.sort_by(Expr::canon_order);
    println!("{a:?}");

    doc_simplify(n!(x + 2 * y - x - y).flatten());
    doc_simplify(n!(x + 2 * y - 1 * x - 1 * y).flatten());
    doc_simplify(n!(-1 * x + 1 * y + 1 * x).flatten());
}

pub fn run() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .filter_module("noctua", log::LevelFilter::Trace)
        // .filter_module("wgpu_hal::auxil::dxgi", log::LevelFilter::Error)
        // .filter_module("wgpu_hal::auxil::dxgi", log::LevelFilter::Warn)
        .format_timestamp(None)
        .init();

    test2();
}
