use std::{hint::black_box, time::Instant};
use liq_ta::kernels::simd::{sum_f64, dot_product_f64, sum_and_sum_sq_and_count_f64};
fn main() {
 let input: Vec<f64> = (0..131_072).map(|n| f64::from(n % 101) / 7.0).collect();
 for name in ["sum", "dot", "moments"] {
  for _ in 0..3 { let start=Instant::now(); let mut result=0.0;
   for _ in 0..3000 { result += match name { "sum"=>sum_f64(black_box(&input)), "dot"=>dot_product_f64(black_box(&input),black_box(&input)), _=> { let (sum, squares, count) = sum_and_sum_sq_and_count_f64(black_box(&input)); black_box(count); black_box(sum) + squares } }; }
   println!("{name} iterations=3000 length={} elapsed_ns={} checksum={}", input.len(), start.elapsed().as_nanos(), black_box(result));
  }
 }
}
