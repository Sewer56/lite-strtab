use core::ffi::c_void;
use core::mem::{size_of, size_of_val};
use lite_strtab::{Global, StringId, StringTable, StringTableBuilder};
use std::io::Read;

mod output;

const DATASETS: &[(&str, &str)] = &[
    (
        "YakuzaKiwami",
        concat!(env!("CARGO_MANIFEST_DIR"), "/benches/data/YakuzaKiwami.zst"),
    ),
    (
        "EnvKeys",
        concat!(env!("CARGO_MANIFEST_DIR"), "/benches/data/EnvKeys.zst"),
    ),
    (
        "ApiUrls",
        concat!(env!("CARGO_MANIFEST_DIR"), "/benches/data/ApiUrls.zst"),
    ),
];

struct Dataset {
    entries: Vec<String>,
    total_bytes: usize,
}

#[derive(Clone, Copy, Default)]
struct RepresentationTotals {
    heap_usable_bytes: usize,
    distributed_reference_bytes: usize,
    fixed_inline_bytes: usize,
}

impl RepresentationTotals {
    fn total(&self) -> usize {
        self.heap_usable_bytes
            .saturating_add(self.distributed_reference_bytes)
            .saturating_add(self.fixed_inline_bytes)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ComponentKind {
    Heap,
    DistributedReferences,
    FixedInline,
}

struct ComponentMeasurement {
    kind: ComponentKind,
    name: String,
    details: String,
    bytes: usize,
}

struct RepresentationMeasurement {
    name: &'static str,
    components: Vec<ComponentMeasurement>,
}

impl RepresentationMeasurement {
    fn totals(&self) -> RepresentationTotals {
        let mut totals = RepresentationTotals::default();

        for component in &self.components {
            match component.kind {
                ComponentKind::Heap => {
                    totals.heap_usable_bytes =
                        totals.heap_usable_bytes.saturating_add(component.bytes);
                }
                ComponentKind::DistributedReferences => {
                    totals.distributed_reference_bytes = totals
                        .distributed_reference_bytes
                        .saturating_add(component.bytes);
                }
                ComponentKind::FixedInline => {
                    totals.fixed_inline_bytes =
                        totals.fixed_inline_bytes.saturating_add(component.bytes);
                }
            }
        }

        totals
    }

    fn components_by_kind(
        &self,
        kind: ComponentKind,
    ) -> impl Iterator<Item = &ComponentMeasurement> {
        self.components
            .iter()
            .filter(move |component| component.kind == kind)
    }
}

pub fn run() {
    output::print_overview();

    for &(dataset_name, dataset_path) in DATASETS {
        let dataset = load_dataset(dataset_path);
        print_memory_report_for_dataset(dataset_name, &dataset);
    }
}

fn load_dataset(dataset_path: &str) -> Dataset {
    let file = std::fs::File::open(dataset_path).expect("failed to open dataset");
    let mut decoder =
        zstd::stream::read::Decoder::new(file).expect("failed to create zstd decoder for dataset");

    let mut decompressed = String::new();
    decoder
        .read_to_string(&mut decompressed)
        .expect("failed to decompress dataset");

    let mut entries = Vec::new();
    let mut total_bytes = 0usize;

    for line in decompressed.lines() {
        total_bytes = total_bytes.saturating_add(line.len());
        entries.push(line.to_owned());
    }

    assert!(!entries.is_empty(), "dataset did not contain any values");

    Dataset {
        entries,
        total_bytes,
    }
}

fn build_table(entries: &[String], total_bytes: usize) -> StringTable<u32, u16> {
    let mut builder =
        StringTableBuilder::<u32, u16>::with_capacity_in(entries.len(), total_bytes, Global);
    for value in entries {
        builder.try_push(value).expect("failed to insert value");
    }
    builder.build()
}

fn print_memory_report_for_dataset(dataset_name: &str, dataset: &Dataset) {
    let reports = [
        measure_lite_strtab_bytes(&dataset.entries, dataset.total_bytes),
        measure_string_fields_bytes(&dataset.entries),
        measure_boxed_str_fields_bytes(&dataset.entries),
    ];

    output::print_report(
        dataset_name,
        dataset.entries.len(),
        dataset.total_bytes,
        &reports,
    );
}

fn measure_lite_strtab_bytes(entries: &[String], total_bytes: usize) -> RepresentationMeasurement {
    let table = build_table(entries, total_bytes);
    let count = entries.len();
    let id_bytes = size_of::<StringId<u16>>().saturating_mul(count);
    let id_size = size_of::<StringId<u16>>();

    RepresentationMeasurement {
        name: "lite-strtab",
        components: vec![
            heap_component(
                "`StringTable<u32, u16>` byte buffer",
                usable_size_for_slice(table.as_bytes()),
                "concatenated UTF-8 string payload data",
            ),
            heap_component(
                "`StringTable<u32, u16>` offsets buffer",
                usable_size_for_slice(table.offsets()),
                "`u32` offsets into the shared byte buffer",
            ),
            references_component(
                "`StringId<u16>`",
                id_bytes,
                format!("field per string ({id_size} B each x {count})"),
            ),
            fixed_inline_component(
                "`StringTable<u32, u16>` struct itself",
                size_of_val(&table),
                "single table struct stored inline",
            ),
        ],
    }
}

fn measure_string_fields_bytes(entries: &[String]) -> RepresentationMeasurement {
    let mut payload_heap_usable_bytes = 0usize;
    for value in entries {
        if value.capacity() != 0 {
            payload_heap_usable_bytes = payload_heap_usable_bytes
                .saturating_add(usable_size_for_raw_ptr(value.as_ptr().cast()));
        }
    }

    let count = entries.len();
    let field_size = size_of::<String>();
    let reference_bytes = field_size.saturating_mul(count);

    RepresentationMeasurement {
        name: "Vec<String>",
        components: vec![
            heap_component(
                "`String` payload allocations",
                payload_heap_usable_bytes,
                "one UTF-8 allocation per string",
            ),
            references_component(
                "`String`",
                reference_bytes,
                format!("field per string ({field_size} B each x {count})"),
            ),
        ],
    }
}

fn measure_boxed_str_fields_bytes(entries: &[String]) -> RepresentationMeasurement {
    let mut payload_heap_usable_bytes = 0usize;
    for value in entries {
        let boxed = value.clone().into_boxed_str();
        if !boxed.is_empty() {
            payload_heap_usable_bytes = payload_heap_usable_bytes
                .saturating_add(usable_size_for_raw_ptr(boxed.as_ptr().cast()));
        }
    }

    let count = entries.len();
    let field_size = size_of::<Box<str>>();
    let reference_bytes = field_size.saturating_mul(count);

    RepresentationMeasurement {
        name: "Box<[Box<str>]>",
        components: vec![
            heap_component(
                "`Box<str>` payload allocations",
                payload_heap_usable_bytes,
                "one UTF-8 allocation per string",
            ),
            references_component(
                "`Box<str>`",
                reference_bytes,
                format!("field per string ({field_size} B each x {count})"),
            ),
        ],
    }
}

fn heap_component(name: &str, bytes: usize, details: &str) -> ComponentMeasurement {
    ComponentMeasurement {
        kind: ComponentKind::Heap,
        name: name.to_owned(),
        details: details.to_owned(),
        bytes,
    }
}

fn references_component(name: &str, bytes: usize, details: String) -> ComponentMeasurement {
    ComponentMeasurement {
        kind: ComponentKind::DistributedReferences,
        name: name.to_owned(),
        details,
        bytes,
    }
}

fn fixed_inline_component(name: &str, bytes: usize, details: &str) -> ComponentMeasurement {
    ComponentMeasurement {
        kind: ComponentKind::FixedInline,
        name: name.to_owned(),
        details: details.to_owned(),
        bytes,
    }
}

unsafe extern "C" {
    fn malloc_usable_size(ptr: *const c_void) -> usize;
}

fn usable_size_for_raw_ptr(ptr: *const c_void) -> usize {
    // SAFETY: All call sites only pass pointers to live heap allocations made by
    // the process allocator, and never pass null.
    unsafe { malloc_usable_size(ptr) }
}

fn usable_size_for_slice<T>(slice: &[T]) -> usize {
    if slice.is_empty() {
        return 0;
    }

    usable_size_for_raw_ptr(slice.as_ptr().cast())
}
