fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=src/windows.rc");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let version = std::env::var("CARGO_PKG_VERSION").unwrap();
        let numeric = format!("{},0", version.replace('.', ","));
        embed_resource::compile(
            "src/windows.rc",
            [
                format!("APP_VERSION=\"{version}\""),
                format!("APP_VERSION_NUM={numeric}"),
            ],
        )
        .manifest_required()
        .expect("Failed to embed Windows app resources");
    }
}
