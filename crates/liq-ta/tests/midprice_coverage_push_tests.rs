use liq_ta::indicators::midprice::{midprice, midprice_into, midprice_lookback};

fn make_high_low(n: usize) -> (Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);

    let mut base = 100.0_f64;
    for i in 0..n {
        base += if i % 5 == 0 { 0.45 } else { -0.12 } + (i as f64 * 0.01).sin() * 0.2;
        let l = base - 1.0 - ((i % 7) as f64) * 0.01;
        let h = base + 0.9 + ((i % 9) as f64) * 0.01;
        high.push(h.max(l + 0.001));
        low.push(l);
    }

    (high, low)
}

#[test]
fn midprice_large_input_van_herk_alloc_and_into_parity() {
    let (high, low) = make_high_low(1200);
    let period = 55;
    let lookback = midprice_lookback(period);

    let alloc = midprice(&high, &low, period).expect("midprice alloc should succeed");

    let mut into = vec![0.0_f64; high.len()];
    midprice_into(&high, &low, period, &mut into).expect("midprice_into should succeed");

    for i in 0..lookback {
        assert!(alloc[i].is_nan());
        assert!(into[i].is_nan());
    }

    for i in lookback..high.len() {
        if alloc[i].is_nan() || into[i].is_nan() {
            assert!(alloc[i].is_nan() && into[i].is_nan());
        } else {
            assert!((alloc[i] - into[i]).abs() < 1e-10, "mismatch at {i}");
        }
    }
}

#[test]
fn midprice_large_input_van_herk_nan_window_propagation() {
    let (mut high, low) = make_high_low(1200);
    let period = 50;
    let lookback = midprice_lookback(period);
    let nan_idx = 700;
    high[nan_idx] = f64::NAN;

    let out = midprice(&high, &low, period).expect("midprice should succeed with NaN propagation");

    assert!(out[lookback].is_finite());
    assert!(out[nan_idx].is_nan());
    assert!(out[nan_idx + lookback].is_nan());
    assert!(out[nan_idx + lookback + 1].is_finite());
}

#[test]
fn midprice_period_one_non_finite_branch() {
    let high = [10.0_f64, f64::INFINITY, 12.0, 13.0];
    let low = [9.0_f64, 8.0, f64::NAN, 12.0];

    let out = midprice(&high, &low, 1).expect("period 1 should succeed");
    assert!((out[0] - 9.5).abs() < 1e-12);
    assert!(out[1].is_nan());
    assert!(out[2].is_nan());
    assert!((out[3] - 12.5).abs() < 1e-12);
}
