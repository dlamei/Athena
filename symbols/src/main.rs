use std::{fmt, ops, rc::Rc};

#[derive(Debug, Clone, Copy, PartialEq)]
enum AtomRef<'a> {
    Undef,
    U32(u32),
    Var(&'a str),
    Neg(&'a Atom),
    Sum(&'a [Atom]),
    Prod(&'a [Atom]),
    Sub(&'a [Atom; 2]),
    Div(&'a [Atom; 2]),
    Pow(&'a [Atom; 2]),
}

impl<'a> AtomRef<'a> {
    fn is_i32(&self, val: i32) -> bool {
        match self {
            AtomRef::U32(v) if val >= 0 => *v == val.unsigned_abs(),
            AtomRef::Neg(Atom::U32(v)) if val < 0 => *v == val.unsigned_abs(),
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq)]
enum AtomMut<'a> {
    Atom(&'a mut Atom),
    Neg(&'a mut Atom),
    Sum(&'a mut [Atom]),
    Prod(&'a mut [Atom]),
    Sub(&'a mut [Atom; 2]),
    Div(&'a mut [Atom; 2]),
    Pow(&'a mut [Atom; 2]),
}

#[derive(Debug, Clone, PartialEq)]
enum Atom {
    Undef,
    U32(u32),
    Var(Rc<str>),
    // TODO: RcOrBox
    Expr(Box<Expr>),
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Atom::Undef => write!(f, "undef"),
            Atom::U32(val) => write!(f, "{val}"),
            Atom::Var(var) => write!(f, "{}", *var),
            Atom::Expr(expr) => write!(f, "{expr:?}"),
        }
    }
}

impl Atom {
    fn atom_ref<'a>(&'a self) -> AtomRef<'a> {
        match self {
            Atom::Undef => AtomRef::Undef,
            Atom::U32(val) => AtomRef::U32(*val),
            Atom::Var(var) => AtomRef::Var(var.as_ref()),
            Atom::Expr(expr) => expr.atom_ref(),
        }
    }

    fn atom_mut<'a>(&'a mut self) -> AtomMut<'a> {
        match self {
            Atom::Undef
            | Atom::U32(_)
            | Atom::Var(_) => AtomMut::Atom(self),
            Atom::Expr(expr) => expr.atom_mut(),
        }
    }

}

#[derive(Debug, Clone, PartialEq)]
enum Expr {
    Atom(Atom),
    Neg(Atom),

    Sum(Vec<Atom>),
    Prod(Vec<Atom>),

    Sub([Atom; 2]),
    Div([Atom; 2]),
    Pow([Atom; 2]),
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Atom(atom) => write!(f, "{atom}"),
            Expr::Neg(Atom::Expr(expr)) => write!(f, "-({expr})"),
            Expr::Neg(atom) => write!(f, "-{atom}"),
            Expr::Sum(oprnds) => {
                if oprnds.is_empty() {
                    return write!(f, "[+]")
                }
                let mut op_iter = oprnds.iter();
                write!(f, "{}", op_iter.next().unwrap())?;
                for op in op_iter {
                    write!(f, " + {op}")?;
                }
                Ok(())
            },
            Expr::Prod(oprnds) => {
                if oprnds.is_empty() {
                    return write!(f, "[*]")
                }
                let mut op_iter = oprnds.iter();
                write!(f, "{}", op_iter.next().unwrap())?;
                for op in op_iter {
                    write!(f, " * {op}")?;
                }
                Ok(())
            }
            Expr::Sub([lhs, rhs]) => {
                write!(f, "{lhs} - ")?;
                if matches!(rhs, Atom::Expr(_))  {
                    write!(f, "({rhs})")
                } else {
                    write!(f, "{rhs}")
                }
            },
            Expr::Div([lhs, rhs]) => {
                if matches!(lhs, Atom::Expr(_))  {
                    write!(f, "({lhs})/")?;
                } else {
                    write!(f, "{lhs}/")?;
                }
                if matches!(rhs, Atom::Expr(_))  {
                    write!(f, "({rhs})")
                } else {
                    write!(f, "{rhs}")
                }
            },
            Expr::Pow([lhs, rhs]) => {
                if matches!(lhs, Atom::Expr(_))  {
                    write!(f, "({lhs})^")?;
                } else {
                    write!(f, "{lhs}^")?;
                }
                if matches!(rhs, Atom::Expr(_))  {
                    write!(f, "({rhs})")
                } else {
                    write!(f, "{rhs}")
                }
            },
        }
    }
}

impl Expr {

    fn atom_ref<'a>(&'a self) -> AtomRef<'a> {
        match self {
            Expr::Atom(atom) => atom.atom_ref(),
            Expr::Neg(atom) => AtomRef::Neg(atom),
            Expr::Sum(oprnds) => AtomRef::Sum(oprnds.as_slice()),
            Expr::Prod(oprnds) => AtomRef::Prod(oprnds.as_slice()),
            Expr::Sub(oprnds) => AtomRef::Sub(oprnds),
            Expr::Div(oprnds) => AtomRef::Div(oprnds),
            Expr::Pow(oprnds) => AtomRef::Pow(oprnds),
        }
    }

    fn atom_mut<'a>(&'a mut self) -> AtomMut<'a> {
        match self {
            Expr::Atom(atom) => atom.atom_mut(),
            Expr::Neg(atom) => AtomMut::Neg(atom),
            Expr::Sum(oprnds) => AtomMut::Sum(oprnds.as_mut_slice()),
            Expr::Prod(oprnds) => AtomMut::Prod(oprnds.as_mut_slice()),
            Expr::Sub(oprnds) => AtomMut::Sub(oprnds),
            Expr::Div(oprnds) => AtomMut::Div(oprnds),
            Expr::Pow(oprnds) => AtomMut::Pow(oprnds),
        }
    }

    fn oprnds<'a>(&'a self) -> &[Atom] {
        match self {
            Expr::Atom(atom) | Expr::Neg(atom) => std::slice::from_ref(atom),
            Expr::Sum(vec) | Expr::Prod(vec) => vec.as_slice(),
            Expr::Sub(oprnds) | Expr::Div(oprnds) | Expr::Pow(oprnds) => oprnds,
        }
    }

    fn oprnds_mut(&mut self) -> &mut [Atom] {
        match self {
            Expr::Atom(atom) | Expr::Neg(atom) => std::slice::from_mut(atom),
            Expr::Sum(vec) | Expr::Prod(vec) => vec.as_mut_slice(),
            Expr::Sub(oprnds) | Expr::Div(oprnds) | Expr::Pow(oprnds) => oprnds,
        }
    }


    fn base<'a>(&'a self) -> AtomRef<'a> {
        match self {
            Expr::Atom(_)
            | Expr::Neg(_)
            | Expr::Sum(_)
            | Expr::Prod(_)
            | Expr::Sub(_)
            | Expr::Div(_) => self.atom_ref(),
            Expr::Pow([base, _]) => base.atom_ref(),
        }
    }

    fn exponent<'a>(&'a self) -> AtomRef<'a> {
        match self {
            Expr::Atom(_) | Expr::Neg(_) | Expr::Sum(_) | Expr::Prod(_) | Expr::Sub(_) | Expr::Div(_) => AtomRef::U32(1),
            Expr::Pow([_, expon]) => expon.atom_ref(),
        }
    }
}

impl From<u32> for Expr {
    fn from(value: u32) -> Self {
        Expr::Atom(Atom::U32(value))
    }
}

impl From<i32> for Expr {
    fn from(value: i32) -> Self {
        let mut atom = Atom::U32(value.unsigned_abs());
        if value > 0 {
            Expr::Atom(atom)
        } else {
            Expr::Neg(atom)
        }
    }
}

impl From<&str> for Expr {
    fn from(value: &str) -> Self {
        Self::Atom(Atom::Var(value.into()))
    }
}

fn main() {
    let val = Expr::from(-34);
    println!("{}", val);
}
