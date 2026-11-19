# Changelog

## Unreleased (1.1.0)

- New sites: read pairs that a related gene explains better (HLA-Y, -J, -K beside HLA-A; DQA2 beside DQA1) are left out when sites are collected, as they already were when alleles are scored. On the 80 development samples this removed 7 false sites and no true one; four more alleles (three HLA-A, one DQA1) are reported certain at four fields, all exact (1719 of 1740, before 1715 of 1736).
- Input errors are reported at once and in plain words (`error: ...`, exit code 1) instead of after six retries or with a panic trace: database not found, input file missing or empty, BAM/CRAM index missing, not a BAM, FASTQ truncated or not a FASTQ, R1 and R2 of different length, output folder not writable (checked before reading). A BAM that ends early is reopened once and reported if it ends at the same record again. Retries remain for real I/O errors (disk dropping out).
- HLA-E is reported to four fields like the other genes (was capped at three); its fourth field was exact in 274 of 275 alleles reported certain (80 HPRC, 30 check and 38 HGSVC3 samples).
- `PREFIX.tsv`: `call1`/`call2` now hold the safe call (the fields the reads support) instead of repeating
  `allele1`/`allele2`; a gene estimated at 0 copies shows `-` in all allele columns, has no line in
  `PREFIX.conf.tsv` and no alleles in `PREFIX.json` (before, the nearest names were still printed).
- start file: the reference lives in `ref/` next to it; also fetches the WMDA G/P group files of the same release and takes the repeat mask from `data/panel_mask_3.66.bed` (allele names and intervals only, no sequence) instead of asking for a GRCh38 FASTA; the downloaded hla.dat is removed after the build. `PREFIX.json` has `repeat_mask`, and the result page warns when no mask was used.
- `PREFIX.html`: the result as one page for reading and printing (safe call per allele, supported fields, notes, pharmacogenetic alleles, DOI and link to the research article); German and English in one page with a switch (opens in German with `HLA_WGS_LANG=de` or a German `LANG`); PJ Labs design (IBM Plex Mono embedded, OFL 1.1, light and dark), result and details on two tabs; "Details per gene" shows everything from `PREFIX.tsv` and `PREFIX.conf.tsv` (leads per field with threshold and competitor, G/P groups, every differing site) and why each call stops at its number of fields.
- Start file `start/hla-wgs-start.command`: double-click, drag the input file into the window or type its path; sets up the reference once if it is missing.
- Long reads (experimental): `hla-wgs long DBDIR READS.fastq[.gz]|READS.bam PREFIX [threads]` types from
  PacBio HiFi reads. The gene sequence of each haplotype is built from the reads (consensus) and
  compared with every full-length allele; `PREFIX.long.tsv` gives the nearest allele and the number of
  differences, `PREFIX.long.fa` the sequences. On 26 HPRC samples (40x HiFi, MHC region) 689 of 726
  sequences of the 18 genes were identical to the assembled haplotype, and all 546 reported as exact
  were; about one minute per sample on 4 cores. The
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
