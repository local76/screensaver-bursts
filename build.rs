use std::path::Path;

#[path = "build_helpers.rs"]
mod build_helpers;

fn main() {
    let icon_file_path = "assets/scene-bursts.ico";
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") { return; }
    if !Path::new(icon_file_path).exists() { return; }

    let package_name = std::env::var("CARGO_PKG_NAME").unwrap_or_default();
    let resource_script_path = build_helpers::write_brand_rc(
        "build/windows_resource.rc",
        icon_file_path,
        &package_name,
        build_helpers::DEFAULT_PRODUCT_NAME,
        build_helpers::DEFAULT_COMPANY_NAME,
        build_helpers::DEFAULT_LEGAL_COPYRIGHT,
    );
    embed_resource::compile(&resource_script_path, embed_resource::NONE);
}
