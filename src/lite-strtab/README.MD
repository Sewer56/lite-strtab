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

- `Memory`: one UTF-8 byte buffer plus one compact offset table
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

| Representation    | Total               | Heap allocations    | Distributed fields  | vs lite-strtab |
| ----------------- | ------------------- | ------------------- | ------------------- | -------------- |
| `lite-strtab`     | 266068 (259.83 KiB) | 256736 (250.72 KiB) | 9300 (9.08 KiB)     | 1.00x          |
| `Vec<String>`     | 384240 (375.23 KiB) | 272640 (266.25 KiB) | 111600 (108.98 KiB) | 1.44x          |
| `Box<[Box<str>]>` | 346928 (338.80 KiB) | 272528 (266.14 KiB) | 74400 (72.66 KiB)   | 1.30x          |

Heap allocations (tree)

- `lite-strtab`: `256736 (250.72 KiB)` (`96.49%`)
  - `StringTable<u32, u16>` byte buffer: `238120 (232.54 KiB)` (`92.75%` of heap) - concatenated UTF-8 string payload data
  - `StringTable<u32, u16>` offsets buffer: `18616 (18.18 KiB)` (`7.25%` of heap) - `u32` offsets into the shared byte buffer
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

| Representation    | Total           | Heap allocations | Distributed fields | vs lite-strtab |
| ----------------- | --------------- | ---------------- | ------------------ | -------------- |
| `lite-strtab`     | 2490 (2.43 KiB) | 2240 (2.19 KiB)  | 218 B              | 1.00x          |
| `Vec<String>`     | 5504 (5.38 KiB) | 2888 (2.82 KiB)  | 2616 (2.55 KiB)    | 2.21x          |
| `Box<[Box<str>]>` | 4472 (4.37 KiB) | 2728 (2.66 KiB)  | 1744 (1.70 KiB)    | 1.80x          |

Heap allocations (tree)

- `lite-strtab`: `2240 (2.19 KiB)` (`89.96%`)
  - `StringTable<u32, u16>` byte buffer: `1800 (1.76 KiB)` (`80.36%` of heap) - concatenated UTF-8 string payload data
  - `StringTable<u32, u16>` offsets buffer: `440 B` (`19.64%` of heap) - `u32` offsets into the shared byte buffer
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

| Representation    | Total           | Heap allocations | Distributed fields | vs lite-strtab |
| ----------------- | --------------- | ---------------- | ------------------ | -------------- |
| `lite-strtab`     | 4564 (4.46 KiB) | 4352 (4.25 KiB)  | 180 B              | 1.00x          |
| `Vec<String>`     | 6896 (6.73 KiB) | 4736 (4.62 KiB)  | 2160 (2.11 KiB)    | 1.51x          |
| `Box<[Box<str>]>` | 6112 (5.97 KiB) | 4672 (4.56 KiB)  | 1440 (1.41 KiB)    | 1.34x          |

Heap allocations (tree)

- `lite-strtab`: `4352 (4.25 KiB)` (`95.35%`)
  - `StringTable<u32, u16>` byte buffer: `3976 (3.88 KiB)` (`91.36%` of heap) - concatenated UTF-8 string payload data
  - `StringTable<u32, u16>` offsets buffer: `376 B` (`8.64%` of heap) - `u32` offsets into the shared byte buffer
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

## License

MIT

[`Box<[Box<str>]>`]: alloc::boxed::Box
[`Box<[String]>`]: alloc::boxed::Box
[`StringId`]: crate::StringId
[`u32`]: prim@u32
