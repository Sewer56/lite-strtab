use std::hash::{Hash, Hasher};
use std::hint::black_box;
use std::io::Read;

use ahash::AHasher;
use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use lite_strtab::{StringId, StringTable, StringTableBuilder};

const YAKUZA_KIWAMI_DATASET_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/benches/data/YakuzaKiwami.zst");
const BENCHMARK_DATASET_NAME: &str = "YakuzaKiwami";

struct Dataset {
    entries: Vec<String>,
    total_bytes: usize,
}

fn criterion_benchmark(c: &mut Criterion) {
    let benchmark_dataset = load_dataset(YAKUZA_KIWAMI_DATASET_PATH);
    run_dataset_benchmarks(c, BENCHMARK_DATASET_NAME, &benchmark_dataset);
}

fn run_dataset_benchmarks(c: &mut Criterion, dataset_name: &str, dataset: &Dataset) {
    let entries = &dataset.entries;
    let total_bytes = dataset.total_bytes;
    let string_count = entries.len();
    let table = build_table(entries, total_bytes);
    let vec_strings = build_vec_strings(entries);
    let boxed_str_slice = build_boxed_str_slice(entries);

    let mut get_group = c.benchmark_group(format!("{dataset_name}/get"));
    get_group.throughput(Throughput::Bytes(total_bytes as u64));

    get_group.bench_function("vec_string_for_loop", |b| {
        b.iter(|| {
            let mut checksum = 0usize;
            for index in 0..string_count {
                let value = vec_strings
                    .get(index)
                    .expect("benchmark index out of bounds");
                checksum = checksum.wrapping_add(observe_str(value));
            }
            black_box(checksum)
        })
    });
    get_group.bench_function("boxed_str_slice_for_loop", |b| {
        b.iter(|| {
            let mut checksum = 0usize;
            for index in 0..string_count {
                let value = boxed_str_slice
                    .get(index)
                    .expect("benchmark index out of bounds")
                    .as_ref();
                checksum = checksum.wrapping_add(observe_str(value));
            }
            black_box(checksum)
        })
    });
    get_group.bench_function("lite_strtab_for_loop", |b| {
        b.iter(|| {
            let mut checksum = 0usize;
            for index in 0..string_count {
                let id = StringId::new(index as u32);
                let value = table.get(id).expect("benchmark id out of bounds");
                checksum = checksum.wrapping_add(observe_str(value));
            }
            black_box(checksum)
        })
    });
    get_group.finish();

    let mut get_unchecked_group = c.benchmark_group(format!("{dataset_name}/get_unchecked"));
    get_unchecked_group.throughput(Throughput::Bytes(total_bytes as u64));

    get_unchecked_group.bench_function("vec_string_for_loop_unchecked", |b| {
        b.iter(|| {
            let mut checksum = 0usize;
            for index in 0..string_count {
                let value = unsafe { vec_strings.get_unchecked(index) };
                checksum = checksum.wrapping_add(observe_str(value));
            }
            black_box(checksum)
        })
    });
    get_unchecked_group.bench_function("boxed_str_slice_for_loop_unchecked", |b| {
        b.iter(|| {
            let mut checksum = 0usize;
            for index in 0..string_count {
                let value = unsafe { boxed_str_slice.get_unchecked(index) }.as_ref();
                checksum = checksum.wrapping_add(observe_str(value));
            }
            black_box(checksum)
        })
    });
    get_unchecked_group.bench_function("lite_strtab_for_loop_unchecked", |b| {
        b.iter(|| {
            let mut checksum = 0usize;
            for index in 0..string_count {
                let id = StringId::new(index as u32);
                let value = unsafe { table.get_unchecked(id) };
                checksum = checksum.wrapping_add(observe_str(value));
            }
            black_box(checksum)
        })
    });
    get_unchecked_group.finish();

    let mut insert_group = c.benchmark_group(format!("{dataset_name}/insert"));
    insert_group.throughput(Throughput::Bytes(total_bytes as u64));
    insert_group.bench_function("reinsert_items", |b| {
        b.iter(|| {
            let mut builder = StringTableBuilder::<u32>::new();
            for value in entries {
                builder
                    .try_push(value)
                    .expect("failed to insert benchmark path");
            }
            black_box(builder.bytes_len())
        })
    });
    insert_group.bench_function("preallocated_buffer", |b| {
        b.iter(|| {
            let mut builder = StringTableBuilder::<u32>::with_capacity(string_count, total_bytes);
            for value in entries {
                builder
                    .try_push(value)
                    .expect("failed to insert benchmark path");
            }
            black_box(builder.bytes_len())
        })
    });
    insert_group.finish();

    let mut build_group = c.benchmark_group(format!("{dataset_name}/build"));
    build_group.throughput(Throughput::Bytes(total_bytes as u64));
    build_group.bench_function("reference_realloc_only", |b| {
        b.iter_batched(
            || {
                let mut builder =
                    StringTableBuilder::<u32>::with_capacity(string_count, total_bytes);
                for value in entries {
                    builder
                        .try_push(value)
                        .expect("failed to insert benchmark path");
                }
                builder
            },
            |builder| {
                let table = builder.build();
                black_box(table.as_bytes().len())
            },
            BatchSize::LargeInput,
        )
    });
    build_group.finish();
}

fn load_dataset(dataset_path: &str) -> Dataset {
    let file = std::fs::File::open(dataset_path).expect("failed to open benchmark dataset");
    let mut decoder = zstd::stream::read::Decoder::new(file)
        .expect("failed to create zstd decoder for benchmark dataset");

    let mut decompressed = String::new();
    decoder
        .read_to_string(&mut decompressed)
        .expect("failed to decompress benchmark dataset");

    let mut entries = Vec::new();
    let mut total_bytes = 0usize;

    for line in decompressed.lines() {
        total_bytes = total_bytes.saturating_add(line.len());
        entries.push(line.to_owned());
    }

    assert!(
        !entries.is_empty(),
        "benchmark dataset did not contain any file paths"
    );

    Dataset {
        entries,
        total_bytes,
    }
}

fn build_table(entries: &[String], total_bytes: usize) -> StringTable<u32, u32> {
    let mut builder = StringTableBuilder::<u32>::with_capacity(entries.len(), total_bytes);
    for value in entries {
        builder
            .try_push(value)
            .expect("failed to insert benchmark path");
    }
    builder.build()
}

fn build_vec_strings(entries: &[String]) -> Vec<String> {
    entries.to_vec()
}

fn build_boxed_str_slice(entries: &[String]) -> Box<[Box<str>]> {
    let mut vec = Vec::with_capacity(entries.len());
    for value in entries {
        vec.push(value.clone().into_boxed_str());
    }
    vec.into_boxed_slice()
}

// Read full payload bytes so cache-coherency effects are reflected in get benchmarks.
#[inline(always)]
fn observe_str(value: &str) -> usize {
    let mut hasher = AHasher::default();
    value.hash(&mut hasher);
    hasher.finish() as usize
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = criterion_benchmark
}

criterion_main!(benches);
