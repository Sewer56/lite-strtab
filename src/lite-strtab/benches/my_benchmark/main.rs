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
    let table_null_padded = build_table_null_padded(entries, total_bytes);
    let vec_strings = build_vec_strings(entries);
    let boxed_str_slice = build_boxed_str_slice(entries);

    bench_get_group(
        c,
        dataset_name,
        "get",
        "",
        total_bytes,
        &table,
        Some(&table_null_padded),
        &vec_strings,
        &boxed_str_slice,
        observe_str_ahash,
    );
    bench_get_unchecked_group(
        c,
        dataset_name,
        "get_unchecked",
        "",
        total_bytes,
        &table,
        Some(&table_null_padded),
        &vec_strings,
        &boxed_str_slice,
        observe_str_ahash,
    );

    bench_get_group(
        c,
        dataset_name,
        "get_u8",
        "_u8",
        total_bytes,
        &table,
        None,
        &vec_strings,
        &boxed_str_slice,
        observe_str_u8,
    );
    bench_get_unchecked_group(
        c,
        dataset_name,
        "get_u8_unchecked",
        "_u8",
        total_bytes,
        &table,
        None,
        &vec_strings,
        &boxed_str_slice,
        observe_str_u8,
    );

    bench_get_group(
        c,
        dataset_name,
        "get_usize",
        "_usize",
        total_bytes,
        &table,
        None,
        &vec_strings,
        &boxed_str_slice,
        observe_str_usize,
    );
    bench_get_unchecked_group(
        c,
        dataset_name,
        "get_usize_unchecked",
        "_usize",
        total_bytes,
        &table,
        None,
        &vec_strings,
        &boxed_str_slice,
        observe_str_usize,
    );

    bench_iter_group(
        c,
        dataset_name,
        "iter",
        "",
        total_bytes,
        &table,
        Some(&table_null_padded),
        &vec_strings,
        &boxed_str_slice,
        observe_str_ahash,
    );
    bench_iter_group(
        c,
        dataset_name,
        "iter_u8",
        "_u8",
        total_bytes,
        &table,
        None,
        &vec_strings,
        &boxed_str_slice,
        observe_str_u8,
    );
    bench_iter_group(
        c,
        dataset_name,
        "iter_usize",
        "_usize",
        total_bytes,
        &table,
        None,
        &vec_strings,
        &boxed_str_slice,
        observe_str_usize,
    );

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

    let mut construct_group_null_padded =
        c.benchmark_group(format!("{dataset_name}/construct_null_padded"));
    construct_group_null_padded.throughput(Throughput::Bytes(
        (total_bytes.saturating_add(string_count)) as u64,
    ));
    construct_group_null_padded.bench_function("typical_builder", |b| {
        b.iter(|| {
            let mut builder = StringTableBuilder::new_null_padded();
            for value in entries {
                builder
                    .try_push(value)
                    .expect("failed to insert benchmark path");
            }

            let table = builder.build();
            black_box(table.as_bytes().len())
        })
    });
    construct_group_null_padded.finish();
}

fn bench_get_group<F>(
    c: &mut Criterion,
    dataset_name: &str,
    group_name_suffix: &str,
    benchmark_name_suffix: &str,
    total_bytes: usize,
    table: &StringTable<u32, u32>,
    table_null_padded: Option<&StringTable<u32, u32, true>>,
    vec_strings: &[String],
    boxed_str_slice: &[Box<str>],
    observe: F,
) where
    F: Fn(&str) -> usize + Copy,
{
    let mut group = c.benchmark_group(format!("{dataset_name}/{group_name_suffix}"));
    group.throughput(Throughput::Bytes(total_bytes as u64));

    group.bench_function(format!("vec_string_for_loop{benchmark_name_suffix}"), |b| {
        b.iter(|| {
            let mut checksum = 0usize;
            for index in 0..vec_strings.len() {
                let value = vec_strings
                    .get(index)
                    .expect("benchmark index out of bounds");
                checksum = checksum.wrapping_add(observe(value));
            }
            black_box(checksum)
        })
    });
    group.bench_function(
        format!("boxed_str_slice_for_loop{benchmark_name_suffix}"),
        |b| {
            b.iter(|| {
                let mut checksum = 0usize;
                for index in 0..boxed_str_slice.len() {
                    let value = boxed_str_slice
                        .get(index)
                        .expect("benchmark index out of bounds")
                        .as_ref();
                    checksum = checksum.wrapping_add(observe(value));
                }
                black_box(checksum)
            })
        },
    );
    group.bench_function(
        format!("lite_strtab_for_loop{benchmark_name_suffix}"),
        |b| {
            b.iter(|| {
                let mut checksum = 0usize;
                for index in 0..table.len() {
                    let id = StringId::new(index as u32);
                    let value = table.get(id).expect("benchmark id out of bounds");
                    checksum = checksum.wrapping_add(observe(value));
                }
                black_box(checksum)
            })
        },
    );
    if let Some(table_null_padded) = table_null_padded {
        group.bench_function(
            format!("lite_strtab_for_loop{benchmark_name_suffix}_null_padded"),
            |b| {
                b.iter(|| {
                    let mut checksum = 0usize;
                    for index in 0..table_null_padded.len() {
                        let id = StringId::new(index as u32);
                        let value = table_null_padded
                            .get(id)
                            .expect("benchmark id out of bounds");
                        checksum = checksum.wrapping_add(observe(value));
                    }
                    black_box(checksum)
                })
            },
        );
    }

    group.finish();
}

fn bench_get_unchecked_group<F>(
    c: &mut Criterion,
    dataset_name: &str,
    group_name_suffix: &str,
    benchmark_name_suffix: &str,
    total_bytes: usize,
    table: &StringTable<u32, u32>,
    table_null_padded: Option<&StringTable<u32, u32, true>>,
    vec_strings: &[String],
    boxed_str_slice: &[Box<str>],
    observe: F,
) where
    F: Fn(&str) -> usize + Copy,
{
    let mut group = c.benchmark_group(format!("{dataset_name}/{group_name_suffix}"));
    group.throughput(Throughput::Bytes(total_bytes as u64));

    group.bench_function(
        format!("vec_string_for_loop{benchmark_name_suffix}_unchecked"),
        |b| {
            b.iter(|| {
                let mut checksum = 0usize;
                for index in 0..vec_strings.len() {
                    let value = unsafe { vec_strings.get_unchecked(index) };
                    checksum = checksum.wrapping_add(observe(value));
                }
                black_box(checksum)
            })
        },
    );
    group.bench_function(
        format!("boxed_str_slice_for_loop{benchmark_name_suffix}_unchecked"),
        |b| {
            b.iter(|| {
                let mut checksum = 0usize;
                for index in 0..boxed_str_slice.len() {
                    let value = unsafe { boxed_str_slice.get_unchecked(index) }.as_ref();
                    checksum = checksum.wrapping_add(observe(value));
                }
                black_box(checksum)
            })
        },
    );
    group.bench_function(
        format!("lite_strtab_for_loop{benchmark_name_suffix}_unchecked"),
        |b| {
            b.iter(|| {
                let mut checksum = 0usize;
                for index in 0..table.len() {
                    let id = StringId::new(index as u32);
                    let value = unsafe { table.get_unchecked(id) };
                    checksum = checksum.wrapping_add(observe(value));
                }
                black_box(checksum)
            })
        },
    );
    if let Some(table_null_padded) = table_null_padded {
        group.bench_function(
            format!("lite_strtab_for_loop{benchmark_name_suffix}_null_padded_unchecked"),
            |b| {
                b.iter(|| {
                    let mut checksum = 0usize;
                    for index in 0..table_null_padded.len() {
                        let id = StringId::new(index as u32);
                        let value = unsafe { table_null_padded.get_unchecked(id) };
                        checksum = checksum.wrapping_add(observe(value));
                    }
                    black_box(checksum)
                })
            },
        );
    }

    group.finish();
}

fn bench_iter_group<F>(
    c: &mut Criterion,
    dataset_name: &str,
    group_name_suffix: &str,
    benchmark_name_suffix: &str,
    total_bytes: usize,
    table: &StringTable<u32, u32>,
    table_null_padded: Option<&StringTable<u32, u32, true>>,
    vec_strings: &[String],
    boxed_str_slice: &[Box<str>],
    observe: F,
) where
    F: Fn(&str) -> usize + Copy,
{
    let mut group = c.benchmark_group(format!("{dataset_name}/{group_name_suffix}"));
    group.throughput(Throughput::Bytes(total_bytes as u64));

    group.bench_function(format!("vec_string_iter{benchmark_name_suffix}"), |b| {
        b.iter(|| {
            let mut checksum = 0usize;
            for value in vec_strings.iter() {
                checksum = checksum.wrapping_add(observe(value));
            }
            black_box(checksum)
        })
    });
    group.bench_function(
        format!("boxed_str_slice_iter{benchmark_name_suffix}"),
        |b| {
            b.iter(|| {
                let mut checksum = 0usize;
                for value in boxed_str_slice.iter() {
                    checksum = checksum.wrapping_add(observe(value.as_ref()));
                }
                black_box(checksum)
            })
        },
    );
    group.bench_function(format!("lite_strtab_iter{benchmark_name_suffix}"), |b| {
        b.iter(|| {
            let mut checksum = 0usize;
            for value in table.iter() {
                checksum = checksum.wrapping_add(observe(value));
            }
            black_box(checksum)
        })
    });
    if let Some(table_null_padded) = table_null_padded {
        group.bench_function(
            format!("lite_strtab_iter{benchmark_name_suffix}_null_padded"),
            |b| {
                b.iter(|| {
                    let mut checksum = 0usize;
                    for value in table_null_padded.iter() {
                        checksum = checksum.wrapping_add(observe(value));
                    }
                    black_box(checksum)
                })
            },
        );
    }

    group.finish();
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

fn build_table_null_padded(entries: &[String], total_bytes: usize) -> StringTable<u32, u32, true> {
    let mut builder = StringTableBuilder::with_capacity_null_padded(
        entries.len(),
        total_bytes.saturating_add(entries.len()),
    );
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

#[inline(always)]
fn observe_str_ahash(value: &str) -> usize {
    let mut hasher = AHasher::default();
    value.hash(&mut hasher);
    hasher.finish() as usize
}

#[inline(always)]
fn observe_str_u8(value: &str) -> usize {
    let bytes = value.as_bytes();
    let mut checksum = 0usize;

    // Safety: `ptr` and `end` are derived from the same slice and `ptr`
    // advances within bounds until it reaches `end`.
    unsafe {
        let mut ptr = bytes.as_ptr();
        let end = ptr.add(bytes.len());

        while ptr != end {
            checksum = checksum.wrapping_add(*ptr as usize);
            ptr = ptr.add(1);
        }
    }

    checksum
}

#[inline(always)]
fn observe_str_usize(value: &str) -> usize {
    let bytes = value.as_bytes();
    let mut checksum = 0usize;

    let chunk_size = core::mem::size_of::<usize>();
    let chunk_count = bytes.len() / chunk_size;
    let chunk_bytes = chunk_count * chunk_size;

    // Safety: all pointer arithmetic stays within `bytes` bounds, and
    // `usize` has no invalid bit patterns, so unaligned loads are valid.
    unsafe {
        let base = bytes.as_ptr();
        let mut offset = 0usize;

        while offset < chunk_bytes {
            let chunk_ptr = base.add(offset).cast::<usize>();
            checksum = checksum.wrapping_add(chunk_ptr.read_unaligned());
            offset += chunk_size;
        }

        while offset < bytes.len() {
            checksum = checksum.wrapping_add(*base.add(offset) as usize);
            offset += 1;
        }
    }

    checksum
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = criterion_benchmark
}

criterion_main!(benches);
