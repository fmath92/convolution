#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::init();
    convolution_wasm::main()
}

#[cfg(target_arch = "wasm32")]
fn main() {
    convolution_wasm::main();
}
