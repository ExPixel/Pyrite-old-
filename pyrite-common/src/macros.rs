///! Some macros that I didn't really have a place for in the source code.

#[cfg(not(feature = "nightly"))]
#[macro_export]
macro_rules! likely {
    ($b:expr) => {
        $b
    };
}

#[cfg(not(feature = "nightly"))]
#[macro_export]
macro_rules! unlikely {
    ($b:expr) => {
        $b
    };
}

#[cfg(feature = "nightly")]
#[macro_export]
macro_rules! unlikely {
    ($b:expr) => {
        unsafe { std::intrinsics::unlikely($b) }
    };
}

#[cfg(feature = "nightly")]
#[macro_export]
macro_rules! likely {
    ($b:expr) => {
        unsafe { std::intrinsics::likely($b) }
    };
}

#[macro_export]
macro_rules! bits {
    ($value:expr, $start:expr) => {
        ($value >> $start) & 1
    };

    ($value:expr, $start:expr, $end:expr) => {
        ($value >> $start) & ((1 << ($end - $start + 1)) - 1)
    };
}

#[macro_export]
macro_rules! bits_b {
    ($value:expr, $start:expr) => {
        (($value >> $start) & 1) != 0
    };

    ($value:expr, $start:expr, $end:expr) => {
        (($value >> $start) & ((1 << ($end - $start + 1)) - 1)) != 0
    };
}

#[macro_export]
macro_rules! bits_set {
    ($dest:expr, $src:expr, $start:expr, $end:expr) => {
        ($dest & !(((1 << ($end - $start + 1)) - 1) << $start))
            | (($src & ((1 << ($end - $start + 1)) - 1)) << $start)
    };

    ($dest:expr, $src:expr, $start:expr) => {
        bits_set!($dest, $src, $start, $start)
    };
}

#[macro_export]
macro_rules! bit_set {
    ($dest:expr, $src:expr, $start:expr) => {
        bits_set!($dest, $src, $start, $start)
    };
}

#[macro_export]
macro_rules! bit_set_b {
    ($dest:expr, $src:expr, $start:expr) => {
        bits_set!($dest, if $src { 1 } else { 0 }, $start, $start)
    };
}

/// #TODO maybe I should use wrapping_shl and wrapping_shr for this
///       but I'm not sure. The high bits that are going to be replaced
///       by the sign bit SHOULD be empty so maybe allowing this to panic is good.
/// Sign extends a value of ${current_bits} to 32bits
#[macro_export]
macro_rules! sign_extend_32 {
    ($value:expr, $current_bits:expr) => {
        ((($value << (32 - $current_bits)) as i32) >> (32 - $current_bits)) as u32
    };
}

// /// negates an unsigned 32bit number
// macro_rules! u32_neg {
//     ($value:expr) => {
//         ($value as i32 * -1i32) as i32
//     };
// }

#[macro_export]
macro_rules! debug_print {
    ($FirstExpr:expr $(,$Expr:expr)* $(,)*) => {
        println!("[{}:{}] {} = {}", file!(), line!(), stringify!($FirstExpr), $FirstExpr);
        $(
            println!("[{}:{}] {} = {}", file!(), line!(), stringify!($Expr), $Expr);
        )*
    }
}

#[macro_export]
macro_rules! gba_error {
    ($Args:tt) => {
        log::error!($Args)
    };
}
