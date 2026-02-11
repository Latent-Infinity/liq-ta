# Contributing to liq-ta Python Bindings

This document describes the design decisions and implementation patterns used in the liq-ta Python bindings.

## Zero-Copy Design

liq-ta is designed for high-performance technical analysis with minimal memory overhead. The Python bindings use PyO3 and rust-numpy to achieve zero-copy data transfer between Python and Rust.

### Input: Zero-Copy from NumPy

Input arrays are passed by reference using `PyReadonlyArray1`:

```rust
fn sma<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,  // Zero-copy borrow
    period: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>>
```

**Key points:**
- `PyReadonlyArray1` provides a read-only view into the NumPy array's memory
- `as_slice()` returns a `&[f64]` directly pointing to NumPy's buffer
- No data is copied during input

**Requirement:** Input arrays must be C-contiguous. Non-contiguous arrays (e.g., strided slices) will raise a `TypeError`. Users can call `np.ascontiguousarray()` to fix this.

### Output: NumPy-Style `out=` Parameter

Following NumPy conventions, all single-output functions support an optional `out=` parameter:

```rust
#[pyfunction]
#[pyo3(signature = (data, period, out=None))]
fn sma<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,  // Optional pre-allocated output
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    match out {
        Some(output) => {
            // SAFETY: We have exclusive access during this function call
            let slice = unsafe { output.as_slice_mut()? };
            liq_ta::indicators::sma_into(input, period, slice).map_err(to_py_err)?;
            Ok(output)  // Return same array
        }
        None => {
            let result = liq_ta::indicators::sma(input, period).map_err(to_py_err)?;
            Ok(PyArray1::from_vec(py, result))  // Allocate new array
        }
    }
}
```

**Usage patterns:**

```python
# Allocating (convenient for one-off calculations)
result = liq_ta.sma(prices, 20)

# Zero-copy (for performance-critical code)
out = np.empty(len(prices), dtype=np.float64)
liq_ta.sma(prices, 20, out=out)

# Chained calculations with reused buffer
buffer = np.empty(len(prices), dtype=np.float64)
liq_ta.sma(prices, 10, out=buffer)
# ... use buffer ...
liq_ta.ema(prices, 10, out=buffer)  # Reuse same memory
```

### Why Not PyReadwriteArray?

We initially attempted to use `PyReadwriteArray1` for type-safe mutable access:

```rust
// Attempted but didn't work well
fn sma<'py>(
    out: Option<PyReadwriteArray1<'py, f64>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    // Problem: Hard to convert PyReadwriteArray back to Bound<PyArray>
}
```

The issue is that `PyReadwriteArray` is a borrow-checking wrapper that doesn't easily convert back to the underlying `Bound<PyArray>` needed for the return type.

Instead, we use `Bound<'py, PyArray1<f64>>` directly with unsafe mutable access:

```rust
Some(output) => {
    let slice = unsafe { output.as_slice_mut()? };
    // ...
    Ok(output)
}
```

This is safe because:
1. The caller passes ownership of the output array reference
2. No other references exist during the function call
3. We immediately return the same array

### Multi-Output Functions

Functions returning multiple arrays (MACD, Bollinger, Stochastic, ADX, Donchian) allocate new arrays for each output. The `out=` pattern doesn't apply to these because:

1. Multiple pre-allocated arrays would be awkward: `out1=, out2=, out3=`
2. These are typically called less frequently than single-output functions
3. The tuple return is more ergonomic for destructuring

### Performance Characteristics

| Operation | Copy | Notes |
|-----------|------|-------|
| Input (contiguous) | None | Direct pointer access |
| Input (strided) | N/A | Raises TypeError |
| Output (no `out=`) | 1 copy | Rust Vec copied to NumPy |
| Output (with `out=`) | None | Direct write to NumPy buffer |

### Polars Integration

Polars DataFrames use Apache Arrow memory layout, which is already contiguous. Converting to NumPy is essentially free:

```python
import polars as pl
import liq_ta

df = pl.DataFrame({'price': [1.0, 2.0, 3.0, ...]})

# .to_numpy() is ~zero-copy due to Arrow backing
sma = liq_ta.sma(df['price'].to_numpy(), 20)
```

## Adding New Indicators

When adding a new indicator to the Python bindings:

1. **Single output indicators** should support `out=` parameter
2. **Multi-output indicators** should return a tuple
3. **All indicators** must validate input is C-contiguous (handled by `as_slice()`)

Template for single-output:

```rust
#[pyfunction]
#[pyo3(signature = (data, period, out=None))]
fn new_indicator<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'py, f64>,
    period: usize,
    out: Option<Bound<'py, PyArray1<f64>>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input = data.as_slice()?;

    match out {
        Some(output) => {
            let slice = unsafe { output.as_slice_mut()? };
            liq_ta::indicators::new_indicator_into(input, period, slice)
                .map_err(to_py_err)?;
            Ok(output)
        }
        None => {
            let result = liq_ta::indicators::new_indicator(input, period)
                .map_err(to_py_err)?;
            Ok(PyArray1::from_vec(py, result))
        }
    }
}
```

## Dependencies

- **PyO3 0.23**: Rust-Python bindings
- **numpy 0.23 (rust-numpy)**: NumPy array support
- **maturin**: Build tool for Python packages from Rust

## Testing

Tests validate both correctness and zero-copy behavior:

```python
def test_sma_out_zero_copy_output(self):
    """sma with out= should write to pre-allocated array without copying."""
    data = np.array([1.0, 2.0, 3.0, 4.0, 5.0])
    out = np.empty(len(data), dtype=np.float64)
    original_ptr = out.ctypes.data

    result = liq_ta.sma(data, 3, out=out)

    # Pointer unchanged = same memory
    assert out.ctypes.data == original_ptr
    # Returns same array
    assert result is out
```
