use std::fmt;

use crate::expr::EvalMode;

/// Global configuration for Noctua operations.
///
/// These configurations are, for example, used to determine the behaviour when using [`std::ops::Add`],
/// [`std::ops::Mul`] etc...
///
/// # Examples
///
/// ```rust
/// use noctua::{Expr, config, noctua};
///
/// // Create a custom configuration
/// let cfg = config::NoctuaConfig::current()
///             .with_default_eval(noctua::EvalMode::expand());
///
/// {
///     let _scoped = cfg.install();
///     // inside this block, the custom configurations now apply
///
///     let e1 = noctua! { (a + b)^2 };
///     let mut e2 = e1.clone().expand();
///
///     // because of our custom config [`Expr::pow`] has already expanded the expression
///     assert_eq!(e1, e2); // e1 == e2
///
/// }
///
/// let e1 = noctua! { (a + b)^2 };
/// let mut e2 = e1.clone().expand();
///
/// // After the scoped config is dropped, the config is restored to the previous state
/// // our expression is no longer expanded by default
/// assert_ne!(e1, e2); // e1 != e2
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoctuaConfig {
    pub default_eval_mode: EvalMode,
    pub expr_dbg_fmt: ExprFmtFn,
    pub expr_fmt: ExprFmtFn,
    // pub expr_dbg_format: fn(&crate::Expr, &mut fmt::Formatter<'_>) -> fmt::Result,
}

impl NoctuaConfig {
    pub fn current() -> Self {
        noctua_global_config()
    }

    pub fn with_default_eval(mut self, mode: EvalMode) -> Self {
        self.default_eval_mode = mode;
        self
    }

    pub fn with_dbg_expr_fmt(
        mut self,
        fmt_fn: fn(&crate::Expr, &mut fmt::Formatter<'_>) -> fmt::Result,
    ) -> Self {
        self.expr_dbg_fmt = ExprFmtFn(fmt_fn);
        self
    }

    pub fn with_expr_fmt(
        mut self,
        fmt_fn: fn(&crate::Expr, &mut fmt::Formatter<'_>) -> fmt::Result,
    ) -> Self {
        self.expr_fmt = ExprFmtFn(fmt_fn);
        self
    }

    /// Installs `self` as the global configuration, returning
    /// a [`ScopedConfig`] guard that restores the previous
    /// configuration on drop.
    pub fn install(self) -> ScopedConfig {
        let mut guard = NOCTUA_CONFIG.write().unwrap();
        let old = *guard;
        *guard = self;
        ScopedConfig { old }
    }
}

// impl PartialEq for NoctuaConfig {
//     fn eq(&self, other: &Self) -> bool {
//         self.default_eval_mode == other .default_eval_mode
//             && std::ptr::eq(&self.expr_dbg_format, &other.expr_dbg_format)
//     }
// }

impl Default for NoctuaConfig {
    fn default() -> Self {
        Self {
            default_eval_mode: EvalMode::default(),
            expr_dbg_fmt: ExprFmtFn(crate::Expr::ascii_fmt),
            expr_fmt: ExprFmtFn(crate::Expr::unicode_fmt),
        }
    }
}

static NOCTUA_CONFIG: once_cell::sync::Lazy<std::sync::RwLock<NoctuaConfig>> =
    once_cell::sync::Lazy::new(|| std::sync::RwLock::new(NoctuaConfig::default()));

/// A guard that sets a new global [`NoctuaConfig`] for the current scope and
/// restores the old configuration when dropped.
///
/// # Example
///
/// ```rust
/// use noctua::config::{NoctuaConfig, noctua_global_config};
///
/// let new_cfg = NoctuaConfig { /* custom config */ ..Default::default() };
/// # let new_cfg = NoctuaConfig { default_eval_mode: noctua::EvalMode::frozen(), ..Default::default() };
/// {
///     let _guard = new_cfg.install();
///     // global config is now `new_cfg`
///     assert_eq!(noctua_global_config(), new_cfg);
///     assert_ne!(noctua_global_config(), NoctuaConfig::default());
/// }
/// // original config is restored
/// assert_eq!(noctua_global_config(), NoctuaConfig::default());
/// ```
pub struct ScopedConfig {
    old: NoctuaConfig,
}

impl ScopedConfig {
    pub fn install(new_cfg: NoctuaConfig) -> Self {
        let mut guard = NOCTUA_CONFIG.write().unwrap();
        let old = *guard;
        *guard = new_cfg;
        Self { old }
    }
}

impl Drop for ScopedConfig {
    fn drop(&mut self) {
        let mut guard = NOCTUA_CONFIG.write().unwrap();
        *guard = self.old;
    }
}

/// Returns a copy of the current global [`NoctuaConfig`].
///
pub fn noctua_global_config() -> NoctuaConfig {
    *NOCTUA_CONFIG.read().unwrap()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExprFmtFn(pub fn(&crate::Expr, &mut fmt::Formatter<'_>) -> fmt::Result);

impl ExprFmtFn {
    pub fn fmt(&self, e: &crate::Expr, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.0)(e, f)
    }

    pub fn fmt_string(&self, e: &crate::Expr) -> String {
        struct Adapter<'a>(&'a crate::Expr, ExprFmtFn);

        impl fmt::Display for Adapter<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.1.fmt(self.0, f)
            }
        }

        format!("{}", Adapter(e, *self))
    }
}
