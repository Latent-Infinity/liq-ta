use liq_ta::Error;
use liq_ta::indicators::{gaussian_channel, gaussian_filter, supertrend};
use proptest::prelude::*;

fn is_length_mismatch(err: &Error) -> bool {
    matches!(err, Error::LengthMismatch { .. })
}

proptest! {
    #[test]
    fn prop_gaussian_variants_reject_non_positive_sigma(
        data in prop::collection::vec(-10_000.0f64..10_000.0f64, 1..128),
        period in 1usize..64,
        sigma in prop_oneof![Just(0.0f64), -100.0f64..0.0f64],
    ) {
        let bounded_period = period.min(data.len());

        let filter_err = gaussian_filter(&data, bounded_period, sigma).unwrap_err();
        prop_assert!(is_length_mismatch(&filter_err));

        let channel_err = gaussian_channel(&data, bounded_period, sigma, 2.0).unwrap_err();
        prop_assert!(is_length_mismatch(&channel_err));
    }
}

proptest! {
    #[test]
    fn prop_supertrend_rejects_non_positive_multiplier(
        high in prop::collection::vec(10.0f64..500.0f64, 8..128),
        period in 1usize..20,
        multiplier in prop_oneof![Just(0.0f64), -20.0f64..0.0f64],
    ) {
        let n = high.len();
        let low: Vec<f64> = high.iter().map(|v| v - 0.5).collect();
        let close: Vec<f64> = high.iter().map(|v| v - 0.25).collect();

        let bounded_period = period.min(n);
        let err = supertrend(&high, &low, &close, bounded_period, multiplier).unwrap_err();
        prop_assert!(is_length_mismatch(&err));
    }
}

#[test]
fn supertrend_mismatched_length_and_empty_input_errors_are_actionable() {
    let high = vec![10.0, 11.0, 12.0, 13.0];
    let low = vec![9.0, 10.0, 11.0];
    let close = vec![9.5, 10.5, 11.5, 12.5];

    let mismatch = supertrend(&high, &low, &close, 2, 3.0).unwrap_err();
    assert!(matches!(mismatch, Error::LengthMismatch { .. }));

    let empty: Vec<f64> = Vec::new();
    let empty_err = gaussian_filter(&empty, 2, 0.5).unwrap_err();
    assert!(matches!(empty_err, Error::EmptyInput));
}

#[test]
fn gaussian_channel_boundary_min_len_matrix() {
    for period in [1usize, 2, 5, 20] {
        let too_short = vec![100.0; period.saturating_sub(1)];
        if too_short.is_empty() {
            continue;
        }

        let err = gaussian_channel(&too_short, period, 0.5, 2.0).unwrap_err();
        assert!(matches!(
            err,
            Error::InsufficientData {
                required,
                actual,
                indicator: "gaussian_channel"
            } if required == period && actual == period - 1
        ));
    }
}
