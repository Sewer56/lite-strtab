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

See [full crate docs](src/lite-strtab/README.MD) for more details.

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

| Representation    | Total               | Heap allocations    | Distributed fields  | vs lite-strtab |
| ----------------- | ------------------- | ------------------- | ------------------- | -------------- |
| `lite-strtab`     | 266068 (259.83 KiB) | 256736 (250.72 KiB) | 9300 (9.08 KiB)     | 1.00x          |
| `Vec<String>`     | 384240 (375.23 KiB) | 272640 (266.25 KiB) | 111600 (108.98 KiB) | 1.44x          |
| `Box<[Box<str>]>` | 346928 (338.80 KiB) | 272528 (266.14 KiB) | 74400 (72.66 KiB)   | 1.30x          |

**EnvKeys** (109 environment variable names, 1,795 bytes):

| Representation    | Total           | Heap allocations | Distributed fields | vs lite-strtab |
| ----------------- | --------------- | ---------------- | ------------------ | -------------- |
| `lite-strtab`     | 2490 (2.43 KiB) | 2240 (2.19 KiB)  | 218 B              | 1.00x          |
| `Vec<String>`     | 5504 (5.38 KiB) | 2888 (2.82 KiB)  | 2616 (2.55 KiB)    | 2.21x          |
| `Box<[Box<str>]>` | 4472 (4.37 KiB) | 2728 (2.66 KiB)  | 1744 (1.70 KiB)    | 1.80x          |

**ApiUrls** (90 API endpoint URLs, 3,970 bytes):

| Representation    | Total           | Heap allocations | Distributed fields | vs lite-strtab |
| ----------------- | --------------- | ---------------- | ------------------ | -------------- |
| `lite-strtab`     | 4564 (4.46 KiB) | 4352 (4.25 KiB)  | 180 B              | 1.00x          |
| `Vec<String>`     | 6896 (6.73 KiB) | 4736 (4.62 KiB)  | 2160 (2.11 KiB)    | 1.51x          |
| `Box<[Box<str>]>` | 6112 (5.97 KiB) | 4672 (4.56 KiB)  | 1440 (1.41 KiB)    | 1.34x          |

For complete comparisons across all datasets and full breakdown details, see [full crate docs](src/lite-strtab/README.MD#memory-comparison-results).

**Summary**: `lite-strtab` uses **20-55% less memory** than the standard methods. Generally the smaller the strings, the better the savings.

## License

MIT
