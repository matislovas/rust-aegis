//! Build script that compiles the bundled libaegis C sources for the native backend.
//!
//! Features: `aegis128` (the AEGIS-128L family; without it libaegis' `DISABLE_AEGIS128`),
//! `avx512` (the X4 ciphers' AVX-512 backends; without it `DISABLE_AVX512`), and `kernel`
//! (code without a C runtime: `AEGIS_NO_CONSTRUCTOR`, `AEGIS_NO_ERRNO`, and on MSVC targets
//! clang-cl with `/GS-` and `/kernel`; no tuning for the build machine and no instruction sets
//! beyond the backends' own target attributes).

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn has_clang() -> bool {
    Command::new("clang")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn is_cross_compiling() -> bool {
    let host = env::var("HOST").unwrap_or_default();
    let target = env::var("TARGET").unwrap_or_default();
    !host.is_empty() && !target.is_empty() && host != target
}

/// clang-cl: on PATH, in `%LLVM_PATH%\bin`, or in `%ProgramFiles%\LLVM\bin`. The backends set
/// their instruction sets per function with clang's target attributes, which MSVC's own
/// compiler ignores.
fn clang_cl() -> PathBuf {
    let candidates = env::var_os("PATH")
        .map(|p| env::split_paths(&p).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .chain(env::var_os("LLVM_PATH").map(|p| Path::new(&p).join("bin")))
        .chain(env::var_os("ProgramFiles").map(|p| Path::new(&p).join("LLVM").join("bin")));
    for dir in candidates {
        let exe = dir.join("clang-cl.exe");
        if exe.is_file() {
            return exe;
        }
    }
    panic!("the `kernel` feature needs LLVM's clang-cl on MSVC targets: install LLVM, or set LLVM_PATH");
}

/// libaegis sources of the AEGIS-128L family, then of the AEGIS-256 family, then the common
/// code. The AVX-512 backends are listed separately.
const AEGIS128_SOURCES: &[&str] = &[
    "aegis128l/aegis128l.c",
    "aegis128l/aegis128l_aesni.c",
    "aegis128l/aegis128l_altivec.c",
    "aegis128l/aegis128l_neon_aes.c",
    "aegis128l/aegis128l_neon_sha3.c",
    "aegis128l/aegis128l_soft.c",
    "aegis128x2/aegis128x2.c",
    "aegis128x2/aegis128x2_aesni.c",
    "aegis128x2/aegis128x2_altivec.c",
    "aegis128x2/aegis128x2_neon_aes.c",
    "aegis128x2/aegis128x2_avx2.c",
    "aegis128x2/aegis128x2_soft.c",
    "aegis128x4/aegis128x4.c",
    "aegis128x4/aegis128x4_aesni.c",
    "aegis128x4/aegis128x4_altivec.c",
    "aegis128x4/aegis128x4_neon_aes.c",
    "aegis128x4/aegis128x4_avx2.c",
    "aegis128x4/aegis128x4_soft.c",
];
const AEGIS256_SOURCES: &[&str] = &[
    "aegis256/aegis256.c",
    "aegis256/aegis256_aesni.c",
    "aegis256/aegis256_altivec.c",
    "aegis256/aegis256_neon_aes.c",
    "aegis256/aegis256_soft.c",
    "aegis256x2/aegis256x2.c",
    "aegis256x2/aegis256x2_aesni.c",
    "aegis256x2/aegis256x2_altivec.c",
    "aegis256x2/aegis256x2_neon_aes.c",
    "aegis256x2/aegis256x2_avx2.c",
    "aegis256x2/aegis256x2_soft.c",
    "aegis256x4/aegis256x4.c",
    "aegis256x4/aegis256x4_aesni.c",
    "aegis256x4/aegis256x4_altivec.c",
    "aegis256x4/aegis256x4_neon_aes.c",
    "aegis256x4/aegis256x4_avx2.c",
    "aegis256x4/aegis256x4_soft.c",
];
const COMMON_SOURCES: &[&str] = &["common/common.c", "common/cpu.c", "common/softaes.c"];
const AEGIS128_AVX512_SOURCES: &[&str] = &["aegis128x4/aegis128x4_avx512.c"];
const AEGIS256_AVX512_SOURCES: &[&str] = &["aegis256x4/aegis256x4_avx512.c", "aegis256x4/aegis256x4_avx512vl.c"];

fn main() {
    let force_relaxed_simd = env::var("CARGO_FEATURE_FORCE_WASM_RELAXED_SIMD").is_ok();
    let force_strict_simd = env::var("CARGO_FEATURE_FORCE_WASM_STRICT_SIMD").is_ok();
    if force_relaxed_simd && force_strict_simd {
        panic!(
            "The `force-wasm-relaxed-simd` and `force-wasm-strict-simd` features are mutually exclusive"
        );
    }
    let pure_rust = env::var("CARGO_FEATURE_PURE_RUST").is_ok();
    if pure_rust {
        return;
    }
    let arch = env::var("CARGO_CFG_TARGET_ARCH").expect("No target arch");
    if arch == "wasm32" {
        let src_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
        let library = if force_relaxed_simd {
            "aegis_relaxed_simd"
        } else if force_strict_simd {
            "aegis"
        } else if target_os == "wasi" {
            "aegis_relaxed_simd"
        } else {
            "aegis"
        };
        println!("cargo:rustc-link-lib=static={library}");
        println!("cargo:rustc-link-search=native={}/wasm-libs", src_dir);
        return;
    }
    let aegis128 = env::var("CARGO_FEATURE_AEGIS128").is_ok();
    let avx512 = env::var("CARGO_FEATURE_AVX512").is_ok();
    let kernel = env::var("CARGO_FEATURE_KERNEL").is_ok();
    let mut build = cc::Build::new();
    let cross = is_cross_compiling();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_env == "msvc" && kernel {
        println!("cargo:rerun-if-env-changed=LLVM_PATH");
        build.compiler(clang_cl());
    } else if target_env == "msvc" {
        // On MSVC targets, use the cc crate's proper API for preferring clang-cl.
        // Manually setting compiler("clang") on MSVC targets causes malformed
        // command lines (the -x flag is generated without its language argument).
        build.prefer_clang_cl_over_msvc(true);
    } else if !cross && has_clang() {
        // Only force host clang for native builds. When cross-compiling (e.g.
        // for Android via NDK), let the `cc` crate auto-detect the correct
        // toolchain from CC_<target> / CFLAGS_<target> environment variables.
        build.compiler("clang");
    }
    build
        .opt_level(3)
        .flag_if_supported("-Wno-unused-command-line-argument")
        .flag_if_supported("-Wno-unknown-pragmas");
    if kernel {
        build.define("AEGIS_NO_CONSTRUCTOR", None).define("AEGIS_NO_ERRNO", None);
        if target_env == "msvc" {
            build.flag("/GS-").flag("/kernel");
        }
    } else {
        if !cross {
            build.flag_if_supported("-mtune=native");
        }
        build
            .flag_if_supported("-mcrypto")
            .flag_if_supported("-mneon")
            .flag_if_supported("-maes");
    }
    let src = Path::new("src/c/libaegis/src");
    build.include(src.join("include"));
    let mut sources: Vec<&str> = Vec::new();
    if aegis128 {
        sources.extend(AEGIS128_SOURCES);
        if avx512 {
            sources.extend(AEGIS128_AVX512_SOURCES);
        }
    } else {
        build.define("DISABLE_AEGIS128", None);
    }
    sources.extend(AEGIS256_SOURCES);
    if avx512 {
        sources.extend(AEGIS256_AVX512_SOURCES);
    } else {
        build.define("DISABLE_AVX512", None);
    }
    sources.extend(COMMON_SOURCES);
    build.files(sources.iter().map(|f| src.join(f)));
    if env::var("CARGO_FEATURE_RAF_CORE").is_ok() {
        build
            .file("src/c/libaegis/src/common/keccak.c")
            .file("src/c/libaegis/src/raf/raf.c")
            .file("src/c/libaegis/src/raf/raf_kdf.c")
            .file("src/c/libaegis/src/raf/raf_merkle.c")
            .file("src/c/libaegis/src/raf/raf_aegis128l.c")
            .file("src/c/libaegis/src/raf/raf_aegis128x2.c")
            .file("src/c/libaegis/src/raf/raf_aegis128x4.c")
            .file("src/c/libaegis/src/raf/raf_aegis256.c")
            .file("src/c/libaegis/src/raf/raf_aegis256x2.c")
            .file("src/c/libaegis/src/raf/raf_aegis256x4.c");
    }
    build.compile("aegis");
}
