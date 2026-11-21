# hla-wgs

[![DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.24367470.svg)](https://doi.org/10.5281/zenodo.24367470)

**HLA typing from short-read whole-genome sequencing, with a statement of what is certain for
every allele.** One Rust program, no external aligner or interpreter at run time. It types 18 genes
(A, B, C, DRB1, DRB3, DRB4, DRB5, DQA1, DQB1, DPA1, DPB1, DRA, E, F, G, H, K, V) from FASTQ, BAM or
CRAM, reports copy number for DRB3/4/5, H and K, and lists the positions at which the reads differ
from the named allele, so that a new allele is flagged instead of silently reported under the name
of its nearest neighbour.

> **Research use only (RUO).** Results must not be used for transplantation, pharmacogenetic or any
> other medical decision without confirmation by an accredited HLA laboratory.

## What is different

Most typers print one four-field name per haplotype. In 80 haplotype-resolved HPRC genomes, 17 % of
all gene copies (422 of 2,489) carry an allele that is not in IPD-IMGT/HLA 3.66 at full gene length,
so a four-field name is frequently only the nearest database entry. hla-wgs therefore reports three
things per allele:

1. `allele`: the best-matching database allele.
2. `safe_call` and `resolution`: the name cut to the number of fields that the reads actually
   support (1 to 4). A call is cut when a competing allele is close, when reads consistently differ
   from the named allele, or when part of the allele is not covered.
3. `new_sites`: the positions and changes that the reads show against the named allele. If the
   named allele plus these changes equals another database allele, the call is renamed to it.

## Comparison with T1K

Measured on the same extracted reads of 1000 Genomes 30x samples whose HPRC release 2 assemblies
give the truth. Each hla-wgs version was frozen before the samples of its round were looked at
(184 samples in four rounds). T1K was run with its default settings and the same IPD-IMGT/HLA
release.

| | hla-wgs | T1K 1.0.10 |
|---|---|---|
| Two-field accuracy, A, B, C, DRB1, DQA1, DQB1, DPA1, DPB1, 184 held-out samples | 2,937 / 2,941 (99.86 %) | 2,921 / 2,941 (99.32 %) |
| DRB3/4/5 copy number (0, 1 or 2), same samples | 551 / 552 | 348 / 552 |
| Wall time, 4 cores, extracted MHC reads of a 30x genome | about 40 s | about 150 s |
| Peak memory | 1.4 GB | 3.2 GB |
| Per-allele certainty | resolution 1 to 4 with margin and nearest alternative | quality score |
| New alleles | positions and changes listed, call cut accordingly | variants of the allele in a VCF |
| Genes | 18 incl. DRA, E, F, G, H, K, V | all genes of the chosen reference |
| Other gene families | HLA only | also KIR |
| RNA-seq | no | yes |

The held-out rounds were run with the prototype of the same algorithm (Python driver with the Rust
kernel, versions 8 to 16); the 1.0.0 program reproduces them. T1K does more than this table shows
(KIR, RNA-seq, single cell); the comparison covers only HLA from WGS.

## Validation in short

All scripts and result tables are in [`validation/`](validation/).

| Test | Result |
|---|---|
| 80 HPRC samples, truth from both assembled haplotypes, 18 genes | two fields 2,486 / 2,489, three fields 2,482 / 2,489 |
| Same samples, alleles reported as certain at four fields | 1,569 of 1,591 exactly equal to the assembled gene sequence (98.6 %) |
| 30 further HPRC samples not used for the four-field rules | 607 of 621 certain four-field alleles exact (97.7 %) |
| Same samples, copy number of DRB3/4/5, H and K | all correct |
| 45 further 1000 Genomes samples with laboratory typing (A, B, C, DRB1, DQB1) | 438 / 450 at two fields; of 12 differences 5 are resolution limits of the laboratory method, 3 are probable laboratory errors (B\*27:05 against 27:03), 4 are open |
| GIAB trios (Ashkenazi, Han Chinese), 60x to 100x | no Mendelian conflict at two or three fields in any gene; none at four fields among calls reported as certain |

With the 1.1 development version (this branch):

| Test | Result |
|---|---|
| 80 HPRC samples, 18 genes | two fields 2,486 / 2,489, three fields 2,482 / 2,489; certain at four fields 1,752, of these 1,731 exact (98.8 %) |
| 59 HGSVC3 samples never used at any stage, assembly truth, 18 genes | two fields 1,833 / 1,833, three fields 1,830 / 1,833; certain at four fields 1,206, of these 1,191 exact (98.8 %) |
| Eight added genes (DMA, DMB, DOA, DOB, MICA, MICB, TAP1, TAP2), 80 HPRC samples | calls certain to three fields or more with every exon equal to the assembly: 988 / 999; fourth field DMA 153 / 153, DOA 112 / 115, DOB 134 / 136, TAP1 40 / 41; DMB and TAP2 are reported to three fields at most, MICA and MICB do not reach four |
| Same 80 samples thinned to 20x, 15x, 10x | two fields 2,485 / 2,481 / 2,476 of 2,489; certain at four fields 1,660 / 1,539 / 1,082, of these exact 98.8 / 98.6 / 97.9 % (`validation/results/depth`) |
| Second assembly truth (Lai et al. 2024), 40 shared samples, 11 genes | two fields equal in 489 / 492 allele calls of the two truths; no difference traced to an error of ours (`validation/results/lai2024`) |

The figures below belong to release 1.0.0. The 22 certain four-field calls that are not exact: 9 differ only in the length of a homopolymer or
short tandem repeat, 4 are phase swaps at the 3' end of HLA-B that short reads cannot resolve, 9
are single missed variants in other genes.

The 80 HPRC samples were used to develop the four-field logic. On 30 further HPRC samples that played
no part in developing these rules (they had been the first held-out round of the two-field typer),
607 of 621 alleles reported as certain at four fields were exact (97.7 %), two fields 935 / 935 and
three fields 934 / 935. The test on genomes never used at any stage is the HGSVC3 row above
([`validation/results/hgsvc3_v1.1/`](validation/results/hgsvc3_v1.1/)); the frequency table may
include laboratory types of some of these samples.

The full report is in [`docs/paper/hla-wgs-report.pdf`](docs/paper/hla-wgs-report.pdf).

## Install

```bash
git clone https://github.com/pjlabsdev/hla-wgs && cd hla-wgs
cargo build --release          # Rust 1.85 or newer; a C compiler is needed for minimap2
./target/release/hla-wgs --version
```

Runs on Linux and macOS (x86-64 and ARM).

## Start file

`start/hla-wgs-start.command` is a double-clickable start file (on macOS it opens in Terminal; on
Linux run it with `bash`). It asks for the input; drag the BAM, CRAM or FASTQ file into the window,
which inserts its path, or type the path, then press Enter. If the reference is missing it sets it up
once in `ref/` next to the start file (download of hla.dat 3.66, `build-db`, the WMDA G/P files and
the repeat mask `data/panel_mask_3.66.bed`; `HLA_WGS_DB` points to another one), then runs
hla-wgs into `~/Downloads/hla-wgs-<sample>` (a new folder if one with results exists) and shows the
result table with an animated progress display, and opens the result page `PREFIX.html`. For PacBio HiFi reads start it with `--long` (or `HLA_WGS_LONG=1`). It looks for the `hla-wgs` program next to itself, in `../target/release` or in PATH.

Another IPD-IMGT/HLA release in one step: start it with `HLA_WGS_RELEASE=3.67` (any release
number). It then builds `ref/hladb_3.67` beside the 3.66 reference and carries the repeat mask over
from it; the 3.66 reference is set up first if it is missing. Validation used release 3.66; the
result page and the JSON name the release of the database that was used.

## Build the reference once

The IPD-IMGT/HLA database is licensed CC BY-ND and is **not** part of this repository. Download
`hla.dat` yourself and build the reference locally:

```bash
curl -LO https://ftp.ebi.ac.uk/pub/databases/ipd/imgt/hla/hla.dat.zip && unzip hla.dat.zip
hla-wgs build-db hla.dat data/freq_1000g_2field.json hladb          # about 4 minutes
hla-wgs build-mask hladb GRCh38.fa                                   # about 1 minute
```

`build-db` writes `hladb/hladb.json.gz` and `hladb/panel.fa`. `build-mask` marks the parts of the
panel that also occur outside the MHC in the genome (repeats), so that reads from elsewhere are not
counted; it writes `hladb/panel.fa.mask.bed`. The genome is assumed to be GRCh38; for another
assembly pass a BED file with the MHC region to exclude as third argument. Validation used
IPD-IMGT/HLA release 3.66.

A database of another release does not need the genome again:
`hla-wgs build-mask hladb_new --from hladb` carries the mask over. Alleles with unchanged sequence
keep their intervals; for new or changed alleles, the positions whose 21-mers lie inside masked
intervals of the older panel are marked and joined by the same rules.

Optional: copy `wmda/hla_nom_g.txt` and `wmda/hla_nom_p.txt` of the same release into `hladb/` to get
G and P groups in the JSON output, and `wmda/rel_dna_ser.txt` with `wmda/rel_ser_ser.txt` for the
serological antigens (the start file fetches all four).

`hla-wgs html PREFIX.json` writes the result page again from the JSON, for example after an update.

`data/freq_1000g_2field.json` holds two-field allele frequencies of A, B, C, DRB1 and DQB1 counted
from the public 1000 Genomes laboratory typing (`data/build_freq.py`). They serve as a weak prior
between otherwise equal candidates.

## Run

```bash
# paired FASTQ (whole genome, or reads extracted from the MHC plus unmapped reads)
hla-wgs hladb sample_R1.fq.gz sample_R2.fq.gz out/sample 4

# BAM or CRAM, coordinate-sorted and indexed, GRCh38 or GRCh37
hla-wgs hladb sample.bam out/sample 4
hla-wgs hladb sample.cram out/sample 4 GRCh38.fa
```

From BAM/CRAM the program reads the MHC region (GRCh38 chr6:28,510,120-33,480,577 or GRCh37
6:28,477,797-33,448,354), all ALT and HLA contigs, and the unmapped reads. The last argument is the
number of threads.

### Long reads (experimental, 1.1)

```bash
hla-wgs long hladb sample.hifi.bam out/sample 4      # indexed BAM: the MHC region is read
hla-wgs long hladb mhc_reads.fastq.gz out/sample 4
```

For PacBio HiFi reads. A read of 10 to 25 kb usually holds a whole gene, so the sequence of each
haplotype is built from the reads and compared with every full-length allele. This also reaches what
short reads cannot: the Alu cluster and the long repeats in DRB1, DRB3 and DRB4, although the exact
length of the longest repeats stays the weak point (marked as unsure sites). Output:
`PREFIX.long.tsv` and `PREFIX.long.fa`, see [`docs/output.md`](docs/output.md). Tested on PacBio HiFi
(26 samples); see `validation/results/long26/`.

## Output

`PREFIX.tsv` (one line per gene), `PREFIX.conf.tsv` (one line per allele with certainty) and
`PREFIX.json` (schema `genome.hla.v1`). Details: [`docs/output.md`](docs/output.md).

```
gene  slot  allele            safe_call         resolution  margin2  margin3  margin4  alt4              new_sites  new_detail
A     1     A*02:06:01:01     A*02:06:01:01     4           41.9     40.6     39.5     A*02:06:01:16     0          -
DPB1  1     DPB1*01:01:01:02  DPB1*01:01:01     3           29.8     29.8     29.8     DPB1*01:01:12     1          3468:T>A:20/33:either
```

## How it works

1. Reads are mapped to a panel of one full-length sequence per allele group (minimap2 library);
   reads anchored only in masked repeats are dropped.
2. Per gene, a k-mer likelihood with expectation maximisation selects candidate alleles.
3. Candidates are compared pairwise on an edit-distance matrix of all reads, which gives the best
   pair and, for each field, the margin to the best competitor.
4. Exon-level refinement and a joint copy-number model for DRB1/3/4/5, H and K.
5. Reads are realigned to the two called alleles; consistent differences become `new_sites`, and
   stretches without any read are reported as blind.
6. `resolution` is the number of fields whose margin passes the threshold and that no new site or
   blind stretch contradicts.

## Limits

- Short reads only (2x100 to 2x150, about 15x or more; validated at 30x and above). No RNA-seq, no
  exome or amplicon data, no long reads.
- A four-field call reported as certain was exact in 98.6 % (development samples) and 97.7 % (30
  further samples) of cases, not in all. Homopolymer and
  repeat lengths and the phase of variants at the far 3' end of HLA-B are the known weak points.
- A variant further from the next heterozygous position than a read pair reaches cannot be assigned
  to a haplotype; such sites are marked `either`.
- Laboratory truth exists only for A, B, C, DRB1 and DQB1 at two fields; all other statements rest
  on assemblies of HPRC individuals.
- GRCh38 is assumed for `build-mask`.

## Licence and data

Code: MIT, see [LICENSE](LICENSE). The IPD-IMGT/HLA database (Barker et al. 2023, Nucleic Acids
Research 51:D1053) is the work of the Anthony Nolan Research Institute and EMBL-EBI under
CC BY-ND 3.0; this repository contains no sequences from it, only allele names in result tables.

## Citation

Jungwirth P. hla-wgs: HLA typing from whole-genome sequencing with a per-allele statement of what is
certain. Version 1.0.0, 2026. https://doi.org/10.5281/zenodo.24367470

See also [CITATION.cff](CITATION.cff).
