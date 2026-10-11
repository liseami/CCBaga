// [INPUT]: Objective-C 原生桥与 Tauri 配置。
// [OUTPUT]: 链接 macOS 剪贴板、授权、钥匙串、定位、代理、SSH 文件选择与 LaunchServices 的桌面可执行文件。
// [POS]: 原生宿主的构建边界。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
fn main() {
    cc::Build::new()
        .file("native/Clipboard.m")
        .file("native/Cleanup.m")
        .file("native/Keychain.m")
        .file("native/System.m")
        .file("native/Browser.m")
        .file("native/SshKeys.m")
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
    println!("cargo:rerun-if-changed=native/System.m");
    println!("cargo:rerun-if-changed=native/Browser.m");
    println!("cargo:rerun-if-changed=native/SshKeys.m");
    tauri_build::build();
}
