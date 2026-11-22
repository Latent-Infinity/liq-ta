//! Buffer reuse, output initialization, and optional heap-allocation tests.

#![expect(
    clippy::needless_range_loop,
    reason = "index loops mirror the reference formulas across parallel arrays"
)]

use liq_ta::indicators::{
    bollinger::{BollingerOutput, bollinger_into},
    macd::macd_into,
    sma::sma_into,
    stochastic::{StochasticOutput, stochastic_fast_into},
};

// ==================== Buffer Reuse Tests ====================
// Verify that `_into` variants can be called multiple times with same buffer

#[test]
fn allocation_buffer_reuse_sma() {
    let data1 = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
    let data2 = vec![5.0_f64, 4.0, 3.0, 2.0, 1.0];
    let mut output = vec![0.0_f64; 5];

    // First computation
    sma_into(&data1, 3, &mut output).unwrap();
    assert!(!output[2].is_nan());
    let first_result = output[2];

    // Reuse buffer with different data
    sma_into(&data2, 3, &mut output).unwrap();
    let second_result = output[2];

    // Results should be different (buffer was correctly overwritten)
    assert!(
        (first_result - second_result).abs() > 1e-10,
        "Buffer reuse should produce different results for different inputs"
    );
}

#[test]
fn allocation_buffer_reuse_macd() {
    let data1: Vec<f64> = (0..40).map(|x| 100.0 + f64::from(x)).collect();
    let data2: Vec<f64> = (0..40).map(|x| 100.0 - f64::from(x) * 0.5).collect();

    let n = data1.len();
    let mut macd = vec![0.0_f64; n];
    let mut signal = vec![0.0_f64; n];
    let mut hist = vec![0.0_f64; n];

    // First computation
    macd_into(&data1, 12, 26, 9, &mut macd, &mut signal, &mut hist).unwrap();
    let first_macd = macd[35];

    // Reuse buffers
    macd_into(&data2, 12, 26, 9, &mut macd, &mut signal, &mut hist).unwrap();
    let second_macd = macd[35];

    // Results should be different
    assert!(
        (first_macd - second_macd).abs() > 0.01,
        "Buffer reuse should produce different results for different inputs"
    );
}

// ==================== Exact Buffer Size Tests ====================
// Verify that output buffers are filled correctly

#[test]
fn allocation_sma_fills_exactly() {
    let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0];
    let mut output = vec![f64::MAX; data.len()]; // Fill with sentinel

    sma_into(&data, 3, &mut output).unwrap();

    // First 2 values should be NaN (lookback)
    assert!(output[0].is_nan());
    assert!(output[1].is_nan());
    // Remaining should be valid (not sentinel)
    assert!(output[2] != f64::MAX && !output[2].is_nan());
    assert!(output[3] != f64::MAX && !output[3].is_nan());
    assert!(output[4] != f64::MAX && !output[4].is_nan());
}

#[test]
fn allocation_macd_fills_exactly() {
    let data: Vec<f64> = (0..40).map(|x| 100.0 + f64::from(x)).collect();
    let n = data.len();

    let mut macd = vec![f64::MAX; n];
    let mut signal = vec![f64::MAX; n];
    let mut hist = vec![f64::MAX; n];

    macd_into(&data, 12, 26, 9, &mut macd, &mut signal, &mut hist).unwrap();

    // First 25 values should be NaN (slow_period - 1)
    for i in 0..25 {
        assert!(
            macd[i].is_nan(),
            "macd[{i}] should be NaN in lookback period"
        );
    }
    // First valid MACD at index 25
    assert!(macd[25] != f64::MAX && !macd[25].is_nan());

    // First 33 signal values should be NaN (slow + signal - 2)
    for i in 0..33 {
        assert!(
            signal[i].is_nan(),
            "signal[{i}] should be NaN in lookback period"
        );
    }
    // First valid signal at index 33
    assert!(signal[33] != f64::MAX && !signal[33].is_nan());
}

// ==================== Struct-Held Output Tests ====================
// Output structs expose public `Vec`s, so `_into` could resize or replace them.

#[test]
fn allocation_bollinger_into_keeps_caller_buffers() {
    let data = vec![
        20.0_f64, 21.0, 22.0, 21.5, 22.5, 23.0, 22.0, 21.0, 20.5, 21.5,
    ];
    let n = data.len();
    let mut output = BollingerOutput {
        upper: vec![0.0_f64; n],
        middle: vec![0.0_f64; n],
        lower: vec![0.0_f64; n],
    };
    let ptrs = (
        output.upper.as_ptr(),
        output.middle.as_ptr(),
        output.lower.as_ptr(),
    );
    bollinger_into(&data, 3, 2.0, &mut output).unwrap();
    assert_eq!(
        (
            output.upper.as_ptr(),
            output.middle.as_ptr(),
            output.lower.as_ptr()
        ),
        ptrs
    );
    assert_eq!(
        (output.upper.len(), output.middle.len(), output.lower.len()),
        (n, n, n)
    );
}

#[test]
fn allocation_bollinger_f32_into_keeps_caller_buffers() {
    let data = vec![
        20.0_f32, 21.0, 22.0, 21.5, 22.5, 23.0, 22.0, 21.0, 20.5, 21.5,
    ];
    let n = data.len();
    let mut output = BollingerOutput {
        upper: vec![0.0_f32; n],
        middle: vec![0.0_f32; n],
        lower: vec![0.0_f32; n],
    };
    let ptrs = (
        output.upper.as_ptr(),
        output.middle.as_ptr(),
        output.lower.as_ptr(),
    );
    bollinger_into(&data, 3, 2.0_f32, &mut output).unwrap();
    assert_eq!(
        (
            output.upper.as_ptr(),
            output.middle.as_ptr(),
            output.lower.as_ptr()
        ),
        ptrs
    );
    assert_eq!(
        (output.upper.len(), output.middle.len(), output.lower.len()),
        (n, n, n)
    );
}

#[test]
fn allocation_stochastic_into_keeps_caller_buffers() {
    let n = 20;
    let high: Vec<f64> = (0..n).map(|x| 55.0 + f64::from(x)).collect();
    let low: Vec<f64> = (0..n).map(|x| 45.0 + f64::from(x)).collect();
    let close: Vec<f64> = (0..n).map(|x| 50.0 + f64::from(x)).collect();
    let mut output = StochasticOutput {
        k: vec![0.0_f64; 20],
        d: vec![0.0_f64; 20],
    };
    let ptrs = (output.k.as_ptr(), output.d.as_ptr());
    stochastic_fast_into(&high, &low, &close, 5, 3, &mut output).unwrap();
    assert_eq!((output.k.as_ptr(), output.d.as_ptr()), ptrs);
    assert_eq!((output.k.len(), output.d.len()), (20, 20));
}

// ==================== DHAT Heap Allocation Tests ====================
// These tests use dhat to verify zero heap allocations during computation.
//
// IMPORTANT: Run dhat tests single-threaded to avoid global allocator conflicts:
//   cargo test -p liq-ta --features dhat-heap --test allocation_tests dhat_tests -- --test-threads=1
//
// The tests verify zero heap allocations for the covered indicators and inputs
// after the initial buffer setup.

#[cfg(feature = "dhat-heap")]
mod dhat_tests {
    use super::*;
    use liq_ta::indicators::{
        atr::atr_into,
        bollinger::{BollingerOutput, bollinger_into},
        ema::ema_into,
        rsi::rsi_into,
    };

    #[global_allocator]
    static ALLOC: dhat::Alloc = dhat::Alloc;

    /// Helper to run a closure and return heap stats for allocations during execution.
    ///
    /// Note: This creates a fresh profiler for each test. Tests must run single-threaded
    /// (--test-threads=1) to avoid interference between concurrent profilers.
    fn measure_allocations<F: FnOnce()>(f: F) -> dhat::HeapStats {
        let _profiler = dhat::Profiler::builder().testing().build();
        f();
        dhat::HeapStats::get()
    }

    #[test]
    fn dhat_sma_into_zero_allocs() {
        // Pre-allocate all data outside measurement
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let mut output = vec![0.0_f64; data.len()];

        // Measure only the computation
        let stats = measure_allocations(|| {
            sma_into(&data, 3, &mut output).unwrap();
        });

        assert_eq!(
            stats.total_blocks, 0,
            "sma_into should perform zero heap allocations, but allocated {} blocks ({} bytes)",
            stats.total_blocks, stats.total_bytes
        );
    }

    #[test]
    fn dhat_ema_into_zero_allocs() {
        let data = vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let mut output = vec![0.0_f64; data.len()];

        let stats = measure_allocations(|| {
            ema_into(&data, 3, &mut output).unwrap();
        });

        assert_eq!(
            stats.total_blocks, 0,
            "ema_into should perform zero heap allocations, but allocated {} blocks ({} bytes)",
            stats.total_blocks, stats.total_bytes
        );
    }

    #[test]
    fn dhat_rsi_into_zero_allocs() {
        let data: Vec<f64> = (0..30).map(|x| 50.0 + f64::from(x) * 0.5).collect();
        let mut output = vec![0.0_f64; data.len()];

        let stats = measure_allocations(|| {
            rsi_into(&data, 14, &mut output).unwrap();
        });

        assert_eq!(
            stats.total_blocks, 0,
            "rsi_into should perform zero heap allocations, but allocated {} blocks ({} bytes)",
            stats.total_blocks, stats.total_bytes
        );
    }

    #[test]
    fn dhat_macd_into_zero_allocs() {
        let data: Vec<f64> = (0..50).map(|x| 100.0 + f64::from(x)).collect();
        let n = data.len();
        let mut macd_line = vec![0.0_f64; n];
        let mut signal_line = vec![0.0_f64; n];
        let mut histogram = vec![0.0_f64; n];

        let stats = measure_allocations(|| {
            macd_into(
                &data,
                12,
                26,
                9,
                &mut macd_line,
                &mut signal_line,
                &mut histogram,
            )
            .unwrap();
        });

        assert_eq!(
            stats.total_blocks, 0,
            "macd_into should perform zero heap allocations, but allocated {} blocks ({} bytes)",
            stats.total_blocks, stats.total_bytes
        );
    }

    #[test]
    fn dhat_bollinger_into_zero_allocs() {
        let data = vec![
            20.0_f64, 21.0, 22.0, 21.5, 22.5, 23.0, 22.0, 21.0, 20.5, 21.5,
        ];
        let n = data.len();
        let mut output = BollingerOutput {
            upper: vec![0.0_f64; n],
            middle: vec![0.0_f64; n],
            lower: vec![0.0_f64; n],
        };

        let stats = measure_allocations(|| {
            bollinger_into(&data, 3, 2.0, &mut output).unwrap();
        });

        assert_eq!(
            stats.total_blocks, 0,
            "bollinger_into should perform zero heap allocations, but allocated {} blocks ({} bytes)",
            stats.total_blocks, stats.total_bytes
        );
    }

    #[test]
    fn dhat_atr_into_zero_allocs() {
        let n = 30;
        let high: Vec<f64> = (0..n).map(|x| 55.0 + (x as f64)).collect();
        let low: Vec<f64> = (0..n).map(|x| 45.0 + (x as f64)).collect();
        let close: Vec<f64> = (0..n).map(|x| 50.0 + (x as f64)).collect();
        let mut output = vec![0.0_f64; n];

        let stats = measure_allocations(|| {
            atr_into(&high, &low, &close, 14, &mut output).unwrap();
        });

        assert_eq!(
            stats.total_blocks, 0,
            "atr_into should perform zero heap allocations, but allocated {} blocks ({} bytes)",
            stats.total_blocks, stats.total_bytes
        );
    }

    #[test]
    fn dhat_large_sma_zero_allocs() {
        // Test with large data to ensure no hidden allocations at scale
        let n = 100_000;
        let data: Vec<f64> = (0..n).map(|x| 100.0 + (x as f64) * 0.01).collect();
        let mut output = vec![0.0_f64; n];

        let stats = measure_allocations(|| {
            sma_into(&data, 200, &mut output).unwrap();
        });

        assert_eq!(
            stats.total_blocks, 0,
            "sma_into with 100k elements should perform zero heap allocations, but allocated {} blocks ({} bytes)",
            stats.total_blocks, stats.total_bytes
        );
    }

    #[test]
    fn dhat_repeated_calls_zero_allocs() {
        // Verify buffer reuse doesn't allocate
        let data1: Vec<f64> = (0..20).map(f64::from).collect();
        let data2: Vec<f64> = (0..20).map(|x| f64::from(x * 2)).collect();
        let mut output = vec![0.0_f64; 20];

        let stats = measure_allocations(|| {
            // Multiple calls reusing same buffer
            sma_into(&data1, 5, &mut output).unwrap();
            sma_into(&data2, 5, &mut output).unwrap();
            ema_into(&data1, 5, &mut output).unwrap();
            ema_into(&data2, 5, &mut output).unwrap();
        });

        assert_eq!(
            stats.total_blocks, 0,
            "Repeated _into calls should perform zero heap allocations, but allocated {} blocks ({} bytes)",
            stats.total_blocks, stats.total_bytes
        );
    }
}
