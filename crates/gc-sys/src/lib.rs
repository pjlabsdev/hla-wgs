//! Systemnahes je Prozessor und Betriebssystem. Fachcode ruft nur diese Funktionen auf und
//! enthält selbst keine Befehle für einen bestimmten Prozessor.

/// Bittet den Prozessor, die Speicherzeile mit `value` schon in den Cache zu holen, weil sie
/// gleich gelesen wird. Ändert nichts am Ergebnis eines Programms: Auf x86_64 und aarch64 ist es
/// ein Hinweis-Befehl, der nie einen Fehler auslöst, auf anderen Prozessoren geschieht nichts.
#[inline(always)]
pub fn prefetch<T>(value: &T) {
    let p = value as *const T;
    #[cfg(target_arch = "x86_64")]
    // SAFETY: `prefetch` liest nichts und schreibt nichts; ein Hinweis auf eine gültige Adresse
    // aus einer Referenz.
    unsafe {
        use std::arch::x86_64::{_MM_HINT_T0, _mm_prefetch};
        _mm_prefetch(p.cast::<i8>(), _MM_HINT_T0);
    }
    #[cfg(target_arch = "aarch64")]
    // SAFETY: `prfm` ist ein reiner Hinweis, er greift nicht auf den Speicher zu und löst keinen
    // Fehler aus.
    unsafe {
        std::arch::asm!("prfm pldl1keep, [{0}]", in(reg) p, options(nostack, readonly, preserves_flags));
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    let _ = p;
}

/// Führt `f` mit AVX2 übersetzt aus, wenn der Prozessor AVX2 kann, sonst wie übersetzt. Für
/// Schleifen, die der Übersetzer erst mit AVX2 in Vektorbefehle mit 32-Bit-Maximum verwandelt
/// (x86_64 setzt sonst nur SSE2 voraus). Das Ergebnis ist in beiden Fällen dasselbe, weil derselbe
/// Code läuft; `f` sollte nur `#[inline(always)]`-Funktionen aufrufen, damit sie mit AVX2 übersetzt
/// werden. Auf anderen Prozessoren läuft `f` unverändert.
#[inline(always)]
pub fn with_avx2<R>(f: impl FnOnce() -> R) -> R {
    #[cfg(target_arch = "x86_64")]
    if std::arch::is_x86_feature_detected!("avx2") {
        #[target_feature(enable = "avx2")]
        unsafe fn run<R>(f: impl FnOnce() -> R) -> R {
            f()
        }
        // SAFETY: der Prozessor kann AVX2, das hat die Abfrage eben gezeigt.
        return unsafe { run(f) };
    }
    f()
}

#[cfg(test)]
mod tests {
    use super::{prefetch, with_avx2};

    #[test]
    fn with_avx2_same_result() {
        let v: Vec<i32> = (0..10_000).map(|i| (i * 7919) % 1013 - 500).collect();
        let plain = v.iter().copied().max();
        assert_eq!(with_avx2(|| v.iter().copied().max()), plain);
    }

    #[test]
    fn prefetch_changes_nothing() {
        let data: Vec<u64> = (0..1024).collect();
        for x in &data {
            prefetch(x);
        }
        assert_eq!(data.iter().sum::<u64>(), 1023 * 1024 / 2);
    }
}
