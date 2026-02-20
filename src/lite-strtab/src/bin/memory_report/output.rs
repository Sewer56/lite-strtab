use super::{ComponentKind, RepresentationMeasurement};

struct SummaryDisplayRow {
    representation: String,
    total: String,
    heap: String,
    references: String,
    ratio: String,
}

pub(super) fn print_overview() {
    println!("How to read these tables:");
    println!("- `Total` = `Heap allocations` + `Distributed fields` + `One-time metadata`");
    println!(
        "- `Distributed fields` = string references distributed across fields/structs (e.g. `String`, `Box<str>`, `StringId<u16>`)"
    );
    println!("- in these results, `lite-strtab` uses `StringId<u16>`");
    println!(
        "- `One-time metadata` is the `StringTable<u32, u16>` struct itself (counted once per table)"
    );
}

pub(super) fn print_report(
    dataset_name: &str,
    entry_count: usize,
    total_bytes: usize,
    reports: &[RepresentationMeasurement],
) {
    println!("\nMemory Comparison: {dataset_name} (malloc_usable_size)");
    println!("dataset: {entry_count} entries, {total_bytes} UTF-8 bytes");

    print_summary_table(reports);
    print_heap_breakdown_tree(reports);
    print_distributed_references(reports);
    print_one_time_metadata_note(reports);
}

fn print_summary_table(reports: &[RepresentationMeasurement]) {
    if reports.is_empty() {
        println!("\nSummary");
        println!("- no representations measured");
        return;
    }

    let lite_total = reports[0].totals().total();
    let rows = reports
        .iter()
        .map(|report| {
            let totals = report.totals();
            SummaryDisplayRow {
                representation: markdown_code(report.name),
                total: format_total_cell(totals.total()),
                heap: format_total_cell(totals.heap_usable_bytes),
                references: format_total_cell(totals.distributed_reference_bytes),
                ratio: format_ratio(totals.total(), lite_total),
            }
        })
        .collect::<Vec<_>>();

    let representation_header = "Representation";
    let total_header = "Total";
    let heap_header = "Heap allocations";
    let references_header = "Distributed fields";
    let ratio_header = "vs lite-strtab";

    let representation_width = rows
        .iter()
        .map(|row| row.representation.len())
        .max()
        .unwrap_or(0)
        .max(representation_header.len());
    let total_width = rows
        .iter()
        .map(|row| row.total.len())
        .max()
        .unwrap_or(0)
        .max(total_header.len());
    let heap_width = rows
        .iter()
        .map(|row| row.heap.len())
        .max()
        .unwrap_or(0)
        .max(heap_header.len());
    let references_width = rows
        .iter()
        .map(|row| row.references.len())
        .max()
        .unwrap_or(0)
        .max(references_header.len());
    let ratio_width = rows
        .iter()
        .map(|row| row.ratio.len())
        .max()
        .unwrap_or(0)
        .max(ratio_header.len());

    println!("\nSummary");
    println!(
        "| {:<representation_width$} | {:<total_width$} | {:<heap_width$} | {:<references_width$} | {:<ratio_width$} |",
        representation_header,
        total_header,
        heap_header,
        references_header,
        ratio_header,
    );
    println!(
        "|{:-<representation_width$}--|{:-<total_width$}--|{:-<heap_width$}--|{:-<references_width$}--|{:-<ratio_width$}--|",
        "", "", "", "", ""
    );

    for row in &rows {
        println!(
            "| {:<representation_width$} | {:>total_width$} | {:>heap_width$} | {:>references_width$} | {:>ratio_width$} |",
            row.representation,
            row.total,
            row.heap,
            row.references,
            row.ratio,
        );
    }
}

fn print_one_time_metadata_note(reports: &[RepresentationMeasurement]) {
    if reports.is_empty() {
        return;
    }

    let mut non_zero = reports
        .iter()
        .filter_map(|report| {
            let bytes = report.totals().fixed_inline_bytes;
            if bytes == 0 {
                return None;
            }

            Some((report, bytes))
        })
        .collect::<Vec<_>>();

    if non_zero.is_empty() {
        return;
    }

    println!("\nOne-time metadata (table object itself)");

    non_zero.sort_by(|(left_report, _), (right_report, _)| left_report.name.cmp(right_report.name));
    for (report, bytes) in non_zero {
        let metadata_label = report
            .components_by_kind(ComponentKind::FixedInline)
            .next()
            .map(|component| component.name.as_str())
            .unwrap_or("table object");

        println!(
            "- {}: {} ({}; one per table, not per string)",
            markdown_code(report.name),
            format_total_cell(bytes),
            metadata_label
        );
    }
}

fn print_heap_breakdown_tree(reports: &[RepresentationMeasurement]) {
    println!("\nHeap allocations (tree)");

    if reports.is_empty() {
        println!("- no representations measured");
        return;
    }

    for report in reports {
        let totals = report.totals();
        let representation_total = totals.total();
        let heap_total = totals.heap_usable_bytes;

        println!(
            "- {}: {} ({})",
            markdown_code(report.name),
            format_total_cell(heap_total),
            format_share(heap_total, representation_total)
        );

        if heap_total == 0 {
            println!("  - no heap allocations");
            continue;
        }

        for component in report.components_by_kind(ComponentKind::Heap) {
            println!(
                "  - {}: {} ({} of heap) - {}",
                component.name,
                format_total_cell(component.bytes),
                format_share(component.bytes, heap_total),
                component.details
            );
        }
    }
}

fn print_distributed_references(reports: &[RepresentationMeasurement]) {
    println!("\nDistributed fields (per-string handles)");

    if reports.is_empty() {
        println!("- no representations measured");
        return;
    }

    for report in reports {
        let totals = report.totals();
        let representation_total = totals.total();
        let reference_total = totals.distributed_reference_bytes;

        if reference_total == 0 {
            println!(
                "- {}: {} ({})",
                markdown_code(report.name),
                format_total_cell(reference_total),
                format_share(reference_total, representation_total)
            );
            continue;
        }

        if let Some(component) = report
            .components_by_kind(ComponentKind::DistributedReferences)
            .next()
        {
            println!(
                "- {}: {} ({}) - {}: {}",
                markdown_code(report.name),
                format_total_cell(reference_total),
                format_share(reference_total, representation_total),
                component.name,
                component.details,
            );
        } else {
            println!(
                "- {}: {} ({})",
                markdown_code(report.name),
                format_total_cell(reference_total),
                format_share(reference_total, representation_total)
            );
        }
    }
}

fn format_total_cell(bytes: usize) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }

    format!("{bytes} ({})", format_bytes(bytes))
}

fn markdown_code(value: &str) -> String {
    format!("`{value}`")
}

fn format_ratio(value: usize, baseline: usize) -> String {
    if baseline == 0 {
        return "n/a".to_owned();
    }

    format!("{:.2}x", (value as f64) / (baseline as f64))
}

fn format_share(component_total: usize, representation_total: usize) -> String {
    if representation_total == 0 {
        return "n/a".to_owned();
    }

    format!(
        "{:.2}%",
        ((component_total as f64) * 100.0) / (representation_total as f64)
    )
}

fn format_bytes(bytes: usize) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;

    if bytes < 1024 {
        return format!("{bytes} B");
    }

    if bytes < (MIB as usize) {
        return format!("{:.2} KiB", (bytes as f64) / KIB);
    }

    format!("{:.2} MiB", (bytes as f64) / MIB)
}
