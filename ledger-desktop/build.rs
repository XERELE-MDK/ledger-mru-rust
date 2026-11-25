use std::{env, path::PathBuf, process::Command};

fn main() {
    let config = slint_build::CompilerConfiguration::new().with_style("fluent".into());
    slint_build::compile_with_config("ui/app.slint", config).expect("Slint UI compiles");
    link_old_glibc_math();
}

/// x86_64 glibc only: see glibc_compat.c (lets the .deb/.rpm run on older distros).
fn link_old_glibc_math() {
    let var = |k: &str| env::var(k).unwrap_or_default();
    if var("CARGO_CFG_TARGET_ARCH") != "x86_64"
        || var("CARGO_CFG_TARGET_OS") != "linux"
        || var("CARGO_CFG_TARGET_ENV") != "gnu"
    {
        return;
    }
    println!("cargo:rerun-if-changed=glibc_compat.c");
    let obj = PathBuf::from(var("OUT_DIR")).join("glibc_compat.o");
    let cc = env::var("CC").unwrap_or_else(|_| "cc".into());
    let ok = Command::new(&cc)
        .args(["-c", "-O2", "-fPIC", "glibc_compat.c", "-o"])
        .arg(&obj)
        .status()
        .is_ok_and(|s| s.success());
    if !ok {
        // No C compiler: the app still builds, it just needs a recent glibc.
        println!("cargo:warning=glibc_compat.c not compiled ({cc} failed): binary needs a recent glibc");
        return;
    }
    println!("cargo:rustc-link-arg-bins={}", obj.display());
    println!("cargo:rustc-link-arg-bins=-Wl,--wrap=acosf,--wrap=atan2f,--wrap=hypotf,--wrap=pidfd_spawnp,--wrap=pidfd_getpid");
}
