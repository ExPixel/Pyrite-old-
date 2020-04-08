#[allow(clippy::suspicious_arithmetic_impl)]
#[allow(clippy::clippy::suspicious_op_assign_impl)]
pub mod fixedpoint;
pub mod memory;
#[macro_use]
pub mod bitfields;

/// Constant boolean type used for compile time ifs.
/// FIXME change this when const generics are stable.
pub trait CBool {
    const BOOL: bool;
}
