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
/// let cfg = config::NoctuaConfig {
///     default_eval_mode: noctua::EvalMode::expand(),
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
    pub default_eval_mode: EvalMode,
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
/// # let new_cfg = NoctuaConfig { default_eval_mode: noctua::EvalMode::frozen(), ..Default::default() };
/// {
///     let _guard = ScopedConfig::install(new_cfg);
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
    /// Installs `new_cfg` as the global configuration, returning
    /// a [`ScopedConfig`] guard that restores the previous
    /// configuration on drop.
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

