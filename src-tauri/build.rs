// [INPUT]: 目标平台、macOS Objective-C 原生/身份凭据桥与 Tauri 配置。
// [OUTPUT]: 仅 macOS 编译链接系统桥；Linux/Windows 使用 Rust 平台边界，各自加载 Tauri 平台配置。
// [POS]: 原生宿主的构建边界。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("native/Clipboard.m")
            .file("native/Cleanup.m")
            .file("native/Keychain.m")
            .file("native/Identity.m")
            .file("native/System.m")
            .file("native/Browser.m")
            .flag("-fobjc-arc")
            .compile("selection");
        for framework in [
            "AppKit",
            "UniformTypeIdentifiers",
            "ApplicationServices",
            "Security",
            "LocalAuthentication",
            "CoreLocation",
            "SystemConfiguration",
            "CoreServices",
        ] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
        println!("cargo:rerun-if-changed=native/Clipboard.m");
        println!("cargo:rerun-if-changed=native/Cleanup.m");
        println!("cargo:rerun-if-changed=native/Keychain.m");
        println!("cargo:rerun-if-changed=native/Identity.m");
        println!("cargo:rerun-if-changed=native/System.m");
        println!("cargo:rerun-if-changed=native/Browser.m");
    }
    tauri_build::build();
}
