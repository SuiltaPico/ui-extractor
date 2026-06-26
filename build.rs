use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android")
        && std::env::var("CARGO_FEATURE_BACKEND_MNN").is_ok()
        && std::env::var("MNN_LINK").as_deref() != Ok("dylib")
    {
        println!("cargo:rustc-link-lib=c++_shared");
    }

    link_infer_core();
}

fn link_infer_core() {
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "release".into());
    let target = std::env::var("TARGET").unwrap();
    let host = std::env::var("HOST").unwrap_or_default();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let local_infer_core_dir = resolve_local_infer_core_dir(&manifest_dir);

    let infer_root = local_infer_core_dir.join("target");

    let mut lib_dir = std::env::var("INFER_CORE_LIB_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| resolve_infer_core_lib_dir(&infer_root, &target, &profile));

    if !has_any_linkable_artifact(&lib_dir, &target_os) && std::env::var("INFER_CORE_LIB_DIR").is_err() {
        build_local_infer_core_ffi(&local_infer_core_dir, &profile, &target, &host);
        lib_dir = resolve_infer_core_lib_dir(&infer_root, &target, &profile);
    }

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    if target_os == "windows" {
        let dll_import = lib_dir.join("infer_core.dll.lib");
        let static_lib = lib_dir.join("infer_core.lib");
        if dll_import.is_file() {
            println!("cargo:rustc-link-lib=infer_core.dll");
        } else if static_lib.is_file() {
            println!("cargo:rustc-link-lib=static=infer_core");
        } else {
            panic!(
                "cannot find infer_core import/static library in {}; expected infer_core.dll.lib or infer_core.lib",
                lib_dir.display()
            );
        }
        println!("cargo:rerun-if-changed={}", dll_import.display());
        println!("cargo:rerun-if-changed={}", static_lib.display());
    } else if target_os == "macos" {
        let dylib = lib_dir.join("libinfer_core.dylib");
        let static_lib = lib_dir.join("libinfer_core.a");
        if dylib.is_file() {
            println!("cargo:rustc-link-lib=dylib=infer_core");
        } else if static_lib.is_file() {
            println!("cargo:rustc-link-lib=static=infer_core");
        } else {
            panic!(
                "cannot find infer_core library in {}; expected libinfer_core.dylib or libinfer_core.a",
                lib_dir.display()
            );
        }
        println!("cargo:rerun-if-changed={}", dylib.display());
        println!("cargo:rerun-if-changed={}", static_lib.display());
    } else {
        let so = lib_dir.join("libinfer_core.so");
        let static_lib = lib_dir.join("libinfer_core.a");
        if so.is_file() {
            println!("cargo:rustc-link-lib=dylib=infer_core");
        } else if static_lib.is_file() {
            println!("cargo:rustc-link-lib=static=infer_core");
        } else {
            panic!(
                "cannot find infer_core library in {}; expected libinfer_core.so or libinfer_core.a",
                lib_dir.display()
            );
        }
        println!("cargo:rerun-if-changed={}", so.display());
        println!("cargo:rerun-if-changed={}", static_lib.display());
    }
    println!("cargo:rerun-if-env-changed=INFER_CORE_LIB_DIR");
    println!("cargo:rerun-if-env-changed=LOCAL_INFER_CORE_ROOT");
}

fn resolve_local_infer_core_dir(manifest_dir: &Path) -> PathBuf {
    if let Ok(root) = std::env::var("LOCAL_INFER_CORE_ROOT") {
        return PathBuf::from(root);
    }
    let nested = manifest_dir.join("local-infer-core");
    if nested.is_dir() {
        return nested;
    }
    manifest_dir.join("..").join("local-infer-core")
}

fn resolve_infer_core_lib_dir(infer_root: &PathBuf, target: &str, profile: &str) -> PathBuf {
    let triple_dir = infer_root.join(target).join(profile);
    let host_dir = infer_root.join(profile);

    if cfg!(target_os = "windows") {
        if triple_dir.join("infer_core.dll.lib").is_file() {
            return triple_dir;
        }
        if host_dir.join("infer_core.dll.lib").is_file() {
            return host_dir;
        }
    }

    if triple_dir.is_dir() {
        triple_dir
    } else {
        host_dir
    }
}

fn has_any_linkable_artifact(lib_dir: &Path, target_os: &str) -> bool {
    match target_os {
        "windows" => {
            lib_dir.join("infer_core.dll.lib").is_file() || lib_dir.join("infer_core.lib").is_file()
        }
        "macos" => {
            lib_dir.join("libinfer_core.dylib").is_file() || lib_dir.join("libinfer_core.a").is_file()
        }
        _ => lib_dir.join("libinfer_core.so").is_file() || lib_dir.join("libinfer_core.a").is_file(),
    }
}

fn build_local_infer_core_ffi(local_infer_core_dir: &Path, profile: &str, target: &str, host: &str) {
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("-p")
        .arg("infer-core-ffi")
        .current_dir(local_infer_core_dir);

    if target.contains("android") {
        cmd.arg("--no-default-features").arg("--features").arg("backend-mnn");
    } else {
        cmd.arg("--features").arg("backend-ort");
    }

    if profile == "release" {
        cmd.arg("--release");
    }
    if !host.is_empty() && target != host {
        cmd.arg("--target").arg(target);
    }

    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("failed to run `cargo build -p infer-core-ffi`: {e}"));
    if !status.success() {
        panic!("`cargo build -p infer-core-ffi` failed with status {status}");
    }
}
