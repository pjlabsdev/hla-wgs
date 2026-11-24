# hla-wgs manual

One page per command, per file format and per procedure. Every page is also readable without the
others. A test (`tests/manual.rs`) checks that every command of the program has its page, that every
environment switch is documented, and that every link in the manual and in the README resolves.

## Commands

| Command | Page |
|---|---|
| `hla-wgs DBDIR R1.fq.gz R2.fq.gz OUTPREFIX [threads]`, `hla-wgs DBDIR INPUT.bam\|cram OUTPREFIX [threads] [REF.fa]` | [Typing short reads](typing.md) |
| `hla-wgs long DBDIR READS OUTPREFIX [threads]` | [Typing long reads](long.md) |
| `hla-wgs build-db hla.dat freq.json DBDIR` | [Building the database](build-db.md) |
| `hla-wgs build-mask DBDIR GENOME.fa [EXCLUDE.bed]`, `--from OLDDBDIR` | [Building the repeat mask](build-mask.md) |
| `hla-wgs html PREFIX.json` | [Result page again](html.md) |
| `hla-wgs diag OUTPREFIX` | [Diagnostic package](diag.md) |
| `hla-wgs mcp` | [MCP server for agents](mcp.md) |
| `hla-wgs unzip ZIP DIR` | [Unpacking a ZIP archive](unzip.md) |
| `hla-wgs --version` (or `-V`), `hla-wgs --db-format` | the version of the program, and the database format it needs (see [database](database.md)) |
| `hla-wgs-validate COMMAND` | [Validation](../../validation/README.md) |

## Files

| File | Page |
|---|---|
| `PREFIX.tsv`, `.conf.tsv`, `.json`, `.novel.fa`, `.novel.tsv`, `.long.tsv`, `.long.fa`, `.html` | [Output](../output.md) |
| `PREFIX.log.jsonl`, `PREFIX.log.txt` | [Run log](log.md) |
| error codes `HW-Ennn` | [Error codes](errors.md) |
| `DBDIR/` (`hladb.json.gz`, `panel.fa`, `panel.fa.mask.bed`, `format`, `release`, WMDA files) | [Database folder](database.md) |
| `HLA_*` environment variables | [Switches](switches.md) |

## Procedures

| Task | Page |
|---|---|
| First run with the start file, update, another IPD-IMGT/HLA release | [Start file](start-file.md) |
| Set up, type, check a result, report a bug, follow a run with an agent | [Procedures](procedures.md) |
| Reproduce the validation | [Validation](../../validation/README.md) |

## How it works

- [Technical picture of every step](../ablauf.html) (German)
- [The aligner](../aligner.md)
- [Software report (PDF)](../paper/hla-wgs-report.pdf)
