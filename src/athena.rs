use std::fmt;

use calcu_rs::{Expr, SymbolicExpr};

#[derive(Clone)]
pub struct BuiltinFn {
    name: &'static str,
    args: &'static [&'static str],
    ptr: &'static dyn Fn(&[Expr]) -> Expr,
}

impl BuiltinFn {
    pub fn params(&self) -> &[&'static str] {
        self.args
    }

    pub fn n_params(&self) -> usize {
        self.args.len()
    }

    pub fn call(&self, args: &[Expr]) -> Expr {
        (self.ptr)(args)
    }
}

impl fmt::Display for BuiltinFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fn {}(", self.name)?;

        let mut args = self.params().iter();
        if let Some(a) = args.next() {
            write!(f, "{a}")?;
        }

        for a in args {
            write!(f, ", {a}")?;
        }
        write!(f, ")")
    }
}

impl fmt::Debug for BuiltinFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BuiltinFn")
            .field("args", &self.args)
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

macro_rules! call {
    ($fn:path, $a:expr, 1) => {{
        if $a.len() != 1 {
            return Expr::undef();
        }
        $fn(&$a[0])
    }};
    ($fn:path, $a:expr, 2) => {{
        if $a.len() != 2 {
            return Expr::undef();
        }
        $fn(&$a[0], &$a[1])
    }};
    ($fn:path, $a:expr, 3) => {{
        if $a.len() != 3 {
            return Expr::undef();
        }
        $fn(&$a[0], &$a[1], &$a[2])
    }};
}

macro_rules! builtin {
    ($name:ident, $fn:path, $args:expr, 1) => {
        BuiltinFn {
            name: stringify!($name),
            args: &$args,
            ptr: &|args: &[Expr]| call!($fn, args, 1),
        }
    };
    ($name:ident, $fn:path, $args:expr, 2) => {
        BuiltinFn {
            name: stringify!($name),
            args: &$args,
            ptr: &|args: &[Expr]| call!($fn, args, 2),
        }
    };
    ($name:ident, $fn:path, $args:expr, 3) => {
        BuiltinFn {
            name: stringify!($name),
            args: &$args,
            ptr: &|args: &[Expr]| call!($fn, args, 3),
        }
    };
}

pub const BUILTINS: &'static [BuiltinFn] = &[
    builtin!(sin, Expr::sin, ["x"], 1),
    builtin!(arcsin, Expr::arc_sin, ["x"], 1),
    builtin!(cos, Expr::cos, ["x"], 1),
    builtin!(arccos, Expr::arc_cos, ["x"], 1),
    builtin!(tan, Expr::tan, ["x"], 1),
    builtin!(arctan, Expr::arc_tan, ["x"], 1),
    builtin!(sec, Expr::sec, ["x"], 1),
    builtin!(ln, Expr::ln, ["x"], 1),
    builtin!(log10, Expr::log10, ["x"], 1),
    builtin!(exp, Expr::exp, ["x"], 1),
    builtin!(sqrt, Expr::sqrt, ["x"], 1),
    builtin!(deriv, Expr::derivative, ["f", "x"], 2),
    builtin!(numer, Expr::numerator, ["frac"], 1),
    builtin!(denom, Expr::denominator, ["frac"], 1),
    builtin!(base, Expr::base, ["power"], 1),
    builtin!(expon, Expr::exponent, ["power"], 1),
    builtin!(reduce, Expr::reduce, ["x"], 1),
    builtin!(expand, Expr::exponent, ["x"], 1),
    builtin!(expand_main, Expr::expand_main_op, ["x"], 1),
    builtin!(cancel, Expr::cancel, ["x"], 1),
    builtin!(rationalize, Expr::rationalize, ["x"], 1),
    builtin!(factor_out, Expr::factor_out, ["x"], 1),
    builtin!(common_factor, Expr::common_factors, ["a", "b"], 2),
    BuiltinFn {
        name: "free_of",
        args: &["expr", "x"],
        ptr: &|args: &[Expr]| Expr::from(call!(Expr::free_of, args, 2) as u32),
    },
];

pub fn get_builtin(name: &str) -> Option<&BuiltinFn> {
    BUILTINS.iter().find(|func| func.name == name)
}
