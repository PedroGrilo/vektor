fn main() {
    // Vektor changes HKLM, services, power plans and drivers, so it always runs
    // elevated (one UAC prompt at start) instead of shipping a privileged service.
    let attrs = tauri_build::Attributes::new().windows_attributes(
        tauri_build::WindowsAttributes::new().app_manifest(include_str!("vektor.exe.manifest")),
    );
    tauri_build::try_build(attrs).expect("failed to run tauri-build");
}
