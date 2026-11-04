// The kernel ELF is built into the stub (REDOX_KERNEL, see README.md).
fn main() {
    println!("cargo:rerun-if-env-changed=REDOX_KERNEL");
    println!("cargo:rerun-if-changed=link.ld");
    let kernel = std::env::var("REDOX_KERNEL").expect("set REDOX_KERNEL to the kernel ELF");
    println!("cargo:rerun-if-changed={kernel}");
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-search={dir}");
}
