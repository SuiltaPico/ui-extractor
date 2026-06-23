fn main() {
    #[cfg(feature = "backend-ncnn")]
    link_ncnn();
}

#[cfg(feature = "backend-ncnn")]
fn link_ncnn() {
    println!("cargo:rerun-if-env-changed=NCNN_LIB_DIR");

    let lib_dir = std::env::var("NCNN_LIB_DIR")
        .expect("NCNN_LIB_DIR must be set when building with --features backend-ncnn");

    println!("cargo:rustc-link-search=native={lib_dir}");
    println!("cargo:rustc-link-lib=static=ncnn");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        println!("cargo:rustc-link-lib=c++_shared");
        return;
    }

    #[cfg(target_env = "msvc")]
    {
        // MSVC links C++ runtime via ncnn.lib dependencies.
    }

    #[cfg(not(target_env = "msvc"))]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("android") {
        println!("cargo:rustc-link-lib=stdc++");
    }
}
