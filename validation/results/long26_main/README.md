# Long-read check on 26 HPRC samples, current development version

Same samples and reads as `../long26/` (PacBio HiFi, MHC region), typed with `hla-wgs long` after the
long-read changes of the unreleased version (plurality consensus, ranking by spanning reads, nearest
allele by exons, joint consensus of read groups naming the same allele). `SAMPLE.long.tsv` are the
calls; `long.txt` is the output of

    HLA_LTAG=TAG hla-wgs-validate long DIR SAMPLES.txt v

run in the work folder with `truth/SAMPLE.{h1,h2,mat,pat}.mhc.fa` (assembled haplotypes, MHC window)
and `truth/truth4.tsv`. It reports:

- consensus sequences identical to one assembled haplotype over their whole length, per gene
  (unaligned ends count as differences; the program's own aligner in asm5 mode is used): 1434 / 1519
  (1.0.0: 1398 / 1501)
- assembled haplotypes reproduced exactly by some consensus: 1524 (within 2 differences: 1623)
- calls reported `exact = yes` whose allele is in the truth set of a haplotype: 607 / 609
- calls with differences (nearest allele) right at two fields: 129 / 131

The same counts came out of the earlier Python scripts that used minimap2 for the alignment, so the
own aligner gives identical distances here.
