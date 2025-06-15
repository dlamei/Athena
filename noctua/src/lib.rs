pub mod config;
// pub mod expr;
pub mod expr;
// pub mod expr_old;
pub mod flat_deque;
pub mod real;

// pub use expr::{Atom, Expr};
pub use expr::Expr;
pub use noctua_macros::{log_fn, noctua};

pub extern crate self as noctua;

use itertools::Itertools;

noctua_macros::setup_fn_log! {
    dbg: false,
}

pub fn run() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .filter_module("noctua", log::LevelFilter::Trace)
        // .filter_module("wgpu_hal::auxil::dxgi", log::LevelFilter::Error)
        // .filter_module("wgpu_hal::auxil::dxgi", log::LevelFilter::Warn)
        .format_timestamp(None)
        .init();

    // let a = noctua!(x + 3);
    // let b = noctua!(x + y);
    // a.simplified_ordering(&b);

    // let a = noctua!(1+x);
    // let b = noctua!(y);

    // println!("{:?}", a.simplified_ordering(&b));

    let a = noctua!(((x ^ (1 / 2)) ^ (1 / 2)) ^ 8);
    println!("{a:?}");

    // println!("{}", noctua!((a + b) ^ 2).expand());
    // let cnst_term = noctua!(x).simplified_ordering(&noctua!(x^2));
    // println!("{}", cnst_term);

    // println!("{:?}", cnst_term.view_const().unwrap());
    // println!("{:?}", cnst_term.view_term().unwrap());

    // let mut terms = [
    //     noctua!(x + y),
    //     noctua!(y ^ 2),
    //     noctua!(x),
    //     noctua!(x ^ 2 + x ^ 3),
    //     noctua!(1 + y - x),
    //     noctua!(x ^ 2),
    //     noctua!(1 + 3),
    //     noctua!(x ^ 2 + y ^ 3),
    //     noctua!(1 + 3),
    // ];

    // println!("unsorted:\n{:?}\n\n", terms.iter().format("\n"));
    // terms.sort_by(Expr::simplified_ordering);

    // println!("sorted:\n{:?}\n\n", terms.iter().format("\n"));

    // println!("{:?}", noctua!((x + y) ^ 3));
    // println!("{:?}", noctua!(((a+b) + y) ^ 5).expand());
}
