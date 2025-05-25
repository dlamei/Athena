pub mod config;
pub mod expr;
pub mod flat_deque;
pub mod real;

pub use expr::Expr;
pub use noctua_macros::noctua;

use itertools::Itertools;

pub fn run() {
    let mut terms = [
        noctua!(x + y),
        noctua!(x),
        noctua!(x^2 + x^3),
        noctua!(1 + y - x),
        noctua!(x^2),
        noctua!(1 + 3),
        noctua!(x^2 + y^3),
        noctua!(1 + 3),
    ];

    println!("unsorted:\n{}", terms.iter().format("\n"));
    terms.sort_by(Expr::simplified_ordering);

    println!("\n\nsorted:\n{}", terms.iter().format("\n"));
}
