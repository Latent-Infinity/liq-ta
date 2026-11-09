use liq_ta::kernels::simd::true_range_f64;
use std::{fs, io::{self, Write}};
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    let bytes = fs::read(&args[1])?;
    let mut high=Vec::new(); let mut low=Vec::new(); let mut close=Vec::new();
    for row in bytes.chunks_exact(24) {
        let value = |i| f64::from_le_bytes(row[i..i+8].try_into().expect("exact row"));
        high.push(value(0)); low.push(value(8)); close.push(value(16));
    }
    assert_eq!(high.len(),449);
    let mut out=vec![f64::NAN; high.len()];
    true_range_f64(&high,&low,&close,&mut out);
    let mut file=fs::File::create(&args[2])?;
    for x in &out[1..] {file.write_all(&x.to_le_bytes())?;}
    file.flush()
}
