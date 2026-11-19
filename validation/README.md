# Validation

All data are public. Scripts are the ones that were run; they expect a working directory
(`HLA_WORK`, default: current directory) with `hla-wgs` on `PATH`, samtools, minimap2 with the
`mappy` Python module, and a reference built with `hla-wgs build-db` / `build-mask` in `bdtest/`.
The 1000 Genomes CRAMs are read remotely; only the MHC region, ALT/HLA contigs and unmapped reads
are fetched.

| folder | content |
|---|---|
| `hprc/` | truth from HPRC release 2 haplotype assemblies: `fetch_r2.sh` (assemblies, MHC contigs), `truth_exonx.py`, `truth_exony.py`, `truth_tablex.py`, `truth_tabley.py`, `truth4.py` (nearest full-gene allele and differences per haplotype), `extract_sr.sh` (reads from the 1000 Genomes 30x CRAM), `eval6.py` (exact four-field evaluation), `eval23.py` (two- and three-field agreement), `run_t1k.sh` (T1K with the same reads) |
| `giab/` | GIAB trios HG002/3/4 and HG005/6/7: `run23.sh`, `trio23.py` (Mendelian consistency) |
| `lab/` | 1000 Genomes laboratory typing (Gourraud et al. 2014; Abi-Rached et al. 2018): `run24.sh`, `eval24.py`. The frequency prior was counted from the same laboratory data, so the samples are split in two folds (`folds.tsv`) and each sample is typed with the prior of the other fold |
| `results/dev80/` | `truth4.tsv` and `truth4diff.tsv`: truth for the 80 HPRC samples used during development |
| `results/heldout_vs_t1k/` | `strata.tsv`: hla-wgs and T1K against assembly truth by population and gene, 150 held-out samples of rounds 1 to 3 (round 4 added 34 samples: two fields 543/543 against 541/543, copy number 102/102 against 64/102) |
| `results/giab/`, `results/lab/` | calls of the trio members and of the 45 laboratory-typed samples |

## How the truth is defined

For each assembled haplotype the gene sequence is cut out and compared with every IPD-IMGT/HLA
allele of the gene at full genomic length. The truth is the nearest allele plus the list of
differences. A haplotype with differences carries an allele that is not in the database; a typer can
then at best name the nearest allele and report the differences. A four-field call counts as exact
only if the assembled sequence is identical to the named allele.

This definition favours shorter database entries when two alleles are equally near, which explains
single truth entries such as a null allele whose own sequence is identical to the common allele.

## Reading the four-field table

`eval6.py` prints per gene: haplotypes, exact calls, known alleles and how many of them were called
exactly, new alleles, alleles reported as certain at four fields (`safe4`) and how many of those are
exact, and how many reported new sites are true.
