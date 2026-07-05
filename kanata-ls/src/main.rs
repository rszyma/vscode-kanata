#[cfg(not(target_arch = "wasm32"))]
mod main_native;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> anyhow::Result<()> {
    if let Err(e) = main_native::main() {
        eprintln!("kanata-ls: error: {e}")
    };
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() {
    panic!("This entrypoint only supports native targets.");
}
