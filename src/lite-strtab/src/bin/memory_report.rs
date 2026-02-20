#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("memory_report only supports Linux (uses malloc_usable_size)");
    std::process::exit(1);
}

#[cfg(target_os = "linux")]
#[path = "memory_report/linux.rs"]
mod linux;

#[cfg(target_os = "linux")]
fn main() {
    linux::run();
}
