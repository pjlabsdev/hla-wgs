# Crates from Genome 4

The aligner of hla-wgs lives in Genome 4 now, as the crate `gc-panel` (with the helper crate
`gc-sys`). Genome 4 is not public, so hla-wgs carries an exact copy of both crates here and builds
without access to it. The copy is never edited here: a change is made in Genome 4 first and then
copied over, together with the new commit and checksums below. `tests/crates.rs` fails when a file
differs from its checksum.

Source: Genome 4, commit 5e7d9ea6d6cf (2026-11-24).

| file | CRC-32 |
|---|---|
| gc-sys/src/lib.rs | 758bad15 |
| gc-panel/src/lib.rs | ce173086 |
| gc-panel/src/props.rs | eb8403b8 |

The code is the former `src/aln.rs` and `src/aln_props.rs` of hla-wgs. The only change is that
two hot loops are compiled for AVX2 through `gc_sys::with_avx2` on processors that have it, with the
same code otherwise. Results are byte-identical (8 HPRC samples, all result files), and the run is
faster (150 s instead of 178 s for the 8 samples, 4 threads). How the aligner works:
[docs/aligner.md](../docs/aligner.md).
