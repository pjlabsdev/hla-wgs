# 80 HPRC development samples with the 1.1 development version

Same samples and truth as `../dev80/` (both assembled haplotypes of each sample), typed with the
1.1 development version of 20 November 2026 and a database built by it (format 2).

- `eval6.tsv`, `eval23.tsv`: the 18 genes of release 1.0, scripts `../../hprc/eval6.py` and `eval23.py`.
  Two fields 2486 / 2489, three fields 2482 / 2489, certain at four fields 1747, of these 1726 exact.
- `truth8.tsv`: for the eight added genes (DMA, DMB, DOA, DOB, MICA, MICB, TAP1, TAP2), per haplotype
  the full-length database alleles with the fewest edits against the assembly (`../../hprc/truth8.py`).
- `eval8_exons.tsv`: calls certain to three fields or more whose exons all occur unchanged in the
  assigned haplotype (`../../hprc/eval8_exons.py`): 988 of 999.

Fourth field of the added genes against the whole gene sequence (`../../hprc/eval8.py`): DMA 153 / 153,
DOA 112 / 115, DOB 134 / 136, TAP1 40 / 41, DMB 88 / 106, TAP2 38 / 44. DMB and TAP2 are therefore
reported to three fields at most. MICA and MICB never reach four fields.

These samples were used for development; the unseen samples are in `../hgsvc3_v1.1/`.
