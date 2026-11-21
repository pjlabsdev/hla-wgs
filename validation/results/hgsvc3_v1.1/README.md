# 59 unseen HGSVC3 samples with the 1.1 development version

Samples of the Human Genome Structural Variation Consortium, phase 3 (HGSVC3), that were used neither
for development nor for any earlier test (`samples.txt`). Truth: the two haplotype assemblies of each
sample (Verkko, `ftp.1000genomes.ebi.ac.uk/.../HGSVC3/working/*_verkko_batch*/assemblies/`), typed
with the same exon-projection and full-gene scripts as the HPRC truth (`../../hprc/hgsvc_truth.sh`).
Input to hla-wgs: the 1000 Genomes 30x short-read CRAM of the same sample, reads of the MHC region
plus unmapped reads (`../../hprc/hgsvc_type.sh`). 18 genes; the eight genes added in 1.1 are not
scored here yet.

| | result |
|---|---|
| two fields correct | 1833 / 1833 |
| three fields correct | 1830 / 1833 (two DRB4 and one DRB1 third field) |
| known alleles, four fields exact | 1552 / 1572 (98.7 %) |
| reported as certain at four fields | 1206, of these exact 1191 (98.8 %) |
| not exact for a reason other than a homopolymer or repeat length | 11 |

Caveat: the two-field frequency prior is built from 1000 Genomes laboratory types, which may include
some of these samples. The assemblies were not used in any way for the typing.

All 59 samples were typed with the 1.1 development version of 19 November 2026 (HLA-E still capped at
three fields, before the later changes to new sites); the extracted reads were not kept, so the table
is not recomputed with later commits. The first 38 samples gave 1188/1188, 1187/1188, 1000/1013 and
754/763; the remaining 21 were added when their assemblies had been processed.
