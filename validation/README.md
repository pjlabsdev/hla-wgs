# Validation

All data are public. The evaluation is the Rust program `hla-wgs-validate` (`src/bin/hla-wgs-validate/`,
built with `cargo build --release` next to `hla-wgs`; `hla-wgs-validate` alone lists the commands).
It and the shell scripts expect a working directory (`HLA_WORK`, default: current directory) with
`hla-wgs` and `hla-wgs-validate` on `PATH`, samtools, and a reference built with `hla-wgs build-db` /
`build-mask` in `bdtest/`. No Python is needed; the commands were checked to give byte-identical
output to the Python scripts they replace.
The 1000 Genomes CRAMs are read remotely; only the MHC region, ALT/HLA contigs and unmapped reads
are fetched.

| folder | content |
|---|---|
| `hprc/` | truth from HPRC release 2 haplotype assemblies: `fetch_r2.sh` (assemblies, MHC contigs), `hla-wgs-validate truth-exon x|y` and `truth-table x|y` (exon truth), `truth4` (nearest full-gene allele and differences per haplotype), `extract_sr.sh` (reads from the 1000 Genomes 30x CRAM), `hla-wgs-validate eval6` (exact four-field evaluation; its table is stored as `results/*/eval6.tsv`), `eval23` (two- and three-field agreement), `run_t1k.sh` (T1K with the same reads) |
| `giab/` | GIAB trios HG002/3/4 and HG005/6/7: `run23.sh`, `hla-wgs-validate trio23` (Mendelian consistency) |
| `lab/` | 1000 Genomes laboratory typing (Gourraud et al. 2014; Abi-Rached et al. 2018): `run24.sh`, `hla-wgs-validate eval24`. The frequency prior was counted from the same laboratory data, so the samples are split in two folds (`folds.tsv`) and each sample is typed with the prior of the other fold |
| `results/dev80/` | `truth4.tsv` and `truth4diff.tsv`: truth for the 80 HPRC samples used during development |
| `results/check30/` | the same for 30 further samples (`hprc/samples_check30.txt`, run with `hprc/ho4.sh`) that were not used for the four-field rules; they had been the first held-out round of the two-field typer |
| `results/check30_v1.1/` | the same 30 samples typed again with the 1.1 development version, with a comparison to 1.0.0 |
| `results/hgsvc3_v1.1/` | 59 unseen HGSVC3 samples (assembly truth), 1.1 development version |
| `results/dev80_v1.1/` | the 80 development samples with the 1.1 development version, including the eight added genes |
| `results/heldout_vs_t1k/` | `strata.tsv`: hla-wgs and T1K against assembly truth by population and gene, 150 held-out samples of rounds 1 to 3 (round 4 added 34 samples: two fields 543/543 against 541/543, copy number 102/102 against 64/102) |
| `results/lab1000g/` | 1101 laboratory-typed 1000 Genomes samples: all calls (`calls.tar.gz`), `eval24` output for the 2018 and 2014 laboratory sets, the differences checked one by one |
| `results/long26_main/` | long-read (HiFi) calls of the 26 samples of `results/long26/` with the current development version; `hla-wgs-validate long` output (consensus against assembly, exact and nearest-allele calls) |
| `results/depth_lead4/` | certain two- and three-field calls at 30x, 15x and 10x before and after the lead-of-4 rule; `hla-wgs-validate eval-res` output |
| `results/giab/`, `results/lab/` | calls of the trio members and of the 45 laboratory-typed samples |

## Commands of hla-wgs-validate

| Command | What it does |
|---|---|
| `eval23 DIR` | two- and three-field agreement with the assembly truth (`T4OUT`) |
| `eval6 DIR [v]` | exact evaluation of the 18 genes against both haplotypes |
| `eval8 DIR [v]` | the same for the eight added genes (DMA to TAP2; truth `T4OUT=truth/truth8.tsv`) |
| `eval8-exons DIR [GENES] [v]` | exon-level check of calls certain to three or more fields |
| `eval24 [2014\|2018\|any]` | 1000 Genomes laboratory types against hla-wgs and T1K |
| `trio23 [DIR]` | Mendelian consistency of the two GIAB trios |
| `long DIR SAMPLES.txt [v]` | long-read results: consensus against assembly, exact and nearest calls |
| `eval-res DIR` | certain calls right at two and three fields (depth study) |
| `sim-eval TRUTH.tsv ASSIGN.tsv` | gene assignment of simulated read pairs |
| `truth4`, `truth8` | best database alleles per haplotype and gene |
| `truth-exon x\|y\|z FA...` | exon projection of the database alleles onto haplotype assemblies |
| `truth-table x\|y\|z` | `truth/truthX.tsv` from the exon projections |
| `mhc-crop FA...` | MHC window of the MHC contigs (HLA-A to DPB1 plus 300 kb) |
| `allele-vs-assembly S HAP GENE PREFIX...` | one allele against one assembled haplotype |
| `mhc-contigs OUT.fa < ASSEMBLY.fa` | the MHC contigs of an assembly |
| `sim DBDIR OUTPREFIX [--per N] [--cov X] [--seed S] [--profile P] [--div D]` | simulated read pairs from database alleles |
| `thin-pairs IN_PREFIX FRAC OUT_PREFIX` | a random fraction of the read pairs (depth study) |
| `build-freq OUT.json EXCLUDE...` | the two-field frequency prior from the laboratory types |
| `paper [--version V] [--date D] [--doi DOI] [--typ OUT.typ]` | the software report as PDF (Typst) on stdout; needs `--features paper` |

Environment variables:

| Variable | Meaning |
|---|---|
| `HLA_WORK` | work folder (default: current folder) |
| `HLA_DB` | database for the truth commands (default `HLA_WORK/db/hladb.json.gz`) |
| `HLA_DAT` | `hla.dat` for the exon features (default `HLA_WORK/hla.dat`) |
| `HLA_OUT` | calls for `eval24` (default `HLA_WORK/lab/out24`) |
| `HLA_LTAG` | `long`: read `DIR/SAMPLE.TAG.long.*` |
| `HLA_RESULTS` | `paper`: result folder (default `validation/results`) |
| `SAMPLES`, `TRUTHSRC`, `T4OUT`, `T8OUT` | sample list, truth sources and truth tables of the truth and evaluation commands |
| `DUMP` | `eval6`, `eval8`: file for one row per call |

## How the truth is defined

For each assembled haplotype the gene sequence is cut out and compared with every IPD-IMGT/HLA
allele of the gene at full genomic length. The truth is the nearest allele plus the list of
differences. A haplotype with differences carries an allele that is not in the database; a typer can
then at best name the nearest allele and report the differences. A four-field call counts as exact
only if the assembled sequence is identical to the named allele.

This definition favours shorter database entries when two alleles are equally near, which explains
single truth entries such as a null allele whose own sequence is identical to the common allele.

## Reading the four-field table

`hla-wgs-validate eval6` prints per gene: haplotypes, exact calls, known alleles and how many of them were called
exactly, new alleles, alleles reported as certain at four fields (`safe4`) and how many of those are
exact, and how many reported new sites are true.
