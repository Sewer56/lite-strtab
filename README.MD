# lite-strtab

[![Crates.io](https://img.shields.io/crates/v/lite-strtab.svg)](https://crates.io/crates/lite-strtab)
[![Docs.rs](https://docs.rs/lite-strtab/badge.svg)](https://docs.rs/lite-strtab)
[![CI](https://github.com/Sewer56/lite-strtab/actions/workflows/rust.yml/badge.svg)](https://github.com/Sewer56/lite-strtab/actions)

Crate for storing many immutable strings in one buffer with minimal resource usage.

Designed to minimize:

- memory overhead (single byte buffer + compact offsets)
- CPU overhead (cheap index-based lookups)
- binary size overhead (no panics on insertion, avoiding backtrace overhead)

See [full crate docs](src/lite-strtab/README.MD) for more details.
Below is just a sneak peek.

## Quick start

```toml
[dependencies]
lite-strtab = "0.1.0"
```

```rust
use lite_strtab::StringTableBuilder;

let mut builder = StringTableBuilder::new();
let id = builder.try_push("hello").unwrap();
let table = builder.build();

assert_eq!(table.get(id), Some("hello"));
```

***See [full crate docs](src/lite-strtab/README.MD)*** for more details.
This page is a summary.

## Memory Usage

Measured on Linux with glibc malloc using `malloc_usable_size`
(capturing allocator block sizes including alignment overhead).

How to read these tables:

- `Total` = `Heap allocations` + `Distributed fields` + `One-time metadata`
- `Distributed fields` = string references distributed across fields/structs (e.g. `String`, `Box<str>`, `StringId<u16>`)
- in these results, `lite-strtab` uses `StringId<u16>`

This keeps the comparison close to real application layouts while still keeping the baseline rows readable.

### Results

**YakuzaKiwami** (4,650 game file paths, 238,109 bytes):

| Representation              | Total               | Heap allocations    | Distributed fields  | vs lite-strtab |
| --------------------------- | ------------------- | ------------------- | ------------------- | -------------- |
| `lite-strtab`               | 266068 (259.83 KiB) | 256736 (250.72 KiB) | 9300 (9.08 KiB)     | 1.00x          |
| `lite-strtab (null-padded)` | 270708 (264.36 KiB) | 261376 (255.25 KiB) | 9300 (9.08 KiB)     | 1.02x          |
| `Vec<String>`               | 384240 (375.23 KiB) | 272640 (266.25 KiB) | 111600 (108.98 KiB) | 1.44x          |
| `Box<[Box<str>]>`           | 346928 (338.80 KiB) | 272528 (266.14 KiB) | 74400 (72.66 KiB)   | 1.30x          |

**EnvKeys** (109 environment variable names, 1,795 bytes):

| Representation              | Total           | Heap allocations | Distributed fields | vs lite-strtab |
| --------------------------- | --------------- | ---------------- | ------------------ | -------------- |
| `lite-strtab`               | 2490 (2.43 KiB) | 2240 (2.19 KiB)  | 218 B              | 1.00x          |
| `lite-strtab (null-padded)` | 2602 (2.54 KiB) | 2352 (2.30 KiB)  | 218 B              | 1.04x          |
| `Vec<String>`               | 5440 (5.31 KiB) | 2824 (2.76 KiB)  | 2616 (2.55 KiB)    | 2.18x          |
| `Box<[Box<str>]>`           | 4472 (4.37 KiB) | 2728 (2.66 KiB)  | 1744 (1.70 KiB)    | 1.80x          |

**ApiUrls** (90 API endpoint URLs, 3,970 bytes):

| Representation              | Total           | Heap allocations | Distributed fields | vs lite-strtab |
| --------------------------- | --------------- | ---------------- | ------------------ | -------------- |
| `lite-strtab`               | 4564 (4.46 KiB) | 4352 (4.25 KiB)  | 180 B              | 1.00x          |
| `lite-strtab (null-padded)` | 4660 (4.55 KiB) | 4448 (4.34 KiB)  | 180 B              | 1.02x          |
| `Vec<String>`               | 6896 (6.73 KiB) | 4736 (4.62 KiB)  | 2160 (2.11 KiB)    | 1.51x          |
| `Box<[Box<str>]>`           | 6112 (5.97 KiB) | 4672 (4.56 KiB)  | 1440 (1.41 KiB)    | 1.34x          |

For complete comparisons across all datasets and full breakdown details, see [full crate docs](src/lite-strtab/README.MD#memory-comparison-results).

**Summary**: `lite-strtab` uses **20-55% less memory** than the standard methods. Generally the smaller the strings, the better the savings.

## Read Performance (YakuzaKiwami)

In this benchmark we sequentially read all of the 4,650 strings (238,109 bytes)
and hash them with `AHash` to simulate a realistic workload that involves reading the strings.

### AHash payload read (`get` / `get_unchecked`)

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
