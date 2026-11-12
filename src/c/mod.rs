/// Encrypted random-access file I/O built on top of the AEGIS ciphers.
#[cfg(feature = "raf-core")]
#[cfg_attr(docsrs, doc(cfg(feature = "raf")))]
pub mod raf;

/// AEGIS-128L.
#[cfg(feature = "aegis128")]
pub mod aegis128l;
/// AEGIS-128X2, the 2-lane variant of AEGIS-128L.
#[cfg(feature = "aegis128")]
pub mod aegis128x2;
/// AEGIS-128X4, the 4-lane variant of AEGIS-128L.
#[cfg(feature = "aegis128")]
pub mod aegis128x4;
/// AEGIS-256.
pub mod aegis256;
/// AEGIS-256X2, the 2-lane variant of AEGIS-256.
pub mod aegis256x2;
/// AEGIS-256X4, the 4-lane variant of AEGIS-256.
pub mod aegis256x4;

/// What libaegis detected about the processor: which instructions its backends use.
pub mod cpu;

use core::sync::atomic::{AtomicU8, Ordering};

extern "C" {
    fn aegis_init() -> core::ffi::c_int;
}

/// `aegis_init` has: not run (0), is running (1), run (2).
static INIT: AtomicU8 = AtomicU8::new(0);

/// Runs libaegis' `aegis_init` once for all the ciphers: it detects the processor and picks
/// each cipher's backend. A caller that arrives meanwhile spins until it is done (microseconds,
/// once). One guard for every cipher, so `aegis_init` never runs twice at the same time.
pub(crate) fn init() {
    if INIT.load(Ordering::Acquire) == 2 {
        return;
    }
    if INIT.compare_exchange(0, 1, Ordering::Acquire, Ordering::Acquire).is_ok() {
        // SAFETY: no arguments; called once (this thread won the exchange), before any other
        // libaegis call (they all call `init` first and wait for it).
        assert_eq!(unsafe { aegis_init() }, 0);
        INIT.store(2, Ordering::Release);
    } else {
        while INIT.load(Ordering::Acquire) != 2 {
            core::hint::spin_loop();
        }
    }
}
