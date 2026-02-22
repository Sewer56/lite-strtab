# lite-strtab

[![Crates.io](https://img.shields.io/crates/v/lite-strtab.svg)](https://crates.io/crates/lite-strtab)
[![Docs.rs](https://docs.rs/lite-strtab/badge.svg)](https://docs.rs/lite-strtab)
[![CI](https://github.com/Sewer56/lite-strtab/actions/workflows/rust.yml/badge.svg)](https://github.com/Sewer56/lite-strtab/actions)

`lite-strtab` is a crate for storing many immutable strings in one buffer with minimal resource usage.

It is a simple, in-memory, build-once data structure:

- Push strings into a builder
- Finalize into an immutable table
- Look strings up by [`StringId`]

As simple as that.

## Design overview

- `Memory`: one UTF-8 byte buffer plus one compact offset table; optional NUL-termination
- `CPU`: cheap ID-based lookups (bounds check + two offset reads)
- `Binary size`: no panics on insertion, avoiding backtrace overhead

Data types for e.g. offsets, string IDs, etc. can be swapped out for
smaller/larger ones as needed; to adapt to your required data size/count.

## Why this exists

[Note: Numbers are for 64-bit machines.]

Types like [`Box<[String]>`] and [`Box<[Box<str>]>`] keep one handle per element:
- [`Box<[String]>`]: 24 bytes (ptr + len + capacity)
- [`Box<[Box<str>]>`]: 16 bytes (ptr + len)

This is in addition to allocator overhead per string allocation (metadata + alignment).

In contrast, `lite-strtab` aims to remove these overheads by storing all strings
in a single buffer, with an offset table to define string boundaries.

- One raw alloc containing all UTF-8 bytes
- One offset table (`len + 1` entries, with a final sentinel)

This removes per-string allocation overhead.
Rather than storing 16/24 bytes per string (+ allocation overhead), we just store
4 bytes per string (for [`u32`] offsets) + one final sentinel offset.

## Installation

```toml
[dependencies]
lite-strtab = "0.1.0"
```

## Feature flags

| Feature   | Description                                                                                                       |
| --------- | ----------------------------------------------------------------------------------------------------------------- |
| `std`     | Enabled by default. The crate still uses `#![no_std]` + `alloc` internally.                                       |
| `nightly` | Uses Rust's unstable allocator API instead of `allocator-api2` and requires a nightly compiler (`allocator_api`). |

## Basic usage

```rust
use lite_strtab::StringTableBuilder;

let mut builder = StringTableBuilder::new();
let hello = builder.try_push("hello").unwrap();
let world = builder.try_push("world").unwrap();

let table = builder.build();
assert_eq!(table.get(hello), Some("hello"));
assert_eq!(table.get(world), Some("world"));
```

## Custom allocator

```rust
# #![cfg_attr(feature = "nightly", feature(allocator_api))]
use lite_strtab::{Global, StringTableBuilder};

let mut builder = StringTableBuilder::<u32>::new_in(Global);
let id = builder.try_push("example").unwrap();
let table = builder.build();

assert_eq!(table.get(id), Some("example"));
```

## Custom offset type

```rust
# #![cfg_attr(feature = "nightly", feature(allocator_api))]
use lite_strtab::{Global, StringTableBuilder};

let mut builder = StringTableBuilder::<u16>::new_in(Global);
let id = builder.try_push("small").unwrap();
let table = builder.build();

assert_eq!(table.get(id), Some("small"));
```

## Custom [`StringId`] type

```rust
# #![cfg_attr(feature = "nightly", feature(allocator_api))]
use lite_strtab::{Global, StringTableBuilder};

let mut builder = StringTableBuilder::<u16, u8>::new_in(Global);
let id = builder.try_push("tiny-id").unwrap();
let table = builder.build();

assert_eq!(id.into_raw(), 0u8);
assert_eq!(table.get(id), Some("tiny-id"));
```

## Null-padded mode

Set `NULL_PADDED = true` to store strings with a trailing NUL byte:

```rust
use lite_strtab::StringTableBuilder;

let mut builder = StringTableBuilder::new_null_padded();
let id = builder.try_push("hello").unwrap();
let table = builder.build();

assert_eq!(table.get(id), Some("hello"));   // NUL trimmed
assert_eq!(table.as_bytes(), b"hello\0");   // raw bytes include NUL
```

## Scope

This crate focuses on in-memory string storage only.

It does not do:

- serialization/deserialization
- compression/decompression
- sorting/deduplication policies

If you need those, build them in a wrapper around this crate.

## Benchmarks

Memory usage was measured on Linux with glibc malloc using `malloc_usable_size`
to capture actual allocator block sizes including alignment and metadata overhead.

They can be captured with `cargo run -p lite-strtab --features memory-report --bin memory_report`.

How to read these tables:

- `Total` = `Heap allocations` + `Distributed fields` + `One-time metadata`
- `Distributed fields` = string references distributed across fields/structs (e.g. `String`, `Box<str>`, `StringId<u16>`)
- in these results, `lite-strtab` uses `StringId<u16>`

### Datasets

Three representative datasets were used:

- **YakuzaKiwami**: 4,650 game file paths (238,109 bytes), for example `sound/ja/some_file.awb`.
- **EnvKeys**: 109 environment variable names from an API specification (1,795 bytes).
- **ApiUrls**: 90 REST API endpoint URLs (3,970 bytes).

#### YakuzaKiwami (4650 entries, 238,109 bytes)

Summary

| Representation              | Total               | Heap allocations    | Distributed fields  | vs lite-strtab |
| --------------------------- | ------------------- | ------------------- | ------------------- | -------------- |
| `lite-strtab`               | 266068 (259.83 KiB) | 256736 (250.72 KiB) | 9300 (9.08 KiB)     | 1.00x          |
| `lite-strtab (null-padded)` | 270708 (264.36 KiB) | 261376 (255.25 KiB) | 9300 (9.08 KiB)     | 1.02x          |
| `Vec<String>`               | 384240 (375.23 KiB) | 272640 (266.25 KiB) | 111600 (108.98 KiB) | 1.44x          |
| `Box<[Box<str>]>`           | 346928 (338.80 KiB) | 272528 (266.14 KiB) | 74400 (72.66 KiB)   | 1.30x          |

Heap allocations (tree)

- `lite-strtab`: `256736 (250.72 KiB)` (`96.49%`)
  - `StringTable<u32, u16>` byte buffer: `238120 (232.54 KiB)` (`92.75%` of heap) - concatenated UTF-8 string payload data
  - `StringTable<u32, u16>` offsets buffer: `18616 (18.18 KiB)` (`7.25%` of heap) - `u32` offsets into the shared byte buffer
- `lite-strtab (null-padded)`: `261376 (255.25 KiB)` (`96.55%`)
  - `StringTable<u32, u16, true>` byte buffer: `242760 (237.07 KiB)` (`92.88%` of heap) - concatenated UTF-8 string payload data with NUL terminators
  - `StringTable<u32, u16, true>` offsets buffer: `18616 (18.18 KiB)` (`7.12%` of heap) - `u32` offsets into the shared byte buffer
- `Vec<String>`: `272640 (266.25 KiB)` (`70.96%`)
  - `String` payload allocations: `272640 (266.25 KiB)` (`100.00%` of heap) - one UTF-8 allocation per string
- `Box<[Box<str>]>`: `272528 (266.14 KiB)` (`78.55%`)
  - `Box<str>` payload allocations: `272528 (266.14 KiB)` (`100.00%` of heap) - one UTF-8 allocation per string

Distributed fields (per-string handles)

- `lite-strtab`: `9300 (9.08 KiB)` (`3.50%`) - `StringId<u16>`: field per string (`2 B` each x `4650`)
- `Vec<String>`: `111600 (108.98 KiB)` (`29.04%`) - `String`: field per string (`24 B` each x `4650`)
- `Box<[Box<str>]>`: `74400 (72.66 KiB)` (`21.45%`) - `Box<str>`: field per string (`16 B` each x `4650`)

One-time metadata (table object itself)

- `lite-strtab`: `32 B` (`StringTable<u32, u16>` struct itself; one per table, not per string)

#### EnvKeys (109 entries, 1,795 bytes)

Summary

| Representation              | Total           | Heap allocations | Distributed fields | vs lite-strtab |
| --------------------------- | --------------- | ---------------- | ------------------ | -------------- |
| `lite-strtab`               | 2490 (2.43 KiB) | 2240 (2.19 KiB)  | 218 B              | 1.00x          |
| `lite-strtab (null-padded)` | 2602 (2.54 KiB) | 2352 (2.30 KiB)  | 218 B              | 1.04x          |
| `Vec<String>`               | 5504 (5.38 KiB) | 2888 (2.82 KiB)  | 2616 (2.55 KiB)    | 2.21x          |
| `Box<[Box<str>]>`           | 4472 (4.37 KiB) | 2728 (2.66 KiB)  | 1744 (1.70 KiB)    | 1.80x          |

Heap allocations (tree)

- `lite-strtab`: `2240 (2.19 KiB)` (`89.96%`)
  - `StringTable<u32, u16>` byte buffer: `1800 (1.76 KiB)` (`80.36%` of heap) - concatenated UTF-8 string payload data
  - `StringTable<u32, u16>` offsets buffer: `440 B` (`19.64%` of heap) - `u32` offsets into the shared byte buffer
- `lite-strtab (null-padded)`: `2352 (2.30 KiB)` (`90.39%`)
  - `StringTable<u32, u16, true>` byte buffer: `1912 (1.87 KiB)` (`81.29%` of heap) - concatenated UTF-8 string payload data with NUL terminators
  - `StringTable<u32, u16, true>` offsets buffer: `440 B` (`18.71%` of heap) - `u32` offsets into the shared byte buffer
- `Vec<String>`: `2888 (2.82 KiB)` (`52.47%`)
  - `String` payload allocations: `2888 (2.82 KiB)` (`100.00%` of heap) - one UTF-8 allocation per string
- `Box<[Box<str>]>`: `2728 (2.66 KiB)` (`61.00%`)
  - `Box<str>` payload allocations: `2728 (2.66 KiB)` (`100.00%` of heap) - one UTF-8 allocation per string

Distributed fields (per-string handles)

- `lite-strtab`: `218 B` (`8.76%`) - `StringId<u16>`: field per string (`2 B` each x `109`)
- `Vec<String>`: `2616 (2.55 KiB)` (`47.53%`) - `String`: field per string (`24 B` each x `109`)
- `Box<[Box<str>]>`: `1744 (1.70 KiB)` (`39.00%`) - `Box<str>`: field per string (`16 B` each x `109`)

One-time metadata (table object itself)

- `lite-strtab`: `32 B` (`StringTable<u32, u16>` struct itself; one per table, not per string)

#### ApiUrls (90 entries, 3,970 bytes)

Summary

| Representation              | Total           | Heap allocations | Distributed fields | vs lite-strtab |
| --------------------------- | --------------- | ---------------- | ------------------ | -------------- |
| `lite-strtab`               | 4564 (4.46 KiB) | 4352 (4.25 KiB)  | 180 B              | 1.00x          |
| `lite-strtab (null-padded)` | 4660 (4.55 KiB) | 4448 (4.34 KiB)  | 180 B              | 1.02x          |
| `Vec<String>`               | 6896 (6.73 KiB) | 4736 (4.62 KiB)  | 2160 (2.11 KiB)    | 1.51x          |
| `Box<[Box<str>]>`           | 6112 (5.97 KiB) | 4672 (4.56 KiB)  | 1440 (1.41 KiB)    | 1.34x          |

Heap allocations (tree)

- `lite-strtab`: `4352 (4.25 KiB)` (`95.35%`)
  - `StringTable<u32, u16>` byte buffer: `3976 (3.88 KiB)` (`91.36%` of heap) - concatenated UTF-8 string payload data
  - `StringTable<u32, u16>` offsets buffer: `376 B` (`8.64%` of heap) - `u32` offsets into the shared byte buffer
- `lite-strtab (null-padded)`: `4448 (4.34 KiB)` (`95.45%`)
  - `StringTable<u32, u16, true>` byte buffer: `4072 (3.98 KiB)` (`91.55%` of heap) - concatenated UTF-8 string payload data with NUL terminators
  - `StringTable<u32, u16, true>` offsets buffer: `376 B` (`8.45%` of heap) - `u32` offsets into the shared byte buffer
- `Vec<String>`: `4736 (4.62 KiB)` (`68.68%`)
  - `String` payload allocations: `4736 (4.62 KiB)` (`100.00%` of heap) - one UTF-8 allocation per string
- `Box<[Box<str>]>`: `4672 (4.56 KiB)` (`76.44%`)
  - `Box<str>` payload allocations: `4672 (4.56 KiB)` (`100.00%` of heap) - one UTF-8 allocation per string

Distributed fields (per-string handles)

- `lite-strtab`: `180 B` (`3.94%`) - `StringId<u16>`: field per string (`2 B` each x `90`)
- `Vec<String>`: `2160 (2.11 KiB)` (`31.32%`) - `String`: field per string (`24 B` each x `90`)
- `Box<[Box<str>]>`: `1440 (1.41 KiB)` (`23.56%`) - `Box<str>`: field per string (`16 B` each x `90`)

One-time metadata (table object itself)

- `lite-strtab`: `32 B` (`StringTable<u32, u16>` struct itself; one per table, not per string)

### Read performance (YakuzaKiwami)

In this benchmark we sequentially read all of the 4,650 strings (238,109 bytes).

By:
- Getting the `&str` with `get` / `get_unchecked`
- Reading the `&str` data to compute a value (i.e. hashing).
  - This factors the other hidden costs such as memory alignment, etc. 

#### AHash payload read (`get` / `get_unchecked`)

Hashing the data with `AHash`, a realistic real world workload.

| Access          | Representation              | avg time (µs) | avg thrpt (GiB/s) |
| --------------- | --------------------------- | ------------- | ----------------- |
| `get`           | `Vec<String>`               | 13.561        | 16.352            |
| `get`           | `Box<[Box<str>]>`           | 13.002        | 17.056            |
| `get`           | `lite-strtab`               | 13.368        | 16.589            |
| `get`           | `lite-strtab (null-padded)` | 13.714        | 16.171            |
| `get_unchecked` | `Vec<String>`               | 13.448        | 16.490            |
| `get_unchecked` | `Box<[Box<str>]>`           | 12.812        | 17.308            |
| `get_unchecked` | `lite-strtab`               | 13.207        | 16.790            |
| `get_unchecked` | `lite-strtab (null-padded)` | 13.828        | 16.037            |

#### Byte-by-byte read (`get_u8` / `get_u8_unchecked`)

Summing bytes one at a time.

| Access             | Representation    | avg time (µs) | avg thrpt (GiB/s) |
| ------------------ | ----------------- | ------------- | ----------------- |
| `get_u8`           | `Vec<String>`     | 18.979        | 11.684            |
| `get_u8`           | `Box<[Box<str>]>` | 18.778        | 11.809            |
| `get_u8`           | `lite-strtab`     | 23.245        | 9.540             |
| `get_u8_unchecked` | `Vec<String>`     | 18.928        | 11.716            |
| `get_u8_unchecked` | `Box<[Box<str>]>` | 18.861        | 11.758            |
| `get_u8_unchecked` | `lite-strtab`     | 19.008        | 11.666            |

#### Chunked read (`get_usize` / `get_usize_unchecked`)

Reading data in `usize` chunks; then `u8` for the remainder.

| Access                | Representation    | avg time (µs) | avg thrpt (GiB/s) |
| --------------------- | ----------------- | ------------- | ----------------- |
| `get_usize`           | `Vec<String>`     | 8.219         | 26.982            |
| `get_usize`           | `Box<[Box<str>]>` | 8.234         | 26.932            |
| `get_usize`           | `lite-strtab`     | 8.038         | 27.590            |
| `get_usize_unchecked` | `Vec<String>`     | 8.167         | 27.154            |
| `get_usize_unchecked` | `Box<[Box<str>]>` | 8.402         | 26.393            |
| `get_usize_unchecked` | `lite-strtab`     | 8.042         | 27.575            |

#### Iterator (`iter` / `iter_u8` / `iter_usize`)

Using native iterators where available.

| Style   | Representation              | avg time (µs) | avg thrpt (GiB/s) |
| ------- | --------------------------- | ------------- | ----------------- |
| `ahash` | `Vec<String>`               | 12.387        | 17.902            |
| `ahash` | `Box<[Box<str>]>`           | 12.145        | 18.259            |
| `ahash` | `lite-strtab`               | 12.897        | 17.195            |
| `ahash` | `lite-strtab (null-padded)` | 14.774        | 15.010            |
| `u8`    | `Vec<String>`               | 17.998        | 12.321            |
| `u8`    | `Box<[Box<str>]>`           | 17.916        | 12.378            |
| `u8`    | `lite-strtab`               | 17.617        | 12.588            |
| `usize` | `Vec<String>`               | 7.751         | 28.610            |
| `usize` | `Box<[Box<str>]>`           | 7.845         | 28.268            |
| `usize` | `lite-strtab`               | 7.588         | 29.226            |

Reproduce with `cargo bench --bench my_benchmark`. Linux glibc. cargo 1.95.0-nightly (fe2f314ae 2026-01-30).

In summary, actual read performance on real data is within margin of error.

The overhead of looking up a string by ID is negligible.
Any difference you see is mostly due to run to run variation.

I've experimented with data alignment too, but saw no notable difference in practice
after aligning to `usize` boundaries to avoid reads across word boundaries.
There may be some in random access patterns; I've only benched sequential here.

### Assembly comparison

Instruction count to get `&str`, x86_64, release mode:

| Method                           | Instructions | Access Pattern                                           |
| -------------------------------- | ------------ | -------------------------------------------------------- |
| `lite-strtab::get`               | ~12          | bounds check → load 2 offsets → compute range → add base |
| `lite-strtab::get_unchecked`     | ~7           | load 2 offsets → compute range → add base                |
| `Vec<String>::get`               | ~8           | bounds check → load ptr from heap → deref for (ptr, len) |
| `Vec<String>::get_unchecked`     | ~5           | load ptr from heap → deref for (ptr, len)                |
| `Box<[Box<str>]>::get`           | ~7           | bounds check → load ptr → deref for (ptr, len)           |
| `Box<[Box<str>]>::get_unchecked` | ~4           | load ptr → deref for (ptr, len)                          |

Overhead of processing the data largely dominates; so the difference here is negligible.

[^1]: `RUSTFLAGS="-C target-cpu=native" cargo bench` is ~80% faster on 9950X3D; relative differences unchanged.

## License

MIT

[`Box<[Box<str>]>`]: alloc::boxed::Box
[`Box<[String]>`]: alloc::boxed::Box
[`StringId`]: crate::StringId
[`u32`]: prim@u32
