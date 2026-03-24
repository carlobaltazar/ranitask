fn main() {
    // Resource embedding requires windres (GNU) or rc.exe (MSVC/Windows SDK).
    // Only attempt if the tool is available; skip gracefully otherwise.
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "windows" {
        return;
    }

    let mut res = winres::WindowsResource::new();
    res.set_manifest_file("assets/ranitask.manifest");

    if std::path::Path::new("assets/ranitask.ico").exists() {
        res.set_icon("assets/ranitask.ico");
    }

    res.set("ProductName", "RaniTask");
    res.set("FileDescription", "RaniTask - Input Sequence Recorder");
    res.set("LegalCopyright", "Copyright (c) 2026");
    res.set_version_info(winres::VersionInfo::PRODUCTVERSION, 0x0001_0000_0000_0000);
    res.set_version_info(winres::VersionInfo::FILEVERSION, 0x0001_0000_0000_0000);

    match res.compile() {
        Ok(_) => {}
        Err(e) => {
            // Print warning but don't fail the build — resources are optional
            println!("cargo:warning=Could not embed Windows resources: {e}");
            println!("cargo:warning=Install MinGW (windres) or Windows SDK (rc.exe) to embed icon/manifest.");
        }
    }
}
