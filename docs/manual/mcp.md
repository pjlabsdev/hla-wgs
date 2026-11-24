# MCP server

`hla-wgs mcp` is a Model Context Protocol server over stdio (JSON-RPC 2.0, one message per line).
With it Claude and other agents can start hla-wgs, follow a run and read what it did. It shows the
same data as the result files and the [run log](log.md), never other data. It works only on this
computer: it starts hla-wgs itself as a child process and reads files, nothing leaves the machine.
Runs it started are stopped when the server ends.

Claude Code: `claude mcp add hla-wgs -- /path/to/hla-wgs mcp`. Claude Desktop, in
`claude_desktop_config.json`:

    {"mcpServers": {"hla-wgs": {"command": "/path/to/hla-wgs", "args": ["mcp"]}}}

The only action that changes something is `run_cancel` (marked `destructiveHint`). `run_start`
writes the result files of a new run.

## run_start

Start a typing run. Arguments: `db` (database folder), `inputs` (two FASTQ files, or one BAM/CRAM,
or for long reads one FASTQ or BAM), `prefix` (output prefix), optional `threads` (default 4),
`mode` (`short` or `long`), `reference` (FASTA for CRAM), `trace` (genes to follow, e.g. `A,DRB1`).
Returns `run_id`, `prefix` and the path of the log.

## run_status

State of a run: `running`, `finished`, `failed`, `cancelled`, or for a run not started by this
server (given by `prefix`) `running or interrupted`. With the last step, the last `progress` event
(genes done, estimated seconds left), the `error` event with its [code](errors.md) when the run
failed, and the `end_run` event.

## run_cancel

Stops a run this server started. Its partial files stay; its log ends without `end_run`.

## result_read

The result of a finished run: `PREFIX.json` (schema `genome.hla.v1`, see [output](../output.md))
for short reads, the rows of `PREFIX.long.tsv` for long reads. `gene` limits it to one gene.

## log_read

Events of `PREFIX.log.jsonl`, filtered by `kind`, `step` (`gene A` or `gene` for all genes),
`contains` (text anywhere in the event) and `after_seq` (only newer events, for following a run).
At most `limit` events (default 200), the last ones when more match; `matched` gives the number.

## explain

Everything the run recorded about one gene: a summary line per allele (why it is certain to the
fields it has, or why the gene was not typed), the `decision` events with leads, thresholds and best
competitors, the counts of the gene step, `trace` events (with `--trace`), errors, and the gene's
entry from the result.

## version

Version and commit of hla-wgs, the database format it needs, the schemas of log and result, and for
`db` the IPD-IMGT/HLA release and format of that database folder.

## Resources

- `hla-wgs://manual/index` and one resource per page of the [manual](README.md), named like the
  page (`hla-wgs://manual/typing`, `.../log`, `.../errors`, `.../switches`, ...), and
  `hla-wgs://manual/output` for the [output formats](../output.md)
- `file://PREFIX.json`, `.report.json`, `.tsv`, `.conf.tsv`, `.long.tsv`, `.log.jsonl`, `.log.txt` of the runs this
  server started; other files cannot be read through the server
