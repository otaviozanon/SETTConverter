fn main() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let mut resource = winresource::WindowsResource::new();
        resource.set_manifest_file("app.manifest");
        if std::path::Path::new("assets/icon.ico").exists() {
            resource.set_icon("assets/icon.ico");
        }
        if let Err(error) = resource.compile() {
            println!("cargo:warning=manifesto nao compilado: {error}");
        }
    }
    println!("cargo:rerun-if-changed=app.manifest");
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=Cargo.toml");
}
