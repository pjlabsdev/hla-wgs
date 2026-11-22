# 59 HGSVC3 samples not used in development, version 1.1

Short reads of the 1000 Genomes 30x data (MHC region, alternative contigs, HLA decoys and unmapped
reads), truth from the HGSVC3 assemblies of the same samples (`truth4_hgsvc.tsv`, built as for the
80 HPRC samples). Database IPD-IMGT/HLA 3.66.

- `eval23.tsv`: two fields 1833 / 1833, three fields 1830 / 1833.
- `eval6.tsv`: certain at four fields 1354, of these 1337 exact (98.7 %); 17 not exact, 12 of them
  beyond long single-base runs and repeats. Reported sites true 743 / 876.
- `eval6_without_f401.tsv`: the same with `HLA_F401=0` (fourth field 01 needs the full lead):
  1315 / 1331 exact. The rule adds 22 exact calls and 1 wrong one here, 38 and 2 on the 80 HPRC samples.
