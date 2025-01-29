fn main() {
    sidex_build_rs::configure()
        .with_bundle(".")
        .generate()
        .expect("Failed to generate code for Sidex bundles.");
}
