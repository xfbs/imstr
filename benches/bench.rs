use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use imstr::{data::Data, string::*};

type Boxed = Box<String>;
type Cloned = imstr::data::Cloned<String>;

static INPUT: &'static str = include_str!("../src/string.rs");

fn clone_repeat<S: Data<String>>(input: &ImString<S>, count: u64) {
    for _ in 0..count {
        black_box(input.clone());
    }
}

pub fn clone(c: &mut Criterion) {
    let mut g = c.benchmark_group("clone");

    for size in [1, 1000] {
        g.throughput(Throughput::Elements(size as u64));

        let string: ImString<Threadsafe> = ImString::from(INPUT);
        g.bench_with_input(BenchmarkId::new("threadsafe", size), &size, |b, &s| {
            b.iter(|| clone_repeat(&string, s))
        });

        let string: ImString<Local> = ImString::from(INPUT);
        g.bench_with_input(BenchmarkId::new("local", size), &size, |b, &s| {
            b.iter(|| clone_repeat(&string, s))
        });

        let string: ImString<Boxed> = ImString::from(INPUT);
        g.bench_with_input(BenchmarkId::new("boxed", size), &size, |b, &s| {
            b.iter(|| clone_repeat(&string, s))
        });

        let string: ImString<Cloned> = ImString::from(INPUT);
        g.bench_with_input(BenchmarkId::new("cloned", size), &size, |b, &s| {
            b.iter(|| clone_repeat(&string, s))
        });
    }

    g.finish();
}

fn slice_down<S: Data<String>>(input: &ImString<S>) -> ImString<S> {
    let mut slice = input.clone();
    while !slice.is_empty() {
        for offset in 1.. {
            if let Ok(next) = slice.try_slice(offset..) {
                slice = next;
                break;
            }
        }
    }

    slice
}

pub fn slice(c: &mut Criterion) {
    let mut g = c.benchmark_group("slice");

    let size = INPUT.len() as u64;
    g.throughput(Throughput::Elements(size));

    let string: ImString<Threadsafe> = ImString::from(INPUT);
    g.bench_function("threadsafe", |b| b.iter(|| black_box(slice_down(&string))));

    let string: ImString<Local> = ImString::from(INPUT);
    g.bench_function("local", |b| b.iter(|| black_box(slice_down(&string))));

    let string: ImString<Boxed> = ImString::from(INPUT);
    g.bench_function("boxed", |b| b.iter(|| black_box(slice_down(&string))));

    let string: ImString<Cloned> = ImString::from(INPUT);
    g.bench_function("cloned", |b| b.iter(|| black_box(slice_down(&string))));

    g.finish();
}

criterion_group!(benches, clone, slice);
criterion_main!(benches);
