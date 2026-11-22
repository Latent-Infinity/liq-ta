//! Type-checked views of generic series as concrete `f32`/`f64` data.
//!
//! Indicators specialize their generic entry points on the concrete element
//! type (for example, to accumulate `f32` input in `f64`). These helpers are
//! the crate's single place where that specialization reinterprets memory:
//! each view exists only when [`TypeId`] proves the two types are identical.

use std::any::{Any, TypeId};

/// Views `data` as `&[U]` when `T` and `U` are the same type.
#[inline]
#[must_use]
pub(crate) fn slice_as<T: 'static, U: 'static>(data: &[T]) -> Option<&[U]> {
    if TypeId::of::<T>() != TypeId::of::<U>() {
        return None;
    }
    // SAFETY: equal `TypeId`s mean `T` and `U` are the same type, so the pointer,
    // length, alignment, initialization and borrow of `data` are valid for `[U]`.
    Some(unsafe { std::slice::from_raw_parts(data.as_ptr().cast::<U>(), data.len()) })
}

/// Views `data` as `&mut [U]` when `T` and `U` are the same type.
#[inline]
#[must_use]
pub(crate) fn slice_as_mut<T: 'static, U: 'static>(data: &mut [T]) -> Option<&mut [U]> {
    if TypeId::of::<T>() != TypeId::of::<U>() {
        return None;
    }
    // SAFETY: equal `TypeId`s mean `T` and `U` are the same type, so the pointer,
    // length, alignment and initialization of `data` are valid for `[U]`; the
    // exclusive borrow of `data` moves into the returned view.
    Some(unsafe { std::slice::from_raw_parts_mut(data.as_mut_ptr().cast::<U>(), data.len()) })
}

/// Converts `Vec<T>` into `Vec<U>` when `T` and `U` are the same type.
///
/// # Errors
///
/// Returns the original vector unchanged when the types differ.
#[inline]
pub(crate) fn vec_into<T: 'static, U: 'static>(
    data: Vec<T>,
) -> std::result::Result<Vec<U>, Vec<T>> {
    // Downcast through a stack slot so the conversion never allocates.
    let mut slot = Some(data);
    if let Some(converted) = (&mut slot as &mut dyn Any).downcast_mut::<Option<Vec<U>>>() {
        return Ok(converted.take().expect("slot holds the vector"));
    }
    Err(slot.expect("slot holds the vector"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn views_exist_only_for_identical_types() {
        let mut data = vec![1.0_f64, 2.0, 3.0];
        assert_eq!(slice_as::<f64, f64>(&data), Some(&[1.0, 2.0, 3.0][..]));
        assert!(slice_as::<f64, f32>(&data).is_none());
        assert!(slice_as_mut::<f64, f32>(&mut data).is_none());

        let view = slice_as_mut::<f64, f64>(&mut data).expect("same type");
        view[1] = 5.0;
        assert_eq!(data, [1.0, 5.0, 3.0]);
    }

    #[test]
    fn empty_slices_keep_their_length() {
        let mut empty: Vec<f32> = Vec::new();
        assert_eq!(slice_as::<f32, f32>(&empty).map(<[f32]>::len), Some(0));
        assert_eq!(
            slice_as_mut::<f32, f32>(&mut empty).map(|s| s.len()),
            Some(0)
        );
    }

    #[test]
    fn vec_into_converts_or_returns_the_original() {
        let same = vec_into::<f32, f32>(vec![1.5_f32, 2.5]).expect("same type");
        assert_eq!(same, [1.5, 2.5]);

        let original = vec_into::<f32, f64>(vec![1.5_f32]).expect_err("different type");
        assert_eq!(original, [1.5]);
    }
}
