# Second truth: Lai et al. 2024

Lai et al. 2024 (PMC11035020) typed 44 HPRC samples for 11 genes from the assemblies and used this
as the benchmark for seven typing programs. 40 of these samples are among the 80 development
samples here, so their table is a truth that was made independently of ours (other people, other
method, older database release).

## Truth against truth (40 samples)

| | A | B | C | DPA1 | DPB1 | DQA1 | DQB1 | DRB1 | DRB3 | DRB4 | DRB5 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| samples compared | 40 | 40 | 40 | 40 | 40 | 40 | 40 | 40 | 30 | 17 | 5 |
| two fields equal | 40 | 40 | 40 | 40 | 40 | 40 | 40 | 39 | 30 | 16 | 5 |
| all fields equal | 39 | 36 | 36 | 40 | 38 | 35 | 37 | 34 | 19 | 11 | 3 |

Where the full names differ:

- Our name is an allele that entered IPD-IMGT/HLA after the release used by Lai et al. (for example
  C\*17:01:01:30, DQB1\*02:02:01:21, DRB4\*01:03:01:19, DPB1\*13:01:01:17, DRB5\*01:01:01:11,
  DQA1\*03:02:01:04, DRB3\*02:02:01:39). The assembly equals our allele without a difference.
- HG01891 DPB1 and HG02572 DQA1: checked against the assembly with
  `hla-wgs-validate allele-vs-assembly`. Our allele (DPB1\*01:01:01:33, DQA1\*05:01:01:16) has no
  difference, the allele named by Lai et al. has one.
- New alleles: both tables name a nearest database allele, and they choose differently when a
  short database entry and a full-length one are equally near. Ours takes the entry with the fewest
  differences over its own length (see the limits in the main README).
- HG01361 DRB4: Lai et al. \*01:01:01, ours \*03:01N. The database entry of \*03:01N holds exons 3 to
  6 only and has one difference to the assembly, \*01:01:01:01 has two over the whole gene. Both
  names describe the same sequence; hla-wgs reports DRB4\*01 as certain there.
- HG01358 DRB1\*04:92 and NA19240 DRB5 are in our exon-level table and missing only in the
  whole-gene table, because the nearest allele is an exon-only entry.

No case shows an error of our truth table.

## hla-wgs and the seven programs on HG002

Read data: 30x FASTQ, hla-wgs 1.1 development version. Against the benchmark of Lai et al., hla-wgs
names the full allele correctly for all 11 genes as far as the benchmark goes (DRB1 has three
fields there; DRB3 and DRB5 are correctly absent). The fields reported as certain are never wrong;
for DPB1, DRB1 and DRB4 fewer fields are certain than are correct. None of the programs in the
study (DRAGEN, HISAT-genotype, HLA-VBSeq, OptiType, SpecHLA, T1K, QzType; results taken from the
supplement of the study) is correct for all 11 genes on this sample. One sample is not a ranking;
the comparison on 150 samples is in the main README.
