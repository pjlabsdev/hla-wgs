# Run log

Every typing run (`hla-wgs DBDIR ...` and `hla-wgs long ...`) writes a log next to its results:

- `PREFIX.log.jsonl`: one JSON object per line, schema `genome.log.v1`, for programs
- `PREFIX.log.txt`: the same events as readable lines, for people

Both are written line by line at once, so the log of an interrupted run is readable up to the
point where it stopped. The log names files without their folders and never holds read sequences,
base qualities or read names.

## Fields of every event

| Field | Meaning |
|---|---|
| `schema` | always `genome.log.v1` |
| `seq` | number of the event in the run, from 1 |
| `t` | seconds since the start of the run |
| `kind` | one of the kinds below |

## Kinds

| Kind | When | Further fields |
|---|---|---|
| `header` | first line | `tool`, `version`, `commit`, `platform` (os, arch, cpus), `mode`, `command`, `parameters` (threads, input type, genes, traced genes, thresholds), `environment_set` (every `HLA_*` and `GENES` variable that is set), `defaults` (every switch with its default), `inputs` and `reference` (name, `bytes`, `crc32`, `crc32_scope`), database `release` and `format` |
| `begin` | a step starts | `step` |
| `end` | a step ends | `step`, `seconds`, `peak_mb` (peak memory of the process so far; Linux only, otherwise null), `counts` |
| `progress` | after every gene | `step`, `done`, `total`, `eta_s` (estimated seconds left) |
| `decision` | the program decided something on its own | `step`, `subject`, `decision`, `reason`, `values` |
| `trace` | detail for genes named with `--trace` | `gene`, `what`, `values` |
| `note` | a message that also goes to stderr | `step`, `text` |
| `error` | the run ends with an error | `code`, `message`, `what`, `cause`, `fix` ([error codes](errors.md)) |
| `end_run` | last line of a finished run | `ok`, `seconds`, `peak_mb`, `outputs` (files written) |

A run that ends without `end_run` was interrupted (killed, power loss). A run that fails writes an
`error` event as its last line.

## Steps and their counts

| Step | Counts |
|---|---|
| `read_and_map` | `pairs_in` = `dropped_no_shared_21mer` + `dropped_no_alignment` + `dropped_repeat_mask` + `dropped_mate_rule` + `mapped`; `mapped` = `assigned_pairs` + `dropped_below_score`; `assigned_per_gene` (a pair can count for several related genes) |
| `gene X` | `pairs` (with the reads of related genes), `candidates_stage_a`, `candidates_pass2`, `reads_scored`, `typed`, `new_sites` per slot |
| `typing` | `genes`, `typed` |
| `drb_copies` | ratio of uniquely explained DRB3/4/5 reads per DRB1 copy |
| `write_results` | `files` |
| `read` (long reads) | `reads`, `bases` |

## Decisions

- `typing`: a gene is not typed (fewer than 20 read pairs, no candidate, no allele pair), with the
  counts that decided it.
- `certainty`: for every reported allele, why its certain call stops where it does: the leads at
  fields 2, 3 and 4, the thresholds that applied, the best competing allele at each field, a gene
  limit (DMB and TAP2 to three fields), a new variant (inside an exon: allele group only, outside:
  three fields) or a withheld allele group. Example:

      decision certainty DRB1 slot 1: DRB1*04:05:01 (3 fields) (new variant outside the exons: three fields hold)

- `long`: per long-read haplotype the named allele, its differences and whether it is a minor read group.

## Following one gene

`--trace A` (or `HLA_TRACE=A,DRB1`) adds `trace` events for the named genes: the stage A candidates,
the best allele pair of stage B with its log-likelihood, the lead over the next pair and the tied
pairs, and the final names after the exon refinement.

## Diagnostic package

`hla-wgs diag PREFIX` writes `PREFIX.diag.tar` with the run log (`diag.log.jsonl`, `diag.log.txt`)
and the program version. It holds no results and no reads. Attach it to a bug report.
