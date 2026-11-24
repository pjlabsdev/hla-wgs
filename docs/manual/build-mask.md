# Building the repeat mask

    hla-wgs build-mask DBDIR GENOME.fa [EXCLUDE.bed]
    hla-wgs build-mask DBDIR --from OLDDBDIR

Marks the parts of the allele panel whose 21-mers also occur outside the MHC in the genome
(repeats), so that reads from elsewhere are not counted for an HLA gene. Writes
`DBDIR/panel.fa.mask.bed`. Without a mask the typing still runs, but the result page and the JSON
say that no mask was used.

| Argument | Meaning |
|---|---|
| `DBDIR` | database folder from [build-db](build-db.md) |
| `GENOME.fa` | the reference genome, plain FASTA; GRCh38 is assumed |
| `EXCLUDE.bed` | the MHC region to leave out, for another assembly than GRCh38 |
| `--from OLDDBDIR` | carry the mask of an older database over, no genome needed |

With `--from`, alleles with unchanged sequence keep their intervals; for new or changed alleles the
positions whose 21-mers lie in masked intervals of the older panel are marked and joined by the
same rules. This is how the start file prepares another IPD-IMGT/HLA release
([start file](start-file.md)). The mask of release 3.66 is in the repository as
`data/panel_mask_3.66.bed`.

The switches `HLA_MASKFRAC` and `HLA_MASKNOCOPY` are in [switches](switches.md). Errors: HW-E050,
HW-E031, see [error codes](errors.md).
