# Typing short reads

    hla-wgs DBDIR R1.fq.gz R2.fq.gz OUTPREFIX [threads]
    hla-wgs DBDIR INPUT.bam OUTPREFIX [threads]
    hla-wgs DBDIR INPUT.cram OUTPREFIX [threads] REF.fa
    hla-wgs ... --trace A,DRB1

Types the 18 HLA genes (A, B, C, DRB1, DRB3, DRB4, DRB5, DQA1, DQB1, DPA1, DPB1, DRA, E, F, G, H,
K, V) and the eight further genes DMA, DMB, DOA, DOB, MICA, MICB, TAP1 and TAP2 from paired short
reads (2x100 to 2x150, about 15x or more).

## Arguments

| Argument | Meaning |
|---|---|
| `DBDIR` | database folder from [build-db](build-db.md); its two files `hladb.json.gz panel.fa` may be given instead |
| `R1.fq.gz R2.fq.gz` | paired FASTQ, plain or gzip; the whole genome or reads extracted from the MHC plus unmapped reads |
| `INPUT.bam`, `INPUT.cram` | coordinate-sorted and indexed (`.bai`, `.csi` or `.crai` next to it), GRCh38 or GRCh37 |
| `REF.fa` | for CRAM: the reference it was compressed against, with its `.fai` index |
| `OUTPREFIX` | folder and name of the result files, e.g. `out/sample`; the folder must exist |
| `threads` | number of threads, default 4 |
| `--trace GENE[,GENE]` | detailed `trace` events in the [run log](log.md) for these genes (also `HLA_TRACE`) |

From BAM or CRAM the program reads the MHC region (GRCh38 chr6:28,510,120-33,480,577 or GRCh37
6:28,477,797-33,448,354), all ALT and HLA contigs, and the unmapped reads.

## What happens

1. Checks before the work: database, inputs, index, reference and output folder. A problem ends the
   run at once with an [error code](errors.md).
2. `read_and_map`: the read pairs are mapped to the allele panel with the program's own
   [aligner](../aligner.md); pairs anchored only in masked repeats are dropped.
3. One step per gene: candidates, best allele pair, exon refinement, new sites, certainty.
4. `drb_copies`: copy number of DRB3/4/5 against DRB1.
5. `write_results`: the files below.

Each step, its counts and every automatic decision are in the [run log](log.md).

## Output

`PREFIX.tsv`, `PREFIX.conf.tsv`, `PREFIX.json`, `PREFIX.html`, the report part `PREFIX.report.json`
([report](report.md)), and when new alleles were found
`PREFIX.novel.fa` and `PREFIX.novel.tsv`; formats in [output](../output.md). The run log
`PREFIX.log.jsonl` and `PREFIX.log.txt` is written next to them.

## Exit code

0 when the results are written, 1 on an error (the code is on stderr and in the log).

## Time and memory

On average 17 s per sample for the extracted MHC reads of a 30x genome on 4 cores, 1.2 GB peak memory
(8 samples, `validation/results/timing_main.tsv`). A whole-genome FASTQ takes longer because every pair is
read once.
