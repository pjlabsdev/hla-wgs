# Building the database

    hla-wgs build-db hla.dat freq.json DBDIR

Builds the database folder from the `hla.dat` of an IPD-IMGT/HLA release. The IPD-IMGT/HLA data are
licensed CC BY-ND and are not part of the repository, so every user builds the database locally.

| Argument | Meaning |
|---|---|
| `hla.dat` | unpacked `hla.dat` of a release (`hla-wgs unzip hla.dat.zip DIR`, see [unzip](unzip.md)) |
| `freq.json` | two-field allele frequencies, `data/freq_1000g_2field.json` from the repository |
| `DBDIR` | folder to write; created if missing |

    curl -LO https://ftp.ebi.ac.uk/pub/databases/ipd/imgt/hla/hla.dat.zip
    hla-wgs unzip hla.dat.zip .
    hla-wgs build-db hla.dat data/freq_1000g_2field.json hladb

It takes about four minutes and writes `hladb.json.gz`, `panel.fa`, `format` and `release`; the
contents are described in [database folder](database.md). Then build the repeat mask with
[build-mask](build-mask.md). The start file does both on its first start
([start file](start-file.md)).

Errors: HW-E030 (not a hla.dat), HW-E031 (folder not writable), see [error codes](errors.md).
