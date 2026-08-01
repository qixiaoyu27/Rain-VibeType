fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("src/platform_macos.m")
            .flag("-fobjc-arc")
            .flag("-fblocks")
            .compile("rain_platform_macos");
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=ApplicationServices");
        println!("cargo:rustc-link-lib=framework=CoreAudio");
        println!("cargo:rustc-link-lib=framework=AudioToolbox");
        println!("cargo:rerun-if-changed=src/platform_macos.m");
    }
    tauri_build::build()
}
