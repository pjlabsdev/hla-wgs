# Typing long reads

    hla-wgs long DBDIR READS.fastq[.gz] OUTPREFIX [threads]
    hla-wgs long DBDIR READS.bam OUTPREFIX [threads]

For PacBio HiFi reads (experimental since 1.1). A read of 10 to 25 kb usually holds a whole gene, so
the sequence of each haplotype is built from the reads and compared with every full-length allele of
the gene. The allele is named by its exons first, then by the whole gene.

| Argument | Meaning |
|---|---|
| `DBDIR` | database folder from [build-db](build-db.md) |
| `READS.fastq[.gz]` | HiFi reads, ideally only those of the MHC |
| `READS.bam` | indexed BAM: only the MHC region is read |
| `OUTPREFIX` | folder and name of the result files |
| `threads` | number of threads, default 4 |

Output: `PREFIX.long.tsv` (one row per haplotype and gene) and `PREFIX.long.fa` (the consensus
sequences), see [output](../output.md), and the [run log](log.md) with one `long` decision per
haplotype. The long-read switches `HLA_L*` are listed in [switches](switches.md).

The weak point is the exact length of long homopolymers and repeats; such positions are marked as
unsure sites. Validation: 26 HPRC samples, `validation/results/long26_main/`.
