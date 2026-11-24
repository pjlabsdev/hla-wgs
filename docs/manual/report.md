# Report part

    hla-wgs report PREFIX.json [--text de|en]

Every typing run writes `PREFIX.report.json` next to `PREFIX.json`: the report part of hla-wgs for
the overall report of Genome (schema `genome.report.v1`). The command writes it again from
`PREFIX.json` without typing again, for example after an update of the program. With `--text` it
also prints the part as plain text in German or English. Long-read runs (`hla-wgs long`) do not
write a report part yet.

The part holds the presentation, not new facts: everything in it is derived from `PREFIX.json` and,
when it lies next to it, the header of the [run log](log.md). The overall report assembles such parts
of all tools and computes nothing itself, so every number it shows can be traced to one part and
from there to one entry of the result (`source`).

## Parts

| Field | Content |
|---|---|
| `schema` | `genome.report.v1` |
| `head` | tool, version, commit, sample, inputs (name, bytes, CRC-32 from the run log), data release (`IPD-IMGT/HLA`), date, run time, the result file with its CRC-32, research-use notice |
| `findings` | key findings: one per clinical locus (A, B, C, DRB1, DRB3/4/5, DQA1, DQB1, DPA1, DPB1) with the supported names, the certainty (`level`: `certain` when every allele is supported to two fields or more, `partial`, `none`; `fields` per allele) and the nearest alternative at the first unsupported field; one per pharmacogenetic allele (carrier, not a carrier, or undecided) with the CPIC recommendation and guideline for carriers; one per possible new allele |
| `results` | the GL string, a table of all alleles, a table of the genes, and a chart as data (alleles by supported fields), so that every report draws it in its own style |
| `quality` | `ok` and the items that matter for this tool: estimated depth (at least 18x for the full resolution), repeat mask, database release, G/P groups and serology, clinical loci typed, read pairs read and assigned |
| `method` | the method in a few sentences and the research report with its DOI |
| `validation` | the validation of this version (80 HPRC and 59 unseen HGSVC3 genomes); a test keeps these numbers equal to `validation/results` |
| `limits` | the limits of the method, and those of this sample (`sample: true`): low depth, loci not typed, no repeat mask |
| `references` | sources in APA style with DOI or URL |
| `messages` | all texts in German (`de`) and English (`en`), Fluent syntax, one message per line |
| `checksum` | CRC-32 of the part as compact JSON without this field |

## Texts

Every text is a message key with arguments, e.g.

    {"key": "hla-genotype", "args": {"gene": "A", "a1": "A*24:02:01:01", "a2": "A*02:01"}}

and the messages are in the part (`hla-genotype = HLA-{ $gene }: { $a1 } and { $a2 }`), so a
report can be set in either language without a new run. Arguments that are themselves texts (the
reason why a locus was not typed) are given as a key object again. A test checks that both languages
have every message with the same arguments and that every key used in a part is there.

Errors: HW-E021 (not a result file), HW-E020 (cannot write), see [error codes](errors.md).
