#[macro_use]
pub mod jit;
pub use jit::JITCompiler;


use dynasmrt::dynasm;
