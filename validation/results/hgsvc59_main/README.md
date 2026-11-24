# 59 HGSVC3 samples not used in development, current main

Same reads, truth (`truth4_hgsvc.tsv`) and database (IPD-IMGT/HLA 3.66) as `hgsvc59_v1.1`, typed
with the program of this branch. Evaluated with `hla-wgs-validate eval6` and `eval23`
(`T4OUT=truth4_hgsvc.tsv`).

- `eval23.tsv`: two fields 1833 / 1833, three fields 1830 / 1833 (unchanged).
- `eval6.tsv`: certain at four fields 1350, of these 1334 exact (98.8 %); 16 not exact (1.1: 17),
  11 of them beyond long single-base runs and repeats (1.1: 12). Against 1.1: A +1, B +1, F +1
  exact certain four-field calls; C -2, DPA1 -1, H -1, V -3 (V also one wrong call fewer).
- New SNV sites found 729 / 822, reported sites true 869 / 973. These counts are not comparable
  with 1.1: the evaluation now reads all sites from `PREFIX.novel.tsv` and counts sites marked
  `:either` on the other haplotype (see CHANGELOG).
