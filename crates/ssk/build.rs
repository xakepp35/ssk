fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    for path in [
        "src",
        "../ssk-client/src",
        "../ssk-net/src",
        "../ssk-sim/src",
        "../../Cargo.toml",
        "../../.git/HEAD",
        "../../.git/refs",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
    if let Ok(output) = std::process::Command::new("git")
        .args(["describe", "--always", "--dirty", "--abbrev=12"])
        .output()
    {
        if output.status.success() {
            println!(
                "cargo:rustc-env=SSK_BUILD_REVISION={}",
                String::from_utf8_lossy(&output.stdout).trim()
            );
        }
    }
    // quad-snd only needs the ALSA ABI; desktop runtime packages contain it.
    // Supply a local linker alias when the distribution's -dev symlink is absent.
    #[cfg(target_os = "linux")]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
        let triple = match arch.as_str() {
            "x86_64" => "x86_64-linux-gnu",
            "aarch64" => "aarch64-linux-gnu",
            _ => return,
        };
        for directory in [
            format!("/usr/lib/{triple}"),
            format!("/lib/{triple}"),
            "/usr/lib64".into(),
            "/usr/lib".into(),
        ] {
            let runtime = std::path::Path::new(&directory).join("libasound.so.2");
            if runtime.exists() {
                let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
                let alias = out.join("libasound.so");
                if !alias.exists() {
                    std::os::unix::fs::symlink(runtime, &alias)
                        .expect("create local ALSA linker alias");
                }
                println!("cargo:rustc-link-search=native={}", out.display());
                break;
            }
        }
    }
}
