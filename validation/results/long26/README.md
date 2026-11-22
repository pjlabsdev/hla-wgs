# Long-read check on 26 HPRC samples

`hla-wgs long` on the PacBio HiFi reads of the MHC region (GRCh38 chr6:28,510,120-33,480,577, about
40x) of the 26 development and check samples for which HPRC provides HiFi alignments to GRCh38
(`working/HPRC/<sample>/analysis/aligned_reads/hifi/GRCh38/`). About one minute per sample on 4 cores.

Each sequence in `PREFIX.long.fa` was aligned to both assembled haplotypes of the sample (HPRC
release 2); "exact" means identical to one of them over its whole length.

| | exact / all |
|---|---|
| all sequences, all genes found | 1,398 / 1,501 |
| the 18 genes of the short-read typer | 689 / 726 |
| of these, without unsure sites | 640 / 642 (99.7 %) |
| of these, with unsure sites | 49 / 84 |
| of these, reported `exact = yes` (distance 0 to a database allele, no unsure site) | 546 / 546 |
| of these, new alleles (distance above 0) | 104 / 137 |

Per gene: A 48/48, B 48/50, C 50/50, DPA1 50/50, DPB1 49/51, DQA1 45/50, DQB1 49/50, DRA 46/48,
DRB1 36/49, DRB3 16/23, DRB4 15/18, DRB5 2/3, E 40/40, F 45/45, G 44/44, H 40/41, K 30/30, V 36/36;
DMA 46/47, DMB 43/48, DOA 49/49, DOB 50/50, MICA 45/49, MICB 23/50, TAP1 40/41, TAP2 46/47;
further DPA2 31/31, DPB2 16/37, DQA2 43/49, DQB2 49/49, J 45/45, L 44/44, P 35/35, R 13/13, T 42/42,
W 38/38, Y 11/11.

The differences are nearly all one to three bases in long single-base runs and short tandem repeats
(the microsatellites of DRB1, DRB3 and DRB4, poly-A and poly-T runs in MICB and DPB2). The program
marks such places as unsure sites.

Limits of this check: the assemblies were built from the same HiFi reads, so agreement shows that the
program reads the data like the assembler, not that both are right. The samples were not selected for
this test but are part of the earlier development and check sets. Nanopore reads were not tested.
