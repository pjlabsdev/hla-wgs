# Changelog

## Unreleased (1.1.0)

- Another IPD-IMGT/HLA release in one step: the start file takes `HLA_WGS_RELEASE=3.67`, builds that release beside the 3.66 reference and carries the repeat mask over with the new `hla-wgs build-mask DBDIR --from OLDDBDIR` (no genome needed; unchanged alleles keep their intervals). `build-db` writes the release it found in hla.dat to `DBDIR/release`; the JSON (`database_release`) and the page name it. Checked by building release 3.65 and carrying the 3.66 mask over: six development samples gave the same calls in all 26 genes as with 3.66 except three MICB alleles, two of them alleles that exist only in 3.66.
- New sites of an allele that lie where the other called allele has only borrowed flank sequence (a short database allele extended with the flanks of a longer one) are no longer listed for that allele when no read pair of its own carries them. On the 80 development samples: certain four-field calls exact 1728 of 1749 (before 1726 of 1747), four fewer false sites in the 18 genes.
- New alleles: `PREFIX.novel.tsv` lists every difference of a candidate allele with its region (exon, intron, untranslated end), its position in the coding sequence and its effect on the protein (synonymous, missense, stop, frameshift, splice site), the supporting reads and whether the sequence is complete and assigned to one copy. It is meant as a worksheet for confirming a candidate and preparing a submission to IPD-IMGT/HLA. The JSON and the page show region and effect per new site.
- DMB and TAP2 are reported to three fields at most: their fourth field equalled the assembly in only 88 of 106 and 38 of 44 alleles that would have been reported certain (80 HPRC samples), against 153 of 153 for DMA, 112 of 115 for DOA, 134 of 136 for DOB and 40 of 41 for TAP1. JSON: `gene_cap` per allele says when this limit applied; the result page names it as the reason.
- Database: short alleles of DMA, DMB, DOA, DOB, MICA, MICB, TAP1 and TAP2 now get the flanks of the nearest full-length allele, as the other genes always did. Before, a longer allele could win only because it covers more reads. On the 80 development samples TAP2 calls certain to three fields with all exons exact rose from 100 of 114 to 116 of 122, the eight genes together from 965 of 984 to 988 of 999; the 18 other genes are unchanged. The database must be rebuilt (`build-db`); it carries a format number (`DBDIR/format`, `hla-wgs --db-format`), the program says when a database is older, and the start file rebuilds it. Panel and repeat mask are unchanged.
- First check of the eight added genes against the assemblies (80 HPRC samples, exon level, calls certain to three fields or more): DMA 159/159, DMB 158/158, DOA 155/156, DOB 157/158, MICB 65/65, MICA 66/68, TAP1 112/113, TAP2 116/122. Their fourth field is less reliable than for the classical genes (662 of 818 calls reported at full resolution equal the assembly over the whole gene).
- FASTQ input is faster: the 21-mer prefilter no longer uses 21-mers that lie inside the repeat mask (Alu, L1 and other copies found all over the genome). Pairs with only such seeds were aligned and then dropped by the mask rule anyway. On the 80 development samples no allele, safe call or resolution changed and the accuracy figures are identical; single read counts of sites differ in 31 samples. `HLA_FULLKMERS=1` restores the old prefilter.
- New sites are assigned to one allele where the reads allow it: a site in a stretch where both alleles are identical used to be listed for both ("either"). It now goes to the allele whose own read pairs carry it, or whose assigned sites the same pairs carry along with it; the other allele loses it. On the 80 development samples the reported sites that are not in the assembly fell from 246 to 158, ten more true sites are attributed correctly, and seven more alleles are reported certain at four fields, all exact (1726 of 1747).
- `build-db` reports a file that is not an hla.dat instead of aborting with a trace.

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
