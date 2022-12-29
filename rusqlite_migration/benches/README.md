# Alpha Benchmarks

This is a collection of alpha-quality benchmarks. This is a work in progress. They are a bit quirky still and have the following gotchas:

* With Criterion, the migrations benchmark actually estimate the cycles spent in the benchmarked code, as the code benchmarked is a bit too big to reliably measure other indicators. This makes it close to Iai benchmarks.
* Criterion benchmark are quite often noisy and have outliers (even on a quiet computer)

## Why Both Criterion and Iai?

> Comparison with Criterion-rs
>
> I intend Iai to be a complement to Criterion-rs, not a competitor. The two projects measure different things in different ways and have different pros, cons, and limitations, so for most projects the best approach is to use both.

From https://bheisler.github.io/criterion.rs/book/iai/comparison.html

## Criterion

Just run
```
cargo bench --bench criterion
```

The migration benchmark use perf counters, so if you get a permission denied error, run (on GNU/Linux):

``` bash
echo 1 | sudo tee /proc/sys/kernel/perf_event_paranoid
```

## Iai

Just run
```
cargo bench --bench iai
```
