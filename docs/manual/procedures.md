# Procedures

## Set up

With the start file nothing needs to be prepared ([start file](start-file.md)). By hand:

1. Build the program (`cargo build --release`) or take it from a release package.
2. `hla-wgs unzip hla.dat.zip .` and `hla-wgs build-db hla.dat data/freq_1000g_2field.json hladb`
   ([build-db](build-db.md)).
3. `hla-wgs build-mask hladb GRCh38.fa`, or copy `data/panel_mask_3.66.bed` to
   `hladb/panel.fa.mask.bed` for release 3.66 ([build-mask](build-mask.md)).
4. Optional: the four WMDA files of the same release into `hladb/` ([database folder](database.md)).

## Type a sample

    hla-wgs hladb sample.bam out/sample 4

([typing](typing.md)). For HiFi reads `hla-wgs long` ([long](long.md)).

## Check a result

1. `out/sample.html` shows the safe call of every allele and, under "Details per gene", why it
   stops where it does ([output](../output.md)).
2. `out/sample.log.txt` holds every step with its counts and every automatic decision. The
   `certainty` decisions say for each allele which lead or which new site limits it
   ([run log](log.md)).
3. For one gene in full detail, run again with `--trace GENE`.

## Use another IPD-IMGT/HLA release

`HLA_WGS_RELEASE=3.67` with the start file, or by hand `build-db` with the new `hla.dat` into a new
folder and `hla-wgs build-mask NEWDB --from OLDDB`. Validation used release 3.66.

## Report a bug

1. `hla-wgs diag out/sample` writes `out/sample.diag.tar` ([diag](diag.md)). It holds the run log
   and the version, no reads and no results.
2. Open an issue at https://github.com/pjlabsdev/hla-wgs/issues with the error code
   ([error codes](errors.md)), what you expected, and the package attached.

## Follow a run with an agent

Register the MCP server (`claude mcp add hla-wgs -- /path/to/hla-wgs mcp`). The agent can then
start a run, follow its progress, read the result and ask `explain` why an allele was called as it
was ([MCP server](mcp.md)).
