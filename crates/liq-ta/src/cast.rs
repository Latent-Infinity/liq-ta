//! Element-type specialization helpers.
//!
//! Indicators pick a fast path by element type (for example, accumulating
//! `f32` input in `f64`). The fast kernels are generic and convert through
//! these helpers, so instantiating them for `f64` or `f32` computes exactly
//! what concrete `f64`/`f32` kernels would, without reinterpreting memory.

use std::any::TypeId;

use num_traits::NumCast;

use crate::traits::SeriesElement;

/// Returns whether `T` and `U` are the same type.
#[inline]
#[must_use]
pub(crate) fn same_type<T: 'static, U: 'static>() -> bool {
    TypeId::of::<T>() == TypeId::of::<U>()
}

/// Converts an element to `f64` exactly as `as f64` would.
///
/// For `f64` this is the identity and for `f32` a widening cast.
#[inline]
#[must_use]
pub(crate) fn to_f64<T: SeriesElement>(value: T) -> f64 {
    value
        .to_f64()
        .expect("float-to-float conversion in num-traits never fails")
}

/// Converts an `f64` to the element type exactly as `as T` would (saturating to infinity).
#[inline]
#[must_use]
pub(crate) fn from_f64<T: SeriesElement>(value: f64) -> T {
    <T as NumCast>::from(value).expect("float-to-float conversion in num-traits never fails")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_type_distinguishes_float_types() {
        assert!(same_type::<f64, f64>());
        assert!(!same_type::<f64, f32>());
    }

    #[test]
    fn conversions_match_as_casts() {
        assert_eq!(to_f64(0.1_f32), 0.1_f32 as f64);
        assert_eq!(from_f64::<f32>(0.1), 0.1_f64 as f32);
        assert_eq!(from_f64::<f32>(1e300), f32::INFINITY);
        assert!(from_f64::<f32>(f64::NAN).is_nan());
        assert_eq!(from_f64::<f64>(-0.0).to_bits(), (-0.0_f64).to_bits());
    }
}
