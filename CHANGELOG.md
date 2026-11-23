# Changelog

## Unreleased

- Long reads (`hla-wgs long`, PacBio HiFi): the consensus takes at every position the variant that most reads carry, also without an absolute majority. In single-base runs the reads split over several lengths, so the right length often had less than half of the reads and the template read's own error stayed. On 26 HPRC samples 1409 instead of 1387 of 1504 gene sequences are equal to the assembly, and 602 instead of 598 four-field calls are exact (the 2 wrong ones are unchanged). `HLA_LPLUR=0` restores the old rule. Read groups are now ranked by the reads that span the whole gene before the number of reads, so a mixture of reads no longer pushes a real haplotype to "minor" (HG02630 HLA-C, HG01358 DPB1; 1411 of 1504). `HLA_LSPAN=0` restores the old order.
- More precise DRB1: the exon refinement of DRB1 now also sees the reads that the panel gave to DRB3, DRB4 and DRB5, which it does for the exon fields only. Without them, DRB1*09:01 was mostly reported as the group "DRB1*09", and carriers of 01:01/07:01 got no group at all. On the 80 HPRC validation samples 13 DRB1 calls and on the 59 unseen HGSVC3 samples 11 are one or two fields deeper, all of them equal to the assembly; every other value of the evaluation is unchanged. In the 1000 Genomes lab comparison most of the 133 DRB1 calls without a safe second field were DRB1*09:01 or 01:01/07:01 carriers. `HLA_DRBREL=0` switches it off.
- Faster typing, same results: the k-mer index of the edit-distance kernel (built about a hundred times per sample) is sorted as packed 64-bit values with a radix sort. HG00438 on one core: 46.5 s instead of 52 s; output byte-identical on the 80 validation samples.
- Faster, same results: the flanks that shorter candidate alleles borrow from longer ones are computed in parallel and kept in `pad_cache.tsv.gz` next to the reference, so from the second run on a reference these alignments are not repeated (HG00438 on one core: 74 s first run, 55 s later runs). Genes with the most reads are typed first, so that a slow gene does not end a run alone. Eight HPRC samples with 4 threads: 133 s instead of 179 s. Output on the 80 validation samples is byte-identical, except that two new sites at the same position are now always listed in the same order (before it could change between runs).
- `hla-wgs unzip` checks the length and CRC-32 of every entry (a damaged `hla.dat.zip` is refused instead of building a wrong reference) and refuses names with a drive letter or a leading backslash on Windows.
- The validation and the software report no longer need Python: `hla-wgs-validate` (built with `cargo build --release`) replaces the evaluation, truth, simulation and report scripts with byte-identical output, including `hla-wgs-validate paper` for `docs/paper/hla-wgs-report.html`. The shell scripts call it; the Python files are removed.

## 1.1.3 (2026-11-22)

- Fresh systems: the start file and the installer need neither curl nor unzip. They download with curl or, where it is missing (some minimal Linux installations), with wget; a ZIP is unpacked by `unzip` or, without it, by hla-wgs itself (new command `hla-wgs unzip ZIP DIR`). Installer line for systems without curl: `wget -qO- https://github.com/pjlabsdev/hla-wgs/releases/latest/download/install.sh | bash`.
- The macOS and Linux packages are also attached as `.tar.gz` (`hla-wgs-macos.tar.gz`, `hla-wgs-linux-x86_64.tar.gz`); the installer takes them when `unzip` is missing.

## 1.1.2 (2026-11-22)

- Installer for macOS and Linux: `curl -fsSL https://github.com/pjlabsdev/hla-wgs/releases/latest/download/install.sh | bash` installs the package of the latest release to `~/hla-wgs` (keeping `ref/` and results) and starts it. Fetched in Terminal, the files carry no quarantine mark, so macOS opens them without the "could not verify" warning.
- The macOS start file removes the quarantine mark from its whole folder at the first start, so after one approval it opens with a double-click. The instructions name the macOS 15 way of approving it (Privacy & Security, Open Anyway).
- Start files: when the program is missing next to them (for example after downloading the "Source code" ZIP of a release, which holds no program), they offer to download the ready package for this system from the latest release and continue. Otherwise they name the release page and `cargo build --release`. The Windows start file also finds a build in `..\target\release`.

## 1.1.1 (2026-11-22)

- Start file (macOS, Linux): it stopped at once when `LANG` was not set (possible on Linux and in a macOS Terminal without locale variables). On Linux it now uses the `C.UTF-8` character set instead of `en_US.UTF-8`, which most Linux systems lack.
- The start files pass the language they chose to the result page in both directions (`HLA_WGS_LANG=en` as well as `de`).
- Packaging: the release workflow checks the program by its exact name.

## 1.1.0 (2026-11-22)

- Own aligner: the reads are now aligned by the program itself (`src/aln.rs`, Rust) instead of the minimap2 library. hla-wgs is one Rust program without C code; building needs no C compiler. Same accuracy on the validation sets (see `docs/aligner.md`): 80 HPRC samples two fields 2485 of 2489, three fields 2481, certain four-field calls 1779 of 1798 exact (minimap2 1778 of 1801); 59 HGSVC3 samples two fields 1833 of 1833, three fields 1830, certain four-field calls 1333 of 1349 exact (minimap2 1337 of 1354), i.e. one wrong certain call fewer. Faster: 22 s instead of 32 s per sample (CPU 79 s instead of 116 s; 8 HPRC samples, 4 threads, two rounds), same peak memory (1.4 GB).
- A database allele that is 9 to 20 bases shorter than its group now also gets the flank of a longer allele when alleles are compared (before only from 21 bases). Reads at the end of the longer allele otherwise overhang the shorter one for free. Found on HG00514 (A\*02:01:01:48, 18 bases shorter at the 3' end). `HLA_PAD_MIN` sets the limit.
- Ready-to-run packages for macOS (Apple silicon and Intel), Linux x86_64 and Windows x86_64 are attached to every release (`hla-wgs-macos.zip`, `hla-wgs-linux-x86_64.zip`, `hla-wgs-windows-x86_64.zip`): program, start file, README in English and German, frequency table and repeat mask. No IPD-IMGT/HLA data is packed; the start file builds the reference on the first start.
- Start file for Windows (`hla-wgs-start.bat`, which runs `hla-wgs-start.ps1` in PowerShell), with the same steps and display as on macOS and Linux. On Linux the start file opens the result page with `xdg-open`.
- The start files check at every start whether a newer release exists on GitHub (5 s limit, silent without a connection) and offer to install it; the update replaces the program files and keeps `ref/` and earlier results. `HLA_WGS_NO_UPDATE=1` switches the check off.
- The start files speak English, or German on a German system; `HLA_WGS_LANG=de|en` chooses. Sample names taken from file names keep only letters, digits, dot, underscore and hyphen.
- CI builds and tests on Linux, macOS and Windows.

- The first-described allele of a three-field group (fourth field 01, mostly the common one) is certain at four fields with a lead of 1 instead of 4 over its nearest sibling. 80 HPRC samples: 1778 of 1801 certain four-field calls exact (before 1740 of 1761); 59 unseen HGSVC3 samples: 1337 of 1354 (before 1315 of 1331). `HLA_F401=0` restores the old threshold.
- A database base is no longer called unsupported where the stretch has fewer than half the usual reads (a repeat piece whose reads the mask removes, such as the L1 piece in intron 1 of DPB1) and more than one read of the allele itself backs it. On 80 HPRC samples: certain four-field calls 1740 of 1761 exact (before 1731 of 1752), 14 fewer false sites, no true site lost. `HLA_UNSTHIN=0` restores the old rule.
- Result page in the form of a laboratory report: a head with sample, report number, input files, date, software, database, depth and a quality line; a clinical summary of the eleven loci of a clinical typing (A, B, C, DRB1, DRB3/4/5, DQA1, DQB1, DPA1, DPB1) at two fields with G group, WHO serological antigen and natural killer cell receptor ligand; the GL String to copy; pharmacogenetics as positive or negative with the CPIC recommendation in one sentence; resolution in words; new alleles with the nearest allele and whether the protein changes; non-classical genes, pseudogenes, MIC, TAP and HFE folded away; method and limits; A4 printing with page numbers, sample and report number on every page, printing the tab that is open (the report alone is about six pages); drug names in German on the German page; info boxes on hover, focus or tap with the measured values (lead per field against its threshold and competitor, groups, reason for the number of fields, sites with reads and depth, reads and lead of the gene, how depth, quality and report number are made, database files loaded); a field is ticked only when it is supported, the threshold shown is the one that applied (1 for a fourth field 01), a gene limit is named as the reason, and only the fields of the allele name are listed. The JSON lists the fourth-field-01 threshold as `4_01`; `hla-wgs html` on an older 1.1 JSON without it infers the threshold of 1 for the whole run when any supported fourth field 01 has a lead below 4. No disease associations.
- JSON: `serology` and `serology_assumed` per allele from the WMDA files `rel_dna_ser.txt` and `rel_ser_ser.txt` (the start file fetches them), `ligand` (Bw4-80I/T, Bw6, C1, C2) from the protein, `input`, `analysis_time`, `serology_loaded`.
- `hla-wgs html PREFIX.json` writes the page again from the JSON.
- `depth_estimate` in the JSON: a rough sequencing depth over the HLA genes from the read pairs of ten always-diploid genes. Below 18x the result page says that fewer fields are supported and that certain fourth fields are somewhat less often exact (validation at 10x: 97.9 % against 98.8 % at 30x). A rule that withholds the fourth field by depth was tested and not taken: at 10x it would withhold about 27 exact calls for each wrong one.
- A new site that read pairs specific to one allele carry, and that pairs specific to the other allele cover without carrying it, belongs to the first allele even when most reads show it; before, it was listed as "this allele" on one copy and "either allele" on the other. On 80 HPRC samples: certain four-field calls 1731 of 1752 exact (before 1728 of 1749), 10 fewer false sites.
- Low depth: a new site needs 5 reads at the usual depth, 4 where the allele is covered by fewer than 18 reads and 3 below 12. At 10x a variant of one copy is carried by about five reads, so about half of them were missed and the fourth field looked certain. Certain four-field calls that are not exact on 80 HPRC samples: 10x 23 instead of 36, 15x 22 instead of 27, 20x 20 instead of 22; 30x unchanged (1728 of 1749 exact).
- The allele group is no longer named as certain when an allele of another group explains the reads about as well (lead at field 2 below 1): `resolution` 0, `safe_call` is the gene name alone. Found in the depth study on HG004, where TAP2 at 20x named group 01 instead of 02. On 80 HPRC samples such near ties named the wrong group in 3 of 7 alleles at 30x.
- The release of an existing database is read from the header of `hla_nom_g.txt` when `DBDIR/release` is missing, so references built before this change also name it.
- Depth study: the 80 HPRC samples thinned to 20x, 15x and 10x (read pairs chosen by a hash of their name), and the GIAB trio and one private sample at the same depths; see `validation/results/depth`.
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
