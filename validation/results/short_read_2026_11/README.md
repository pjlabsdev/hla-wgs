# Short-read WGS: call rate, precision and run time (November 2026)

Starting point is release 1.2.0 (main 62b6a4c). "new" is this branch: the evaluation `eval23` with fixed
denominators, the repeat rule `HLA_REP4` and the switch `HLA_NOFREQ`.

Every number below carries its origin:

- **[measured]**: newly run for this folder (25 Nov 2026).
- **[computed]**: computed here from result files that already existed.
- **[documented]**: taken from an earlier document without rerunning it.

## Data

| Set | Samples | Truth | Role |
|---|---|---|---|
| dev80 | 80 HPRC (`newg/dev80.txt`) | `truth4.tsv`, HPRC release 2 assemblies | development; used to choose every threshold |
| ho30 | 30 HPRC held out (`newg/ho4.txt`, part of `heldout.txt`) | `truth4.tsv` | not used to design the new rule; excluded from the frequency prior |
| hgsvc59 | 59 HGSVC3 (`lr/hgsvc_all59.txt`) | `truth4_hgsvc.tsv`, HGSVC3 assemblies | unseen until 1.2.0, **but used here to find the DRB1 errors that led to the rule**; 22 of the 59 are in the 1000 Genomes laboratory typing that the prior is counted from |

- Reads: CRAM of the 1000 Genomes 30x data (S3), MHC region plus unmapped reads, extracted with
  `scripts/extract_sr.sh`.
- Typing for the quality tables ran on the prefilter cache of these reads (`HLA_SAVE_READS`). The
  results are byte-identical to typing from the full extracted reads (checked on HG00438;
  `PREFIX.json` differs only in `analysis_time`).
- Database: IPD-IMGT/HLA 3.66, the same build as all published validation (format 1).
- Threads: 2.

The design of the rule used dev80 and hgsvc59. The exemption for fourth field 01 was added after
dev80 lost right DQB1 calls. ho30 is the only set on which the final rule was judged without any
choice depending on it.

## Evaluation (`hla-wgs-validate eval23`)

```
SAMPLES=lr/hgsvc_all59.txt T4OUT=truth/truth4_hgsvc.tsv GENES=26 hla-wgs-validate eval23 OUTDIR
SAMPLES=newg/ho4.txt       T4OUT=truth/truth4.tsv       GENES=26 hla-wgs-validate eval23 OUTDIR
SAMPLES=newg/dev80.txt     T4OUT=truth/truth4.tsv       GENES=26 hla-wgs-validate eval23 OUTDIR
```

- **Denominator** (`expected`): fixed before any call is read. It counts one allele copy per
  haplotype of a sample that carries the gene, for samples with two finished haplotypes, at fields
  1 to 4.
  - A copy whose reference is ambiguous at that field (`ref_ambiguous`), shorter than the field
    (`ref_short`) or new at four fields (`ref_new`) is not evaluable there and is left out.
  - Genes absent from the truth table are not evaluated (listed under issues).
- **Explicit counts:** missing result files, genes without a call, wrong copy numbers
  (`copy_number_wrong`) and unresolved references.
- **`candidate_ok`:** the allele named in `allele` is right at that field. This is the hit rate
  of the candidate.
- **`released`, `released_ok`:** `safe_call` reaches that field, and is right.
  - Precision = `released_ok` / (`released` + `released_extra` + `released_on_new`).
  - A four-field call on a haplotype that differs from every database allele is
    `released_on_new` and counts as wrong.
  - With nothing released, precision is `n/a`.
- **Call rate:** `released` / `expected`, over all expected copies.
- **Regression test** (`released_counts_against_all_expected_copies`):
  - Setup: 6 reference copies, 2 right candidates released at one field, 4 without a call.
  - At 2 and 3 fields: 0 / 6 released, precision `n/a`, call rate 0.0 %.
  - At 1 field: 2 / 2 right, call rate 33.3 %.
- **New sites** (`eval6`, columns `site_x_*`):
  - SNVs match by position and base change.
  - Indels match by the resulting sequence (a window of ±40 bases on the allele, changed on both sides).
  - A site seen only on the other haplotype and marked `:either` is counted separately as phase
    uncertain.

## Before and after, all genes [measured]

| Set | Fields | Expected | 1.2.0 released (call rate) | 1.2.0 precision | new released (call rate) | new precision |
|---|---|---|---|---|---|---|
| ho30 | 2 | 932 | 889 (95.4 %) | 99.9 % | 889 (95.4 %) | 99.9 % |
| ho30 | 3 | 880 | 843 (95.8 %) | 99.8 % | 843 (95.8 %) | 99.8 % |
| ho30 | 4 | 759 | 682 (89.9 %) | 98.7 % | 682 (89.9 %) | **99.0 %** |
| hgsvc59 | 2 | 1824 | 1732 (95.0 %) | 99.9 % | 1732 (95.0 %) | 99.9 % |
| hgsvc59 | 3 | 1713 | 1650 (96.3 %) | 99.8 % | 1650 (96.3 %) | 99.8 % |
| hgsvc59 | 4 | 1498 | 1332 (88.9 %) | 98.4 % | 1329 (88.7 %) | 98.5 % |
| dev80 | 2 | 2479 | 2344 (94.6 %) | 99.8 % | 2344 (94.6 %) | 99.8 % |
| dev80 | 3 | 2306 | 2211 (95.9 %) | 99.6 % | 2211 (95.9 %) | 99.6 % |
| dev80 | 4 | 1973 | 1768 (89.6 %) | 98.3 % | 1765 (89.5 %) | 98.3 % |

- The 1.2.0 numbers reproduce the published `hgsvc59_main` and `dev80_main` results exactly.
  `eval6`: safe four-field calls exact 1334 / 1350 and 1780 / 1799.
- Per gene and field: `eval23_<set>_<version>.tsv`.

**What changed at four fields:**

| Set | Wrong calls removed | Right calls lost |
|---|---|---|
| ho30 | 2 (DRB1) | 0 |
| hgsvc59 | 2 (DRB1) | 2 (HLA-F) |
| dev80 | 0 | 3 (DQA1) |

- In every lost case the deciding repeat (51 to 75 bp) had at most 1 spanning read.
- HGSVC3 `eval6` with the new rule: safe four-field exact 1332 / 1346 (was 1334 / 1350); new sites unchanged.
- Two and three fields are unchanged everywhere.

### DRB1 at four fields [measured]

| Set | 1.2.0 right / judged | Precision | new right / judged | Precision |
|---|---|---|---|---|
| ho30 | 13 / 16 | 81.2 % | 13 / 14 | 92.9 % |
| hgsvc59 | 16 / 21 | 76.2 % | 16 / 19 | 84.2 % |
| dev80 | 15 / 19 | 78.9 % | 15 / 19 | 78.9 % |

The five DRB1 four-field calls in HGSVC3 that are not exact, examined with `--trace DRB1` and the
reads over the assembly:

- **HG00268 (both copies, 15:01:01:03), HG02492 (15:02:01:02) and HG02554 (15:03:01:03):**
  - The assembly differs from the database allele only by one base in a T run of 23.
  - Spanning short reads support the database length; at HG00268, 3 to 5 reads show T23 and the
    assembly has T24.
  - These are disagreements between reference and reads, not clear typing errors.
  - The truth counts them as new haplotypes, so the four-field call counts as wrong.
- **HG03807 (called 15:02:01:02, truth 15:02:01:03):** a real error.
  - The two alleles differ only in a (TG)n repeat (5 spanning reads, all for the call) and in a
    T26 run that no read spans.
  - The fourth field rested on a repeat length that no read had seen.

The rule (`HLA_REP4`, on by default) targets the last case:

- **Trigger:** the called allele differs from its nearest four-field rival of Stage B only in repeat lengths.
- **Requirement:** each such repeat needs at least 3 reads that span it with the call's length,
  and three times as many as with the rival's length.
- **Otherwise:** a site `repunverified<len>bp` is reported and the call stops at three fields.
- **Exemption:** a fourth field 01 keeps the older rule `HLA_F401`. Without this exemption,
  dev80 lost 14 right calls, mostly DQB1, F and DQA1, and no wrong one.
- `HLA_REP4=0` reproduces 1.2.0 byte for byte (6 HGSVC3 samples, all result files).

### Equal call rate: a plain margin threshold [computed]

`threshold_sweep_v1.2.0.tsv` lowers 1.2.0 four-field calls to three fields when `margin4` is below
a threshold. It uses the 1.2.0 result files and the same `eval23`.

- Removing as many four-field calls as the new rule does (3 to 5 per set) needs a threshold of
  about 1.15 to 1.3. That removes **no** wrong call on ho30 and hgsvc59.
- Even at threshold 15 (call rate 69 to 72 %), the DRB1 errors of ho30 and hgsvc59 mostly stay.
- The wrong DRB1 calls have large margins, because the reads cannot tell the two repeat lengths
  apart, so a stricter threshold does not reach them.
- The new rule removes 4 wrong calls at a cost of 5 right ones over all 169 samples. On the set
  not used to design it (ho30), it removes 2 wrong calls at no cost.

## Population prior [measured]

- **Shipped prior:** `data/freq_1000g_2field.json` is built from the 1000 Genomes laboratory
  typing (`20140702_hla_diversity.txt`, 1278 samples) by
  `hla-wgs-validate build-freq OUT newg/dev80.txt heldout.txt`.
  - Rebuilding it today gives an identical file.
  - It excludes dev80 and the held-out HPRC samples, so ho30 has no overlap.
  - It does **not** exclude HGSVC3: 22 of the 59 HGSVC3 samples are in the laboratory typing
    (13 of dev80 and 8 of ho30 are too, but they are excluded).
- **Leak check:** a database whose prior also excludes the 59 HGSVC3 samples
  (`build-freq OUT dev80.txt heldout.txt hgsvc_all59.txt`) gives the same counts on hgsvc59 in
  every gene and field (`eval23_hgsvc59_prior_without_hgsvc.tsv`). The overlap does not change
  the HGSVC3 result.
- **Without any prior** (`HLA_NOFREQ=1`). These runs used an intermediate version of the rule
  (without the 01 exemption), so they are compared with that version (`eval23_*_c1.tsv`).
  - hgsvc59: two fields 1725 instead of 1732 released, three fields 1644 instead of 1650, four
    fields 1319 instead of 1326. Precision is unchanged.
  - ho30: two fields 885 instead of 889, three fields 839 instead of 843, four fields 681 both.
    Precision is unchanged.
  - The prior adds calls and costs no precision. It stays on.

## Comparison with other tools

Same samples, same truth, same `eval23` code. Each tool's calls were converted to `conf.tsv`, with
`allele` and `safe_call` both set to the tool's call.

The conversion scripts are `t1k2conf.py` and `spec2conf.py` in the work folder:
- T1K: `HLA-A*03:01:01` → `A*03:01:01`; a gene reported once is counted twice (homozygous).
- SpecHLA: `hla.result.txt` columns.

Genes:
- 15 genes with a fixed copy number: A, B, C, DRB1, DQA1, DQB1, DPA1, DPB1, DRA, E, F, G, H, K, V.
- For SpecHLA, its 8 genes.

DRB3/4/5 are left out, because T1K reports no copy number.

**T1K 1.0.10** [computed]: existing T1K runs (`--preset hla-wgs`, 4 threads, run 16 Nov 2026 on the
same extracted reads, T1K's own index from IPD-IMGT/HLA), evaluated with `eval23`.

| Set | Fields | Expected | T1K released (call rate) | T1K precision | hla-wgs 1.2.0 released (call rate) | hla-wgs precision |
|---|---|---|---|---|---|---|
| ho30 | 2 | 882 | 882 (100.0 %) | 98.4 % | 840 (95.2 %) | 100.0 % |
| ho30 | 3 | 831 | 828 (99.6 %) | 98.7 % | 795 (95.7 %) | 99.9 % |
| ho30 | 4 | 737 | 3 (0.4 %) | 0.0 % | 678 (92.0 %) | 98.8 % |
| dev80 | 2 | 2345 | 2345 (100.0 %) | 98.1 % | 2217 (94.5 %) | 99.8 % |
| dev80 | 3 | 2176 | 2174 (99.9 %) | 97.8 % | 2087 (95.9 %) | 99.6 % |
| dev80 | 4 | 1900 | 10 (0.5 %) | 0.0 % | 1762 (92.7 %) | 98.3 % |

- T1K always reports an answer. In absolute right two-field calls it is ahead on these 15 genes:
  ho30 878 against 840, dev80 2338 against 2216.
- hla-wgs releases fewer copies but makes fewer errors: 4 wrong two-field calls (including extra copies) out of 3060 released, against 60 out of 3276 for T1K.
- T1K stops at three fields.
- Tables: `tools15_<set>_<tool>.tsv`.

**SpecHLA 1.0.12** [measured]:

Setup:
- Samples: all 30 held-out samples, its 8 genes.
- Install: Bioconda, with freebayes raised to 1.3.10 (the resolved 1.0.2 fails with "invalid FORMAT: AD").
- Input: the MHC-plus-unmapped BAM of each sample, read by its own `ExtractHLAread.sh -r hg38`, then
  `SpecHLA.sh -j 4`.
- Database: its own bundled one, IPD-IMGT/HLA 3.38.0, so alleles named after 3.38 cannot be called by it.

Same 30 samples and 8 genes for every tool (`tools8_ho30_<tool>.tsv`):

| Tool | Fields | Released (call rate) | Precision |
|---|---|---|---|
| SpecHLA 1.0.12 | 2 | 476 / 476 (100.0 %) | 94.7 % |
| SpecHLA 1.0.12 | 3 | 457 / 460 (99.3 %) | 94.5 % |
| SpecHLA 1.0.12 | 4 | 397 / 414 (95.9 %) | 53.0 % |
| T1K 1.0.10 | 2 | 476 / 476 (100.0 %) | 99.4 % |
| T1K 1.0.10 | 3 | 458 / 460 (99.6 %) | 98.9 % |
| T1K 1.0.10 | 4 | 3 / 414 (0.7 %) | 0.0 % |
| hla-wgs 1.2.0 | 2 | 474 / 476 (99.6 %) | 100.0 % |
| hla-wgs 1.2.0 | 3 | 454 / 460 (98.7 %) | 99.8 % |
| hla-wgs 1.2.0 | 4 | 368 / 414 (88.9 %) | 98.4 % |
| hla-wgs new | 2 | 474 / 476 (99.6 %) | 100.0 % |
| hla-wgs new | 3 | 454 / 460 (98.7 %) | 99.8 % |
| hla-wgs new | 4 | 368 / 414 (88.9 %) | 98.9 % |

- SpecHLA's two-field errors are mostly in DPB1 (48 / 60 right), DQA1 (52 / 57) and DPA1 (57 / 60).
  A, B, C, DRB1 and DQB1 are at 96.6 to 100 %.
- At four fields, 48 of SpecHLA's released calls fall on haplotypes that are new against IPD-IMGT/HLA
  3.66, and only 236 of 397 are right.

**HLA\*LA** [not run]: its graph (`PRG_MHC_GRCh38_withIMGT`) is downloaded from
`www.well.ox.ac.uk`, which the network of the cloud machine refuses (HTTP 403). The missing
dataset is that graph.

To reproduce on a machine with access (HLA\*LA from Bioconda, 4 threads):

```
wget http://www.well.ox.ac.uk/downloads/PRG_MHC_GRCh38_withIMGT.tar.gz && tar xzf PRG_MHC_GRCh38_withIMGT.tar.gz -C graphs
HLA-LA.pl --BAM SAMPLE.mhc_un.bam --graph PRG_MHC_GRCh38_withIMGT --sampleID SAMPLE --maxThreads 4 --workingDir out
```

The BAM is the MHC-plus-unmapped extract (`scripts/extract_sr.sh`). Then convert
`out/SAMPLE/hla/R1_bestguess_G.txt` to `conf.tsv` and run `eval23` as above.

## Run time

### From extracted MHC reads, cloud machine [measured]

Setup:
- Machine: 4 vCPU Intel Xeon 2.1 GHz, 15 GB RAM, Linux.
- Database: bdx (IPD-IMGT/HLA 3.66).
- Cache: pad cache and page cache warm (one warm-up run first).
- Threads: 4. Nothing else ran (the SpecHLA loop was stopped with SIGSTOP during the series).
- Samples: the 8 of `timing_main.tsv` (HG00126 HG00146 HG00438 HG00621 HG00673 HG00733 HG00735
  HG00738), the full extracted reads (`keep/`), two rounds per version.
- Commits: 1.2.0 = 62b6a4c; new = this branch.
- Command: `hla-wgs DB S.R1.fq.gz S.R2.fq.gz OUT 4`, timed with `tm.py` (wall, user, sys, peak RSS).
- Table: `timing_cloud.tsv`.

| Series | Median wall | p95 wall | Mean CPU | Peak RSS | Samples per hour (1 at a time) |
|---|---|---|---|---|---|
| 1.2.0, round 1 | 17.3 s | 20.2 s | 61.0 s | 1244 MB | 207 |
| new, round 1 | 16.3 s | 20.9 s | 59.4 s | 1222 MB | 213 |
| 1.2.0, round 2 | 16.3 s | 19.9 s | 59.0 s | 1235 MB | 214 |
| new, round 2 | 16.5 s | 19.9 s | 58.7 s | 1266 MB | 215 |

The two versions take the same time, within run-to-run noise.

**From a BAM** (HG00438, MHC plus unmapped, 147 MB), three runs per version:

| Version | Wall times |
|---|---|
| 1.2.0 | 17.6 s, 17.7 s, 16.9 s |
| new | 17.8 s, 17.8 s, 18.8 s |

- The result is byte-identical to typing from FASTQ.
- Open mate map (new log line `[input] mates waiting for their partner`): at most 27,630 reads and
  9.4 MB of names, bases and qualities. 17,054 reads without a partner are dropped.

**Where the time goes** (HG00438, 1 thread, `HLA_TDBG=1`, 56.2 s wall):

| Stage | Time | Share |
|---|---|---|
| Panel index, prefilter and panel mapping | 21.4 s | 38 % |
| Stage A/B edit distances | 10.8 s | 19 % |
| Keeping reads | 3.0 s | 5 % |
| Pair scoring of Stage B | 1.9 s | 3 % |
| Rest of Stage B | 1.9 s | 3 % |
| Everything else (exon refinement, new-site examination, output; not broken down) | about 17 s | 30 % |

Stage B pair scoring is about 3 % of the run, so sharing identical likelihood columns between
pairs would save at most about 2 s per sample on one core. It was not built. No performance change
is part of this branch, so calls, `safe_call`, alternatives and copy numbers are unchanged by
construction outside the rule above.

### Comparison tools on the same 8 samples

| Tool | Median wall | Mean wall | Max wall | Peak RSS | Source |
|---|---|---|---|---|---|
| hla-wgs (then main) | 17.1 s | 17.0 s | 20.1 s | 1241 MB | [documented] `timing_main.tsv` |
| T1K 1.0.10 | 193.9 s | 197.1 s | 236.8 s | 3525 MB | [documented] `timing_main.tsv` |
| SpecHLA 1.0.12 | 666.5 s | 678.2 s | 802.7 s | 867 MB (largest single process) | [measured] `timing_spechla.tsv` |

All three used the same 4-vCPU machine type, 4 threads and the same extracted reads (MHC plus unmapped).

SpecHLA was run alone on the idle machine:
- Command: `SpecHLA.sh -n S -1 S.R1.fq.gz -2 S.R2.fq.gz -j 4`.
- Mean CPU time: 1728 s.
- Its own `ExtractHLAread.sh` would first shrink a BAM to HLA reads (about 14 s per sample in the
  quality runs) and so shorten the typing step.

### From complete inputs, pj's Mac [measured]

Setup:
- Program: release 1.2.0 (commit e6abde400bf6).
- Database: IPD-IMGT/HLA 3.66.0, format 2.
- Machine: Apple M5 MacBook Air, passively cooled (4 performance and 6 efficiency cores), 16 GB,
  macOS 27.2.1.
- Inputs: all on an external USB SSD (about 380 MB/s, measured with `dd`).
- Runs: three runs with the default 4 threads, then one with 10.
- Cache: "first access" means the first read in this series (`purge` needs root, so no true cold
  start). The FASTQ (53 GB) is larger than RAM, so it is never cached.
- Table: `timing_full_mac.tsv`.

| Input | Read pairs typed | Wall, 4 threads | Wall, 10 threads | Peak RSS |
|---|---|---|---|---|
| PJ.bam, 70.9 GB, whole genome | 15.6 M (MHC, unmapped, ALT/HLA contigs, via index) | 26.2 s first, 20.2 s and 20.4 s warm | 20.9 s | 1.5–2.0 GB |
| NA18949.final.cram, 16.0 GB | 0.68 M (via index) | 17.7 s first, 15.7 s and 15.7 s warm | 14.3 s | 1.8 GB |
| HG001 30x FASTQ, 53.1 GB, 361 M pairs | all read | 599 s, 732 s, 776 s | 632 s | 1.5–1.8 GB |

- Results were identical over the four runs of each input.
- For FASTQ, more than 98 % of the time is reading and mapping, at 0.60 M pairs per second in
  the first run.
- The later FASTQ runs got slower while their CPU time grew (2618 to 3415 s). This is probably
  the fanless machine throttling under a 10-minute load; it is not proven.
- A FASTQ figure without that effect needs cooling pauses or an actively cooled machine.

### The run-time figures in the README until now

- **"about 40 s" against "about 150 s"** (README since 1.0.0, 19 Nov 2026) [documented]: these
  came from the prototype before 1.0.0 (Python driver with the Rust kernel), timed on extracted
  reads with 4 cores during the held-out rounds. No result file in the repository holds them,
  and they cannot be reproduced from stored data.
- **"17.0 s against 197.1 s"** (`timing_main.tsv`, 24 Nov 2026) [documented]: means of 8 samples
  on the same extracted reads with 4 threads. The program had become faster since 1.0.0 (pad
  cache, kernel index, own aligner, see CHANGELOG 1.1.3/1.2.0).
- **Today** [measured]: 16.3–17.3 s median for both 1.2.0 and this branch, which confirms the 17.0 s.
- The README now quotes the measured values and links this folder.
