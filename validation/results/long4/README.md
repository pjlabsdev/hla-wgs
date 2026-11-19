# Long-read check on four HPRC samples

`hla-wgs long` on the PacBio HiFi reads of the MHC region (GRCh38 chr6:28,510,120-33,480,577, about
40x) of HG00438, HG00621, HG01071 and HG02630, taken from the HPRC alignments
(`working/HPRC/<sample>/analysis/aligned_reads/hifi/GRCh38/`).

Each sequence in `PREFIX.long.fa` was aligned to both assembled haplotypes of the sample (HPRC
release 2). 214 of 232 sequences are identical to a haplotype. Per gene:

A 8/8, B 7/8, C 8/8, DPA1 8/8, DPB1 8/8, DQA1 4/7, DQB1 6/7, DRA 6/6, DRB1 5/7, DRB3 3/3, DRB4 5/5,
E 6/6, F 8/8, G 8/8, H 6/6, K 6/6, V 5/5, DMA 6/6, DMB 6/6, DOA 8/8, DOB 8/8, MICA 7/8, MICB 3/8,
TAP1 6/6, TAP2 7/7; further DPA2 5/5, DPB2 2/7, DQA2 8/8, DQB2 8/8, J 7/7, L 7/7, P 6/6, T 6/6, W 7/7.

Most of the 18 differences are one to three bases in long single-base runs. Larger ones: DRB1*07 of
HG01071 (11, a 16 kb gene without a spanning read at low depth), DQB1 of HG01071 (20; the sequence fits
neither assembled haplotype and is 3 edits from DQB1*02:02:01:01), MICA of HG01071 (43) and MICB of
HG02630 (294). The last three have not been examined yet.

Limits of this check: four samples, and the assemblies were built from the same HiFi reads, so
agreement shows that the program reads the data like the assembler, not that both are right.
