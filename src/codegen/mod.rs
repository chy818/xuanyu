/**
 * @file mod.rs
 * @brief CCAS 代码生成模块
 */

pub mod codegen;
pub mod optimize;

pub use codegen::{CodeGenerator, generate_ir, generate_ir_with_module_name};
pub use optimize::{IROptimizer, OptimizationConfig, FunctionInliner};
