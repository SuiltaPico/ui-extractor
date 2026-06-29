use std::fs::{self, File};
use std::io::copy;
use std::path::{Path, PathBuf};

const DEFAULT_RELEASE_REPO: &str = "SuiltaPico/local-infer-core";

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
    let target = std::env::var("TARGET").unwrap();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

    ensure_infer_core_release(&manifest_dir, &target, &target_os);

    let lib_dir = resolve_infer_core_release_lib_dir(&manifest_dir, &target);
    if !has_any_linkable_artifact(&lib_dir, &target_os) {
        panic!(
            "infer_core native library not found in {} after release download.\n\
Check network access to GitHub Releases or run:\n\
  powershell -ExecutionPolicy Bypass -File scripts/download_infer_core_release.ps1",
            lib_dir.display()
        );
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

    let release_root = manifest_dir.join(".infer-core-release");
    println!("cargo:rerun-if-changed={}", release_root.display());
    println!("cargo:rerun-if-changed={}", manifest_dir.join("Cargo.toml").display());
}

fn ensure_infer_core_release(manifest_dir: &Path, target: &str, target_os: &str) {
    let lib_dir = resolve_infer_core_release_lib_dir(manifest_dir, target);
    if has_any_linkable_artifact(&lib_dir, target_os) {
        return;
    }

    let asset = infer_core_release_asset_base(target);
    let release_root = manifest_dir.join(".infer-core-release");
    let extract_dir = release_root.join(asset);
    let marker = extract_dir.join(".extracted");

    if marker.is_file() && has_any_linkable_artifact(&lib_dir, target_os) {
        return;
    }

    let tag = resolve_release_tag();
    let url = format!(
        "https://github.com/{}/releases/download/{}/{asset}.zip",
        DEFAULT_RELEASE_REPO,
        tag,
        asset = asset
    );

    eprintln!("build.rs: downloading infer_core release {url}");

    fs::create_dir_all(&release_root).unwrap_or_else(|e| {
        panic!("failed to create {}: {e}", release_root.display());
    });

    let cache_dir = manifest_dir.join(".infer-core-release-cache");
    fs::create_dir_all(&cache_dir).unwrap_or_else(|e| {
        panic!("failed to create {}: {e}", cache_dir.display());
    });
    let zip_path = cache_dir.join(format!("{asset}.zip"));

    download_url(&url, &zip_path);

    if extract_dir.exists() {
        fs::remove_dir_all(&extract_dir).unwrap_or_else(|e| {
            panic!("failed to remove {}: {e}", extract_dir.display());
        });
    }
    fs::create_dir_all(&extract_dir).unwrap_or_else(|e| {
        panic!("failed to create {}: {e}", extract_dir.display());
    });

    extract_zip(&zip_path, &extract_dir);

    fs::write(&marker, url.as_bytes()).unwrap_or_else(|e| {
        panic!("failed to write marker {}: {e}", marker.display());
    });

    if !has_any_linkable_artifact(&lib_dir, target_os) {
        panic!(
            "infer_core release extracted to {} but link artifacts missing in {}\n\
Expected infer-core Windows layout: lib/infer_core.dll.lib\n\
Android layout: jniLibs/<abi>/libinfer_core.so",
            extract_dir.display(),
            lib_dir.display()
        );
    }

    eprintln!(
        "build.rs: infer_core release ready at {}",
        lib_dir.display()
    );
}

fn resolve_release_tag() -> String {
    if let Ok(ref_name) = std::env::var("GITHUB_REF_NAME") {
        if ref_name.starts_with('v') {
            return ref_name;
        }
        return format!("v{ref_name}");
    }

    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".into());
    format!("v{version}")
}

fn download_url(url: &str, dest: &Path) {
    let response = ureq::get(url)
        .call()
        .unwrap_or_else(|e| panic!("GET {url} failed: {e}"));

    let status = response.status();
    if status != 200 {
        panic!("GET {url} failed with status {status}");
    }

    let mut reader = response.into_reader();
    let mut file = File::create(dest).unwrap_or_else(|e| {
        panic!("failed to create {}: {e}", dest.display());
    });
    copy(&mut reader, &mut file).unwrap_or_else(|e| {
        panic!("failed to write {}: {e}", dest.display());
    });
}

fn extract_zip(zip_path: &Path, dest_dir: &Path) {
    let file = File::open(zip_path).unwrap_or_else(|e| {
        panic!("failed to open zip {}: {e}", zip_path.display());
    });
    let mut archive = zip::ZipArchive::new(file).unwrap_or_else(|e| {
        panic!("invalid zip {}: {e}", zip_path.display());
    });

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).unwrap_or_else(|e| {
            panic!("zip entry {i} in {}: {e}", zip_path.display());
        });
        let entry_path = match entry.enclosed_name() {
            Some(path) => dest_dir.join(path),
            None => continue,
        };

        if entry.name().ends_with('/') {
            fs::create_dir_all(&entry_path).unwrap_or_else(|e| {
                panic!("failed to create dir {}: {e}", entry_path.display());
            });
        } else {
            if let Some(parent) = entry_path.parent() {
                fs::create_dir_all(parent).unwrap_or_else(|e| {
                    panic!("failed to create dir {}: {e}", parent.display());
                });
            }
            let mut outfile = File::create(&entry_path).unwrap_or_else(|e| {
                panic!("failed to create {}: {e}", entry_path.display());
            });
            copy(&mut entry, &mut outfile).unwrap_or_else(|e| {
                panic!("failed to extract {}: {e}", entry_path.display());
            });
        }
    }
}

fn infer_core_release_asset_base(target: &str) -> &'static str {
    match target {
        "x86_64-pc-windows-msvc" => "infer-core-windows-x86_64",
        "aarch64-pc-windows-msvc" => "infer-core-windows-aarch64",
        "aarch64-linux-android" => "infer-core-android-arm64-v8a",
        "x86_64-linux-android" => "infer-core-android-x86_64",
        _ => panic!(
            "unsupported TARGET for infer_core release layout: {target}\n\
Supported: x86_64-pc-windows-msvc, aarch64-pc-windows-msvc, aarch64-linux-android, x86_64-linux-android"
        ),
    }
}

fn resolve_infer_core_release_lib_dir(manifest_dir: &Path, target: &str) -> PathBuf {
    let asset = infer_core_release_asset_base(target);
    let extract_dir = manifest_dir.join(".infer-core-release").join(asset);

    if target.contains("android") {
        let abi = match target {
            "aarch64-linux-android" => "arm64-v8a",
            "x86_64-linux-android" => "x86_64",
            _ => unreachable!(),
        };
        extract_dir.join("jniLibs").join(abi)
    } else {
        extract_dir.join("lib")
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
