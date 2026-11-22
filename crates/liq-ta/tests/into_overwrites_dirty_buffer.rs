//! Contract: every public `*_into` writes every output index in `[0, n)`, so a
//! reused (dirty) buffer yields exactly what the allocating function returns.

use liq_ta::SeriesElement;
use liq_ta::indicators::bollinger::{Bollinger, BollingerOutput};
use liq_ta::indicators::candlestick as cdl;
use liq_ta::indicators::ichimoku::IchimokuOutput;
use liq_ta::indicators::macd::Macd;
use liq_ta::indicators::sarext::SarExtParams;
use liq_ta::indicators::stochastic::{Stochastic, StochasticOutput};
use liq_ta::indicators::*;
use liq_ta::kernels::rolling_extrema as rx;
use std::collections::BTreeSet;

const FILLS: [f64; 2] = [0.0, -999.0];
const SIZES: [usize; 2] = [50, 1500];
// 3 / 14 / 30 straddle aroon's `< 8` and `< 21` dispatch thresholds.
const PERIODS: [usize; 3] = [3, 14, 30];

trait Val: Copy + std::fmt::Debug {
    fn of(x: f64) -> Self;
    fn same(self, o: Self) -> bool;
}
impl Val for f64 {
    fn of(x: f64) -> Self {
        x
    }
    fn same(self, o: Self) -> bool {
        self.to_bits() == o.to_bits() || (self.is_nan() && o.is_nan())
    }
}
impl Val for f32 {
    fn of(x: f64) -> Self {
        x as f32
    }
    fn same(self, o: Self) -> bool {
        self.to_bits() == o.to_bits() || (self.is_nan() && o.is_nan())
    }
}
impl Val for i32 {
    fn of(x: f64) -> Self {
        x as i32
    }
    fn same(self, o: Self) -> bool {
        self == o
    }
}

fn filled<V: Val>(like: &[V], fill: f64) -> Vec<V> {
    vec![V::of(fill); like.len()]
}

struct Log {
    fails: Vec<String>,
    seen: BTreeSet<&'static str>,
    ran: BTreeSet<&'static str>,
}

impl Log {
    fn check<V: Val>(&mut self, ctx: &str, name: &str, out: &str, want: &[V], got: &[V]) {
        if let Some(i) = (0..want.len()).find(|&i| !want[i].same(got[i])) {
            self.fails.push(format!(
                "{name}.{out} [{ctx}] idx {i}: want {:?} got {:?}",
                want[i], got[i]
            ));
        }
    }
}

struct Data<T> {
    o: Vec<T>,
    h: Vec<T>,
    l: Vec<T>,
    c: Vec<T>,
    v: Vec<T>,
}

fn data<T: Val>(n: usize, bad: bool) -> Data<T> {
    let mut cols: [Vec<f64>; 5] = Default::default();
    for i in 0..n {
        let x = |j: usize| {
            let t = j as f64;
            100.0
                + 10.0 * (t * 0.07).sin()
                + 3.0 * (t * 0.31).sin()
                + ((j * 7919) % 13) as f64 * 0.2
        };
        let (c, o) = (x(i), x(i + 1000) - 0.4);
        cols[0].push(o);
        cols[1].push(c.max(o) + 0.5 + (i % 5) as f64 * 0.1);
        cols[2].push(c.min(o) - 0.5 - (i % 3) as f64 * 0.1);
        cols[3].push(c);
        cols[4].push(1000.0 + ((i * 37) % 101) as f64 * 10.0);
    }
    if bad {
        for col in &mut cols {
            col[n / 2] = f64::NAN;
            col[n / 2 + 7] = f64::INFINITY;
        }
    }
    let [o, h, l, c, v] = cols.map(|col| col.into_iter().map(T::of).collect());
    Data { o, h, l, c, v }
}

fn run<T: SeriesElement + Val>(d: &Data<T>, p: usize, ctx: &str, log: &mut Log) {
    let (o, h, l, c, v) = (&d.o[..], &d.h[..], &d.l[..], &d.c[..], &d.v[..]);
    let t = |x: f64| T::of(x);

    // Single output: `|buf| into_call`.
    macro_rules! one {
        ($name:expr, $alloc:expr, |$b:ident| $into:expr) => {{
            log.seen.insert($name);
            if let Ok(want) = $alloc {
                for fill in FILLS {
                    let mut $b = filled(&want, fill);
                    match $into {
                        Ok(_) => {
                            log.ran.insert($name);
                            log.check(ctx, $name, "out", &want, &$b);
                        }
                        Err(e) => log.fails.push(format!("{} [{ctx}]: into err {e:?}", $name)),
                    }
                }
            }
        }};
    }
    // Multiple slices: `|buf = field, ...| into_call`.
    macro_rules! many {
        ($name:expr, $alloc:expr, |$($b:ident = $f:ident),+| $into:expr) => {{
            log.seen.insert($name);
            if let Ok(w) = $alloc {
                for fill in FILLS {
                    $(let mut $b = filled(&w.$f, fill);)+
                    match $into {
                        Ok(_) => {
                            log.ran.insert($name);
                            $(log.check(ctx, $name, stringify!($f), &w.$f, &$b);)+
                        }
                        Err(e) => log.fails.push(format!("{} [{ctx}]: into err {e:?}", $name)),
                    }
                }
            }
        }};
    }
    // Output struct: `Type { fields } |buf| into_call`.
    macro_rules! st {
        ($name:expr, $alloc:expr, $ty:ident { $($f:ident),+ }, |$b:ident| $into:expr) => {{
            log.seen.insert($name);
            if let Ok(w) = $alloc {
                for fill in FILLS {
                    let mut $b = $ty { $($f: filled(&w.$f, fill)),+ };
                    match $into {
                        Ok(_) => {
                            log.ran.insert($name);
                            $(log.check(ctx, $name, stringify!($f), &w.$f, &$b.$f);)+
                        }
                        Err(e) => log.fails.push(format!("{} [{ctx}]: into err {e:?}", $name)),
                    }
                }
            }
        }};
    }
    macro_rules! cdl {
        ($($f:ident $fi:ident),+ $(,)?) => {
            $(one!(stringify!($fi), cdl::$f(o, h, l, c), |b| cdl::$fi(o, h, l, c, &mut b));)+
        };
    }

    one!("ad_into", ad::ad(h, l, c, v), |b| ad::ad_into(
        h, l, c, v, &mut b
    ));
    one!("adosc_into", adosc::adosc(h, l, c, v, p, 2 * p), |b| {
        adosc::adosc_into(h, l, c, v, p, 2 * p, &mut b)
    });
    many!("adx_into", adx(h, l, c, p), |a = adx, pd = plus_di, md = minus_di| adx_into(h, l, c, p, &mut a, &mut pd, &mut md));
    one!("ao_into", ao(h, l), |b| ao_into(h, l, &mut b));
    one!("apo_into", apo(c, p, 2 * p), |b| apo_into(
        c,
        p,
        2 * p,
        &mut b
    ));
    one!("ppo_into", ppo(c, p, 2 * p), |b| ppo_into(
        c,
        p,
        2 * p,
        &mut b
    ));
    many!("aroon_into", aroon::aroon(h, l, p), |u = aroon_up, dn = aroon_down| aroon::aroon_into(h, l, p, &mut u, &mut dn));
    one!("aroonosc_into", aroon::aroonosc(h, l, p), |b| {
        aroon::aroonosc_into(h, l, p, &mut b)
    });
    one!("true_range_into", true_range(h, l, c), |b| true_range_into(
        h, l, c, &mut b
    ));
    one!("atr_into", atr(h, l, c, p), |b| atr_into(
        h, l, c, p, &mut b
    ));
    one!("autocorr_into", autocorr::autocorr(c, p, 1), |b| {
        autocorr::autocorr_into(c, p, 1, &mut b)
    });
    st!(
        "bollinger_into",
        bollinger(c, p, t(2.0)),
        BollingerOutput {
            middle,
            upper,
            lower
        },
        |b| bollinger_into(c, p, t(2.0), &mut b)
    );
    let bb = Bollinger::new().with_period(p);
    st!(
        "Bollinger::compute_into",
        bb.compute(c),
        BollingerOutput {
            middle,
            upper,
            lower
        },
        |b| bb.compute_into(c, &mut b)
    );
    one!("rolling_stddev_into", rolling_stddev(c, p), |b| {
        rolling_stddev_into(c, p, &mut b)
    });
    one!("bop_into", bop::bop(o, h, l, c), |b| bop::bop_into(
        o, h, l, c, &mut b
    ));
    one!("cci_into", cci::cci(h, l, c, p), |b| cci::cci_into(
        h, l, c, p, &mut b
    ));
    one!("chop_into", chop::chop(h, l, c, p), |b| chop::chop_into(
        h, l, c, p, &mut b
    ));
    one!("cmo_into", cmo::cmo(c, p), |b| cmo::cmo_into(c, p, &mut b));
    many!("hma_atr_bands_into", composite_bands::hma_atr_bands(h, l, c, p, p, 2.0), |u = upper, m = middle, lo = lower| composite_bands::hma_atr_bands_into(h, l, c, p, p, 2.0, &mut u, &mut m, &mut lo));
    many!("hma_bollinger_bands_into", composite_bands::hma_bollinger_bands(c, p, p, 2.0), |u = upper, m = middle, lo = lower| composite_bands::hma_bollinger_bands_into(c, p, p, 2.0, &mut u, &mut m, &mut lo));
    many!("vwap_atr_bands_into", composite_bands::vwap_atr_bands(h, l, c, v, p, 2.0), |u = upper, m = middle, lo = lower| composite_bands::vwap_atr_bands_into(h, l, c, v, p, 2.0, &mut u, &mut m, &mut lo));
    many!("vwap_bollinger_bands_into", composite_bands::vwap_bollinger_bands(h, l, c, v, p, 2.0), |u = upper, m = middle, lo = lower| composite_bands::vwap_bollinger_bands_into(h, l, c, v, p, 2.0, &mut u, &mut m, &mut lo));
    one!("connors_rsi_into", connors_rsi(c, p, 2, p), |b| {
        connors_rsi_into(c, p, 2, p, &mut b)
    });
    one!("dema_into", dema(c, p), |b| dema_into(c, p, &mut b));
    one!("demarker_into", demarker(h, l, p), |b| demarker_into(
        h, l, p, &mut b
    ));
    many!("donchian_into", donchian(h, l, p), |u = upper, m = middle, lo = lower| donchian_into(h, l, p, &mut u, &mut m, &mut lo));
    one!("dpo_into", dpo(c, p), |b| dpo_into(c, p, &mut b));
    one!(
        "dss_bressert_into",
        dss_bressert::dss_bressert(h, l, c, p, 3),
        |b| dss_bressert::dss_bressert_into(h, l, c, p, 3, &mut b)
    );
    one!("adxr_into", dx::adxr(h, l, c, p), |b| dx::adxr_into(
        h, l, c, p, &mut b
    ));
    one!("dx_into", dx::dx(h, l, c, p), |b| dx::dx_into(
        h, l, c, p, &mut b
    ));
    one!("plus_dm_into", dx::plus_dm(h, l, p), |b| dx::plus_dm_into(
        h, l, p, &mut b
    ));
    one!("minus_dm_into", dx::minus_dm(h, l, p), |b| {
        dx::minus_dm_into(h, l, p, &mut b)
    });
    one!("ema_into", ema(c, p), |b| ema_into(c, p, &mut b));
    one!("ema_wilder_into", ema_wilder(c, p), |b| ema_wilder_into(
        c, p, &mut b
    ));
    one!("ema_with_alpha_into", ema_with_alpha(c, p, t(0.3)), |b| {
        ema_with_alpha_into(c, p, t(0.3), &mut b)
    });
    many!("gaussian_channel_into", gaussian_channel(c, p, 2.0, 2.0), |ce = center, u = upper, lo = lower, tr = trend| gaussian_channel_into(c, p, 2.0, 2.0, &mut ce, &mut u, &mut lo, &mut tr));
    one!("gaussian_filter_into", gaussian_filter(c, p, 2.0), |b| {
        gaussian_filter_into(c, p, 2.0, &mut b)
    });
    one!("hma_into", hma(c, p), |b| hma_into(c, p, &mut b));
    one!("ht_dcperiod_into", ht_dcperiod::ht_dcperiod(c), |b| {
        ht_dcperiod::ht_dcperiod_into(c, &mut b)
    });
    one!("ht_dcphase_into", ht_dcphase::ht_dcphase(c), |b| {
        ht_dcphase::ht_dcphase_into(c, &mut b)
    });
    many!("ht_phasor_into", ht_phasor::ht_phasor(c), |i = inphase, q = quadrature| ht_phasor::ht_phasor_into(c, &mut i, &mut q));
    many!("ht_sine_into", ht_sine::ht_sine(c), |s = sine, ls = lead_sine| ht_sine::ht_sine_into(c, &mut s, &mut ls));
    one!("ht_trendline_into", ht_trendline(c), |b| ht_trendline_into(
        c, &mut b
    ));
    one!("ht_trendmode_into", ht_trendmode::ht_trendmode(c), |b| {
        ht_trendmode::ht_trendmode_into(c, &mut b)
    });
    one!("hurst_into", hurst::hurst(c, p), |b| hurst::hurst_into(
        c, p, &mut b
    ));
    st!(
        "ichimoku_into",
        ichimoku(h, l, c, p, 2 * p, 4 * p, p),
        IchimokuOutput {
            tenkan,
            kijun,
            senkou_a,
            senkou_b,
            chikou
        },
        |b| ichimoku_into(h, l, c, p, 2 * p, 4 * p, p, &mut b)
    );
    one!("kama_into", kama(c, p), |b| kama_into(c, p, &mut b));
    one!(
        "kama_full_into",
        kama_full(c, p, 2, 30),
        |b| kama_full_into(c, p, 2, 30, &mut b)
    );
    many!("keltner_channel_into", keltner_channel(h, l, c, p, 2.0), |u = upper, m = middle, lo = lower| keltner_channel_into(h, l, c, p, 2.0, &mut u, &mut m, &mut lo));
    one!("laguerre_rsi_into", laguerre_rsi(c, 0.5), |b| {
        laguerre_rsi_into(c, 0.5, &mut b)
    });
    many!("macd_into", macd(c, p, 2 * p, p), |m = macd_line, s = signal_line, hi = histogram| macd_into(c, p, 2 * p, p, &mut m, &mut s, &mut hi));
    let mc = Macd::new()
        .with_fast_period(p)
        .with_slow_period(2 * p)
        .with_signal_period(p);
    many!("Macd::compute_into", mc.compute(c), |m = macd_line, s = signal_line, hi = histogram| mc.compute_into(c, &mut m, &mut s, &mut hi));
    many!("mama_into", mama(c), |m = mama, f = fama| mama_into(c, &mut m, &mut f));
    many!("mama_full_into", mama_full(c, t(0.5), t(0.05)), |m = mama, f = fama| mama_full_into(c, t(0.5), t(0.05), &mut m, &mut f));
    let periods: Vec<T> = (0..c.len()).map(|i| t((2 + i % p) as f64)).collect();
    one!("mavp_into", mavp::mavp(c, &periods, 2, p.max(2)), |b| {
        mavp::mavp_into(c, &periods, 2, p.max(2), &mut b)
    });
    one!("mfi_into", mfi::mfi(h, l, c, v, p), |b| mfi::mfi_into(
        h, l, c, v, p, &mut b
    ));
    one!("midpoint_into", midpoint(c, p), |b| midpoint_into(
        c, p, &mut b
    ));
    one!("midprice_into", midprice(h, l, p), |b| midprice_into(
        h, l, p, &mut b
    ));
    one!("mom_into", mom(c, p), |b| mom_into(c, p, &mut b));
    one!("obv_into", obv(c, v), |b| obv_into(c, v, &mut b));
    one!("osma_into", osma(c, p, 2 * p, p), |b| osma_into(
        c,
        p,
        2 * p,
        p,
        &mut b
    ));
    one!("bulls_power_into", bulls_power(h, l, c, p), |b| {
        bulls_power_into(h, l, c, p, &mut b)
    });
    one!("bears_power_into", bears_power(h, l, c, p), |b| {
        bears_power_into(h, l, c, p, &mut b)
    });
    one!(
        "avgprice_into",
        price_transform::avgprice(o, h, l, c),
        |b| price_transform::avgprice_into(o, h, l, c, &mut b)
    );
    one!("medprice_into", price_transform::medprice(h, l), |b| {
        price_transform::medprice_into(h, l, &mut b)
    });
    one!("typprice_into", price_transform::typprice(h, l, c), |b| {
        price_transform::typprice_into(h, l, c, &mut b)
    });
    one!("wclprice_into", price_transform::wclprice(h, l, c), |b| {
        price_transform::wclprice_into(h, l, c, &mut b)
    });
    many!("qqe_into", qqe(c, p, 5, p, 4.236), |q = qqe, u = upper_band, lo = lower_band| qqe_into(c, p, 5, p, 4.236, &mut q, &mut u, &mut lo));
    one!("roc_into", roc(c, p), |b| roc_into(c, p, &mut b));
    one!("rocp_into", rocp(c, p), |b| rocp_into(c, p, &mut b));
    one!("rocr_into", rocr(c, p), |b| rocr_into(c, p, &mut b));
    one!("rocr100_into", rocr100(c, p), |b| rocr100_into(
        c, p, &mut b
    ));
    one!("rsi_into", rsi(c, p), |b| rsi_into(c, p, &mut b));
    one!("rvi_into", rvi(o, h, l, c, p), |b| rvi_into(
        o, h, l, c, p, &mut b
    ));
    one!("sar_into", sar(h, l), |b| sar_into(h, l, &mut b));
    one!(
        "sar_full_into",
        sar_full(h, l, t(0.02), t(0.02), t(0.2)),
        |b| sar_full_into(h, l, t(0.02), t(0.02), t(0.2), &mut b)
    );
    one!("sarext_into", sarext(h, l), |b| sarext_into(h, l, &mut b));
    one!(
        "sarext_full_into",
        sarext_full(h, l, SarExtParams::default()),
        |b| sarext_full_into(h, l, SarExtParams::default(), &mut b)
    );
    one!("sma_into", sma(c, p), |b| sma_into(c, p, &mut b));
    for s in [0, 5] {
        // No allocating twin: reference is the same call into a NaN-filled buffer.
        let mut r = filled(c, f64::NAN);
        let want = sma_from_idx_into(c, p, s, &mut r).map(|_| r);
        one!("sma_from_idx_into", want, |b| sma_from_idx_into(
            c, p, s, &mut b
        ));
    }
    one!("var_into", statistics::var(c, p), |b| statistics::var_into(
        c, p, &mut b
    ));
    one!("stddev_into", statistics::stddev(c, p), |b| {
        statistics::stddev_into(c, p, &mut b)
    });
    one!("skew_into", statistics::skew(c, p), |b| {
        statistics::skew_into(c, p, &mut b)
    });
    one!("kurt_into", statistics::kurt(c, p), |b| {
        statistics::kurt_into(c, p, &mut b)
    });
    one!("cov_into", statistics::cov(c, o, p), |b| {
        statistics::cov_into(c, o, p, &mut b)
    });
    one!("zscore_into", statistics::zscore(c, p), |b| {
        statistics::zscore_into(c, p, &mut b)
    });
    one!("mad_into", statistics::mad(c, p), |b| statistics::mad_into(
        c, p, &mut b
    ));
    one!("sem_into", statistics::sem(c, p), |b| statistics::sem_into(
        c, p, &mut b
    ));
    one!("correl_into", statistics::correl(c, o, p), |b| {
        statistics::correl_into(c, o, p, &mut b)
    });
    one!("beta_into", statistics::beta(c, o, p), |b| {
        statistics::beta_into(c, o, p, &mut b)
    });
    one!("linearreg_into", statistics::linearreg(c, p), |b| {
        statistics::linearreg_into(c, p, &mut b)
    });
    one!(
        "linearreg_slope_into",
        statistics::linearreg_slope(c, p),
        |b| statistics::linearreg_slope_into(c, p, &mut b)
    );
    one!(
        "linearreg_intercept_into",
        statistics::linearreg_intercept(c, p),
        |b| statistics::linearreg_intercept_into(c, p, &mut b)
    );
    one!(
        "linearreg_angle_into",
        statistics::linearreg_angle(c, p),
        |b| statistics::linearreg_angle_into(c, p, &mut b)
    );
    one!("tsf_into", statistics::tsf(c, p), |b| statistics::tsf_into(
        c, p, &mut b
    ));
    one!("stc_into", stc(c, p, 2 * p, p, 3), |b| stc_into(
        c,
        p,
        2 * p,
        p,
        3,
        &mut b
    ));
    st!(
        "stochastic_into",
        stochastic(h, l, c, p, 3, 3),
        StochasticOutput { k, d },
        |b| stochastic_into(h, l, c, p, 3, 3, &mut b)
    );
    st!(
        "stochastic_fast_into",
        stochastic_fast(h, l, c, p, 3),
        StochasticOutput { k, d },
        |b| stochastic_fast_into(h, l, c, p, 3, &mut b)
    );
    st!(
        "stochastic_slow_into",
        stochastic_slow(h, l, c, p, 3),
        StochasticOutput { k, d },
        |b| stochastic_slow_into(h, l, c, p, 3, &mut b)
    );
    st!(
        "stochastic_full_into",
        stochastic_full(h, l, c, p, 3, 3),
        StochasticOutput { k, d },
        |b| stochastic_full_into(h, l, c, p, 3, 3, &mut b)
    );
    let sto = Stochastic::new().with_k_period(p);
    st!(
        "Stochastic::compute_into",
        sto.compute(h, l, c),
        StochasticOutput { k, d },
        |b| sto.compute_into(h, l, c, &mut b)
    );
    many!("stochrsi_into", stochrsi::stochrsi(c, p, p, 3, 3), |k = fastk, dd = fastd| stochrsi::stochrsi_into(c, p, p, 3, 3, &mut k, &mut dd));
    many!("supertrend_into", supertrend(h, l, c, p, 3.0), |s = supertrend, u = upper_band, lo = lower_band, tr = trend| supertrend_into(h, l, c, p, 3.0, &mut s, &mut u, &mut lo, &mut tr));
    one!("t3_into", t3(c, p), |b| t3_into(c, p, &mut b));
    one!("t3_full_into", t3_full(c, p, t(0.7)), |b| t3_full_into(
        c,
        p,
        t(0.7),
        &mut b
    ));
    one!("tema_into", tema(c, p), |b| tema_into(c, p, &mut b));
    one!("trima_into", trima(c, p), |b| trima_into(c, p, &mut b));
    one!("trix_into", trix::trix(c, p), |b| trix::trix_into(
        c, p, &mut b
    ));
    one!("ulcer_index_into", ulcer_index::ulcer_index(c, p), |b| {
        ulcer_index::ulcer_index_into(c, p, &mut b)
    });
    one!(
        "ultosc_into",
        ultosc::ultosc(h, l, c, p, 2 * p, 4 * p),
        |b| ultosc::ultosc_into(h, l, c, p, 2 * p, 4 * p, &mut b)
    );
    many!("vortex_into", vortex(h, l, c, p), |pl = plus_vi, mi = minus_vi| vortex_into(h, l, c, p, &mut pl, &mut mi));
    one!("vwap_into", vwap(h, l, c, v), |b| vwap_into(
        h, l, c, v, &mut b
    ));
    one!("williams_r_into", williams_r(h, l, c, p), |b| {
        williams_r_into(h, l, c, p, &mut b)
    });
    one!("wma_into", wma(c, p), |b| wma_into(c, p, &mut b));
    one!("rolling_max_into", rx::rolling_max(c, p), |b| {
        rx::rolling_max_into(c, p, &mut b)
    });
    one!("rolling_min_into", rx::rolling_min(c, p), |b| {
        rx::rolling_min_into(c, p, &mut b)
    });
    many!("rolling_extrema_into", rx::rolling_extrema(c, p), |mx = max, mn = min| rx::rolling_extrema_into(c, p, &mut mx, &mut mn));

    cdl!(
        cdl_doji cdl_doji_into, cdl_dragonfly_doji cdl_dragonfly_doji_into,
        cdl_gravestone_doji cdl_gravestone_doji_into, cdl_longleg_doji cdl_longleg_doji_into,
        cdl_rickshaw_man cdl_rickshaw_man_into, cdl_marubozu cdl_marubozu_into,
        cdl_closing_marubozu cdl_closing_marubozu_into, cdl_spinning_top cdl_spinning_top_into,
        cdl_high_wave cdl_high_wave_into, cdl_long_line cdl_long_line_into,
        cdl_short_line cdl_short_line_into, cdl_hammer cdl_hammer_into,
        cdl_hanging_man cdl_hanging_man_into, cdl_inverted_hammer cdl_inverted_hammer_into,
        cdl_shooting_star cdl_shooting_star_into, cdl_takuri cdl_takuri_into,
        cdl_belt_hold cdl_belt_hold_into,
        cdl_morning_star cdl_morning_star_into, cdl_evening_star cdl_evening_star_into,
        cdl_morning_doji_star cdl_morning_doji_star_into, cdl_evening_doji_star cdl_evening_doji_star_into,
        cdl_abandoned_baby cdl_abandoned_baby_into, cdl_3white_soldiers cdl_3white_soldiers_into,
        cdl_3black_crows cdl_3black_crows_into, cdl_3inside cdl_3inside_into,
        cdl_3outside cdl_3outside_into, cdl_3line_strike cdl_3line_strike_into,
        cdl_3stars_in_south cdl_3stars_in_south_into, cdl_tristar cdl_tristar_into,
        cdl_identical_3crows cdl_identical_3crows_into,
        cdl_stick_sandwich cdl_stick_sandwich_into, cdl_unique_3river cdl_unique_3river_into,
        cdl_advance_block cdl_advance_block_into, cdl_stalled_pattern cdl_stalled_pattern_into,
        cdl_tasuki_gap cdl_tasuki_gap_into, cdl_upside_gap_2crows cdl_upside_gap_2crows_into,
        cdl_gap_side_side_white cdl_gap_side_side_white_into, cdl_breakaway cdl_breakaway_into,
        cdl_ladder_bottom cdl_ladder_bottom_into, cdl_mat_hold cdl_mat_hold_into,
        cdl_rise_fall_3methods cdl_rise_fall_3methods_into,
        cdl_concealing_baby_swallow cdl_concealing_baby_swallow_into,
        cdl_xside_gap_3methods cdl_xside_gap_3methods_into,
        cdl_engulfing cdl_engulfing_into, cdl_harami cdl_harami_into,
        cdl_harami_cross cdl_harami_cross_into, cdl_piercing cdl_piercing_into,
        cdl_dark_cloud_cover cdl_dark_cloud_cover_into, cdl_doji_star cdl_doji_star_into,
        cdl_kicking cdl_kicking_into, cdl_kicking_by_length cdl_kicking_by_length_into,
        cdl_matching_low cdl_matching_low_into, cdl_homing_pigeon cdl_homing_pigeon_into,
        cdl_in_neck cdl_in_neck_into, cdl_on_neck cdl_on_neck_into,
        cdl_thrusting cdl_thrusting_into, cdl_separating_lines cdl_separating_lines_into,
        cdl_counter_attack cdl_counter_attack_into, cdl_2crows cdl_2crows_into,
        cdl_hikkake cdl_hikkake_into, cdl_hikkake_mod cdl_hikkake_mod_into,
    );
}

fn sweep<T: SeriesElement + Val>(ty: &str, log: &mut Log) {
    for n in SIZES {
        for bad in [false, true] {
            let d = data::<T>(n, bad);
            for p in PERIODS {
                run(&d, p, &format!("{ty} n={n} bad={bad} p={p}"), log);
            }
        }
    }
}

#[test]
fn into_overwrites_dirty_buffer() {
    let mut log = Log {
        fails: vec![],
        seen: BTreeSet::new(),
        ran: BTreeSet::new(),
    };
    sweep::<f64>("f64", &mut log);
    sweep::<f32>("f32", &mut log);
    let never: Vec<_> = log.seen.difference(&log.ran).collect();
    assert!(
        never.is_empty(),
        "never exercised (allocating fn always errored): {never:?}"
    );
    assert!(
        log.fails.is_empty(),
        "{} violations:\n{}",
        log.fails.len(),
        log.fails.join("\n")
    );
}
