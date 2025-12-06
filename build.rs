fn main() {
    // This runs BEFORE your app is compiled to bake the icon into the .exe
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();
        // Ensure this path is correct relative to Cargo.toml
        res.set_icon("wix/Product.ico");
        res.compile().unwrap();
    }
}