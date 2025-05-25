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
/// let cfg = config::NoctuaConfig {
///     default_pow_strategy: config::PowStrategy::Expand,
///     ..Default::default()
/// };
///
/// {
///     let _scoped = config::ScopedConfig::install(cfg);
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
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoctuaConfig {
    pub default_add_strategy: AddStrategy,
    pub default_mul_strategy: MulStrategy,
    pub default_pow_strategy: PowStrategy,
}

static NOCTUA_CONFIG: once_cell::sync::Lazy<std::sync::RwLock<NoctuaConfig>> =
    once_cell::sync::Lazy::new(|| std::sync::RwLock::new(NoctuaConfig::default()));

/// A guard that sets a new global [`NoctuaConfig`] for the current scope and
/// restores the old configuration when dropped.
///
/// # Example
///
/// ```rust
/// use noctua::config::{NoctuaConfig, ScopedConfig, noctua_global_config};
///
/// let new_cfg = NoctuaConfig { /* custom config */ ..Default::default() };
/// # let new_cfg = NoctuaConfig { default_add_strategy: noctua::config::AddStrategy::None, ..Default::default() };
/// {
///     let _guard = ScopedConfig::install(new_cfg);
///     // global config is now `new_cfg`
///     assert_eq!(noctua_global_config(), new_cfg);
/// }
/// // original config is restored
/// assert_eq!(noctua_global_config(), NoctuaConfig::default());
/// ```
pub struct ScopedConfig {
    old: NoctuaConfig,
}

impl ScopedConfig {
    /// Installs `new_cfg` as the global configuration, returning
    /// a `NoctuaScopedConfig` guard that restores the previous
    /// configuration on drop.
    pub fn install(new_cfg: NoctuaConfig) -> Self {
        let mut guard = NOCTUA_CONFIG.write().unwrap();
        let old = guard.clone();
        *guard = new_cfg;
        Self { old }
    }
}

impl Drop for ScopedConfig {
    fn drop(&mut self) {
        let mut guard = NOCTUA_CONFIG.write().unwrap();
        *guard = self.old.clone();
    }
}

/// Returns a copy of the current global [`NoctuaConfig`].
///
pub fn noctua_global_config() -> NoctuaConfig {
    NOCTUA_CONFIG.read().unwrap().clone()
}

/// Determine how multiplications are handled
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MulStrategy {
    /// Do not perform any simplifications
    None,
    /// Perform basic simplifications like merging sum expressions and removing zeros
    #[default]
    Simple,
    /// When multiplying multiple expressions, noctua will split expressions into `base` and `exponent`
    /// and sum up the `exponents` of expressions with matching bases
    Base,
    /// Expand the multiplied expressions
    Expand,
}

/// Determine how powers are handled
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PowStrategy {
    /// Do not perform any simplifications
    None,
    /// Perform basic simplifications like merging the product of [`Expr::Prod`]
    #[default]
    Simple,
    /// Expand the expression when possible
    Expand,
}

/// Determine how additions are handled
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AddStrategy {
    /// Prevent the discarding of zeros or the handling of [`Atom::Undef`]
    None,
    /// Perform basic simplifications like merging the sum of [`Expr::Sum`]
    #[default]
    Simple,
    /// When adding multiple expressions, noctua will split expressions into `coeff` and `term`
    /// and sum up the `coeff`'s of expressions with matching terms
    Coeff,
}
