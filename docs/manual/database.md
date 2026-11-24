# Database folder

`hla-wgs build-db` ([build-db](build-db.md)) and `hla-wgs build-mask` ([build-mask](build-mask.md))
write the database folder `DBDIR`. Typing reads it; nothing in it is changed by a run except the
padding cache.

| File | Written by | Content |
|---|---|---|
| `hladb.json.gz` | build-db | every allele of the typed genes with its sequence, exons and the frequency prior |
| `panel.fa` | build-db | the allele panel the reads are mapped to (one full-length sequence per allele group) |
| `format` | build-db | format number of the database; `hla-wgs --db-format` prints the number the program needs |
| `release` | build-db | IPD-IMGT/HLA release, e.g. `3.66.0` |
| `panel.fa.mask.bed` | build-mask | repeat mask (`HLA_MASK` names another file) |
| `pad_cache.tsv.gz` | first typing run | alignments of database alleles against each other; later runs read them (`HLA_PAD_CACHE`) |
| `hla_nom_g.txt`, `hla_nom_p.txt` | copied by hand or by the start file | WMDA G and P groups of the same release, for the JSON and the page |
| `rel_dna_ser.txt`, `rel_ser_ser.txt` | copied by hand or by the start file | WMDA serological equivalents |
| `repeat_decoys.fa` | optional | repeat elements inside genes, `>GENE\|location` (`HLA_DECOYS` names another file) |

## Format and release

When `format` holds a lower number than the program needs, the run warns and the result for DMB,
DOB, MICA, MICB, TAP1 and TAP2 is better after building the database again. The release is written
into the [run log](log.md) header, `PREFIX.json` (`database_release`) and the result page. A
database built before 1.1 has no `release` file; the release is then read from the header of
`hla_nom_g.txt` when that file is there.

## Licence

The files hold IPD-IMGT/HLA sequences (CC BY-ND 3.0). Do not publish the folder; build it from the
original `hla.dat`.
