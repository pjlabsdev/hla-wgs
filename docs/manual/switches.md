# Switches

Environment variables of `hla-wgs`. The defaults are the validated configuration; the switches are
for experiments, debugging and tests. Every switch that is set is written into the header of the
[run log](log.md) (`environment_set`), and every switch that can change a result is listed there with
its default (`defaults`). A test checks that every variable the program reads is on this page or on
the [start file](start-file.md) page, and that every switch of the first table is in `defaults`.

## Switches that can change the result

| Variable | Effect |
|---|---|
| `GENES` | comma-separated genes to type (default: all 26) |
| `HLA_TGRP` | lead a certain two- or three-field call needs (default 4; `6` as before 1.2) |
| `HLA_F401=0` | fourth field 01 needs the full lead of 4 like any other allele |
| `HLA_FIELD1=0` | the allele group is not withheld when a competitor of another group is nearly as good |
| `HLA_DRBREL=0` | the DRB1 exon refinement does not see the reads of DRB3/4/5 |
| `HLA_REP4=0` | a fourth field that differs from its nearest four-field rival only in the length of a repeat is released even without reads that span the repeat (the behaviour of 1.2.0); with the rule on, it needs at least 3 spanning reads with its length and three times as many as with the rival's, except for a fourth field 01 while `HLA_F401` is on |
| `HLA_NOFREQ=1` | no population frequency prior: every two-field group starts equal, and the common groups of the population data do not join the exon refinement |
| `HLA_PADF1=0` | an allele stored without its ends gets no flank from its first-field group |
| `HLA_PAD_MIN` | an allele more than this many bases shorter than another allele of its gene gets that allele's flank (default 8; 1.0 used 20) |
| `HLA_PAD1` | flanks are borrowed in one step only, not from an allele that was padded itself |
| `HLA_NONOVEL` | skip the new-allele examination |
| `HLA_NOKNOWN` | do not rename a call to the database allele that its new sites describe |
| `HLA_NOREP` | skip the repeat-length examination |
| `HLA_NOBIG` | count long insertions and deletions per base instead of as one event |
| `HLA_NOEXT` | do not extend clipped read ends when looking for new sites |
| `HLA_NOBLIND` | do not report stretches without reads (`blind`) |
| `HLA_NOFIRM` | a site carried by the pairs of one allele is not moved from `either` to that allele |
| `HLA_NOPARA` | do not discount reads that fit a paralogous gene better |
| `HLA_NOPHASE` | do not phase sites shared by both alleles |
| `HLA_NOV20` | skip the phase-conflict check of new sites |
| `HLA_NMIN` | reads a new site needs (default 5, 4 below 18x, 3 below 12x) |
| `HLA_NOV_EDGE` | distance from a gene end within which a new site needs 90 % of the reads (default 15) |
| `HLA_UNPH` | also cut the fourth field when a sibling allele differs only at positions without phase information (fewer wrong, clearly fewer certain calls) |
| `HLA_UNSTHIN=0` | old rule for unsupported database bases: also in stretches with fewer than half the usual reads |
| `HLA_PHASE`, `HLA_BREAK` | experimental phase and break-point rules, off by default |
| `HLA_REPV` | experimental: `MINLEN:FRAC` marks long repeats whose length too few reads confirm as unverified |
| `HLA_FULLKMERS` | use all panel 21-mers in the FASTQ prefilter, also those inside the repeat mask (slower, the behaviour before 1.1) |
| `HLA_MASK` | repeat mask file instead of `panel.fa.mask.bed` |
| `HLA_DECOYS` | repeat element file instead of `repeat_decoys.fa` |
| `HLA_PANEL_N` | alignments per read against the panel (default 100) |
| `HLA_PANEL_G` | chained sequences per gene (default 25) |
| `HLA_PANEL_C` | smallest chain score (default 25) |
| `HLA_PANEL_S` | score a panel hit needs (default 70) |
| `HLA_PANEL_SE` | score of a hit that runs over the end of the panel sequence (default 60) |
| `HLA_PANEL_MW` | minimizer window for read seeds (default 11; 1 looks up every k-mer) |
| `HLA_PANEL_FULL` | reseed with every k-mer when the best chain covers less than this percentage of the read (default 80) |
| `HLA_PANEL_W` | also count a mate for genes within this many points of its best alignment (default 0) |
| `HLA_PANEL_CLIP` | bases a mate may be clipped where the gene sequence goes on (default 0 = no rule) |
| `HLA_PANEL_B` | pairing bonus (default 33) |
| `HLA_PANEL_RESCUE=0` | no mate rescue |
| `HLA_PANEL_RS` | score a rescued mate needs (default as `HLA_PANEL_S`) |
| `HLA_PANEL_RW` | mate rescue for genes within this many points of the best hit of the other mate (default 40) |
| `HLA_PAIR` | mate rule: a gene keeps a pair only if both mates hit it or the other mate may lie beyond the end of the sequence (`dropped_mate_rule` in the log) |
| `HLA_PANEL_Q` | mismatches at bases below this quality cost less (default 0 = off) |
| `HLA_RESCUE_V` | shared 11-mers a rescue stretch needs (default 3) |
| `HLA_RESCUE_BAND` | indel margin of the banded rescue alignment (default 24; 0 = whole window) |
| `HLA_LPLUR=0` | long reads: majority instead of plurality consensus |
| `HLA_LSPAN=0` | long reads: rank haplotypes by all reads, not by spanning reads |
| `HLA_LEXON=0` | long reads: name the nearest allele by the whole gene, not by its exons first |
| `HLA_LMERGE=0` | long reads: no joint consensus of groups that name the same allele |
| `HLA_LMU` | long reads: new unsure sites that keep two such groups apart (default 2) |

## Switches with the same result

| Variable | Effect |
|---|---|
| `HLA_PAD_CACHE` | file for the padding cache (default `pad_cache.tsv.gz` next to `hladb.json.gz`; `0` switches it off), see [database folder](database.md) |
| `HLA_IO_RETRIES` | retries after an input I/O error (default 6); damaged content is not retried but reported at once ([HW-E015](errors.md#hw-e015)) |
| `HLA_EXT_AD=0`, `HLA_RESCUE_AD=0` | row-wise instead of anti-diagonal alignment kernels (slower) |
| `HLA_WGS_LANG` | `de` or `en`: language of the result page ([html](html.md)) |

## Debugging output

| Variable | Effect |
|---|---|
| `HLA_TRACE` | genes to follow in the run log, as `--trace` ([run log](log.md#following-one-gene)) |
| `HLA_SAVE_READS` | prefix for a FASTQ copy of the read pairs that pass the panel prefilter |
| `HLA_ALNSTAT` | alignment statistics on stderr ([aligner](../aligner.md#measuring)) |
| `HLA_ALNBIG` | every large alignment matrix on stderr |
| `HLA_TDBG` | time of each stage per gene on stderr |
| `HLA_CMP`, `HLA_CMPZ` | `GENE\|alleleA\|alleleB`: the reads that tell two candidates apart; `HLA_CMPZ` adds a third allele's score |
| `HLA_DRBDUMP` | per read the DRB allele that explains it best |
| `HLA_NOVDBG` | `GENE:POS`: the evidence for one position of the new-site examination |
| `HLA_REPDUMP` | every repeat with the length changes seen in reads |
| `HLA_REP4DBG` | per called allele the fourth-field rival and the repeats in which the two differ only in length |
| `HLA_THINDBG` | coverage classes of the thin-stretch rule |
| `HLA_LONG_TIME` | long reads: time per gene family |

## Building the repeat mask

| Variable | Effect |
|---|---|
| `HLA_MASKFRAC` | with `--from`: fraction of a k-mer's occurrences that must lie in masked intervals (default 0.8) |
| `HLA_MASKNOCOPY` | with `--from`: recompute unchanged alleles too instead of copying their intervals |

## Tests and build

| Variable | Effect |
|---|---|
| `HLA_TEST_IO_FAULT` | tests only: simulate I/O errors on the input |
| `HLA_PROPS` | tests only: run the random property tests of the aligner this many times longer |
| `HLA_WGS_COMMIT` | set by `build.rs` at build time: the commit shown in the log and by the MCP `version` tool |

The switches of the start files (`HLA_WGS_DB`, `HLA_WGS_RELEASE`, `HLA_WGS_LONG`,
`HLA_WGS_NO_UPDATE`, `HLA_WGS_HOME`) are on the [start file](start-file.md) page; those of
`hla-wgs-validate` in [validation](../../validation/README.md).
