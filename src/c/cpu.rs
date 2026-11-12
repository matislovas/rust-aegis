//! What libaegis detected about the processor, for callers that need to know which
//! instructions its backends use: kernel code, for example, may use vector registers only
//! while their state is saved, and must know which state that is.
//!
//! Each function runs libaegis' processor detection first, if nothing has yet.

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
extern "C" {
    fn aegis_runtime_has_aesni() -> core::ffi::c_int;
    fn aegis_runtime_has_avx() -> core::ffi::c_int;
    fn aegis_runtime_has_vaes() -> core::ffi::c_int;
    fn aegis_runtime_has_avx2() -> core::ffi::c_int;
    #[cfg(feature = "avx512")]
    fn aegis_runtime_has_avx512f() -> core::ffi::c_int;
}

#[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
extern "C" {
    fn aegis_runtime_has_neon_aes() -> core::ffi::c_int;
}

/// Whether the AEGIS ciphers run on hardware AES here: AES-NI and AVX on x86 and x86_64
/// (where every hardware backend uses AVX), the AES instructions on Arm. Otherwise libaegis
/// uses its software AES.
pub fn has_hardware_aes() -> bool {
    super::init();
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    // SAFETY: no arguments; reads what `aegis_init` detected.
    return unsafe { aegis_runtime_has_aesni() != 0 && aegis_runtime_has_avx() != 0 };
    #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
    // SAFETY: as above.
    return unsafe { aegis_runtime_has_neon_aes() != 0 };
    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86", target_arch = "aarch64", target_arch = "arm")))]
    return false;
}

/// Whether this x86 processor has VAES and AVX2, on which the X2 and X4 ciphers run on 256-bit
/// vector AES. False on other processors.
pub fn has_vaes_avx2() -> bool {
    super::init();
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    // SAFETY: no arguments; reads what `aegis_init` detected.
    return unsafe { aegis_runtime_has_vaes() != 0 && aegis_runtime_has_avx2() != 0 };
    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
    return false;
}

/// Whether the X4 ciphers use AVX-512 registers here: only with the `avx512` feature (which
/// builds those backends), on x86 processors with VAES and AVX-512F. Kernel code must then
/// save the AVX-512 state as well as the AVX state.
pub fn uses_avx512() -> bool {
    super::init();
    #[cfg(all(feature = "avx512", any(target_arch = "x86_64", target_arch = "x86")))]
    // SAFETY: no arguments; reads what `aegis_init` detected.
    return unsafe { aegis_runtime_has_vaes() != 0 && aegis_runtime_has_avx512f() != 0 };
    #[cfg(not(all(feature = "avx512", any(target_arch = "x86_64", target_arch = "x86"))))]
    return false;
}
