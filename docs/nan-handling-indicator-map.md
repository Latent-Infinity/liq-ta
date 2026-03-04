# NaN Handling Indicator Map (Draft)

This map classifies indicators by NaN-handling behavior to guide implementation updates. It is a working inventory for Stage 0 and can be refined as behavior is verified.

## Rolling-window (fixed window, no recursive state)

- `sma`, `wma`, `trima`, `midpoint`, `midprice`, `mavp`
- `bollinger`, `rolling_stddev`
- `donchian`, `aroon`, `aroonosc`, `williams_r`
- `stochastic` (fast/slow/full)
- `mom`, `roc`, `rocp`, `rocr`, `rocr100`, `cmo`
- `cci`, `ultosc`, `mfi`
- `statistics`: `beta`, `correl`, `linearreg`, `linearreg_angle`, `linearreg_intercept`, `linearreg_slope`, `tsf`, `var`
- `true_range` (window of 2)

## Cumulative (recursive state / running totals)

- `ema`, `ema_wilder`, `ema_with_alpha`
- `dema`, `tema`, `t3`, `kama`, `mama`, `trix`
- `apo`, `ppo`, `macd`
- `obv`, `ad`, `adosc`, `vwap`
- `sar`, `sarext`
- `ht_dcperiod`, `ht_dcphase`, `ht_trendline`, `ht_trendmode`, `ht_phasor`, `ht_sine`
- `rsi`

## Mixed behavior (rolling + cumulative)

- `atr` (window seed + Wilder smoothing)
- `adx`, `dx`, `adxr`, `plus_dm`, `minus_dm` (rolling windows + Wilder smoothing)
- `stochrsi` (RSI + rolling stochastic/SMA)

## Pointwise (no lookback / per-bar)

- `bop`
- `price_transform`: `avgprice`, `medprice`, `typprice`, `wclprice`

## Multi-output indicators

- `bollinger`, `donchian`, `macd`, `mama`, `adx`, `aroon`
- `stochastic` (fast/slow/full), `stochrsi`
- `ht_phasor`, `ht_sine`

## Exclusions

- Candlestick pattern indicators return integer codes; NaN/Infinity propagation rules do not apply.
