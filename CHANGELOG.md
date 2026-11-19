# Changelog

## Unreleased (1.1.0)

- Long reads (experimental): `hla-wgs long DBDIR READS.fastq[.gz]|READS.bam PREFIX [threads]` types from
  PacBio HiFi reads. The gene sequence of each haplotype is built from the reads (consensus) and
  compared with every full-length allele; `PREFIX.long.tsv` gives the nearest allele and the number of
  differences, `PREFIX.long.fa` the sequences. On four HPRC samples (40x HiFi, MHC region) 214 of 232
  sequences were identical to the assembled haplotype, about one minute per sample on 4 cores. The
  assemblies were made from the same reads, so this is not an independent truth.
- New alleles as sequence: `PREFIX.novel.fa` holds, for every allele with new sites, the nearest
  database allele with the substitutions, insertions and deletions applied. The header names the
  nearest allele and the changes, lists what could not be applied (`incomplete=`, for example blind
  stretches) and marks changes that could belong to either allele (`phase=uncertain`).
- Repeat lengths: single-base runs of 8 or more and short tandem repeats are examined with the reads
  that span them with 10 bases on both sides. If these reads agree on another length than the
  allele has, the site is reported as `rep+N` / `rep-N` and limits the resolution like any other new
  site. On the 80 development samples this removed one inexact four-field statement and no exact
  one. Lengths of repeats longer than about 20 bases stay undetermined with short reads.
- JSON: `g_group` and `p_group` per allele when `hla_nom_g.txt` and `hla_nom_p.txt` of the
  IPD-IMGT/HLA release lie in the database directory, `expression` for alleles with a suffix
  (N, L, S, Q, C, A), and `pharmacogenetic_alleles` (B*57:01, B*15:02, A*31:01, B*58:01 with copy
  count and whether the two-field statement is certain).
- Eight more genes: DMA, DMB, DOA, DOB, MICA, MICB, TAP1 and TAP2 (26 in total). Against the
  assemblies of the 80 development samples DMA, DMB, DOA and DOB agree at two and at three fields on
  640 of 640 haplotypes. MICA, MICB, TAP1 and TAP2 are experimental: agreement on the first two
  name fields is 140/148, 153/154, 101/108 and 145/160. The calls of the first 18 genes are
  unchanged.
- Input: an I/O error (for example a USB disk that drops out) no longer aborts the run. The input is
  reopened after a pause and reading resumes after the last record (`HLA_IO_RETRIES`, default 6).
- `HLA_SAVE_READS=PREFIX` writes the read pairs that pass the panel prefilter as FASTQ; typing these
  files gives the same result without reading the BAM or CRAM again.

## 1.0.0 (2026-11-19)

First public release.

- Types A, B, C, DRB1, DRB3, DRB4, DRB5, DQA1, DQB1, DPA1, DPB1, DRA, E, F, G, H, K and V from
  paired FASTQ, BAM or CRAM (GRCh38 or GRCh37).
- Per allele: best database allele, `safe_call` with `resolution` 1 to 4, margin and nearest
  alternative for fields 2, 3 and 4.
- Copy number for DRB3/4/5 (joint model with DRB1), H and K.
- New-allele detection: reads are realigned to the called alleles; consistent differences and
  stretches without reads are listed as `new_sites` and cut the resolution.
- A called allele whose new sites turn it into another database allele is renamed to that allele
  (`renamed_from` in the JSON).
- `hla-wgs build-db` builds the reference from the IPD-IMGT/HLA `hla.dat`; `hla-wgs build-mask`
  computes the repeat mask of the panel from a genome FASTA. No IPD-IMGT/HLA data is shipped.
- Output schema `genome.hla.v1`.
