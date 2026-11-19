# 38 unseen HGSVC3 samples with the 1.1 development version

Samples of the Human Genome Structural Variation Consortium, phase 3 (HGSVC3), that were used neither
for development nor for any earlier test (`samples.txt`). Truth: the two haplotype assemblies of each
sample (Verkko, `ftp.1000genomes.ebi.ac.uk/.../HGSVC3/working/*_verkko_batch*/assemblies/`), typed
with the same exon-projection and full-gene scripts as the HPRC truth (`../../hprc/hgsvc_truth.sh`).
Input to hla-wgs: the 1000 Genomes 30x short-read CRAM of the same sample, reads of the MHC region
plus unmapped reads (`../../hprc/hgsvc_type.sh`). 18 genes; the eight genes added in 1.1 are not
scored here yet.

| | result |
|---|---|
| two fields correct | 1188 / 1188 |
| three fields correct | 1187 / 1188 (one DRB4 third field) |
| known alleles, four fields exact | 1000 / 1013 (98.7 %) |
| reported as certain at four fields | 763, of these exact 754 (98.8 %) |
| not exact for a reason other than a homopolymer or repeat length | 5 |

Caveat: the two-field frequency prior is built from 1000 Genomes laboratory types, which may include
some of these samples. The assemblies were not used in any way for the typing.
