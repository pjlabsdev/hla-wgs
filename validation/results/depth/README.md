# Depth study

How the result changes with less sequencing depth. The 30x read pairs were thinned by a hash of
the read name (`hla-wgs-validate thin-pairs`, fractions 0.667, 0.5 and 0.333), so both mates of a
pair stay together and the same pairs are kept in every run. Program: 1.1 development version with
the depth-dependent minimum of reads for a new site; database IPD-IMGT/HLA 3.66.

## 80 HPRC samples, 18 genes, against the assemblies

| depth | two fields right | three fields right | certain at four fields | of these exact |
|---|---|---|---|---|
| 30x | 2486 / 2489 | 2482 / 2489 | 1749 | 1728 (98.8 %) |
| 20x | 2485 / 2489 | 2482 / 2489 | 1660 | 1640 (98.8 %) |
| 15x | 2481 / 2489 | 2475 / 2489 | 1539 | 1517 (98.6 %) |
| 10x | 2476 / 2488 | 2469 / 2488 | 1082 | 1059 (97.9 %) |

Before the depth-dependent minimum (five reads for a new site at every depth), the certain
four-field calls that were not exact numbered 22 at 20x, 27 at 15x and 36 at 10x. Per-gene tables:
`eval6_*x.tsv` (columns as in `../dev80_v1.1`).

Fewer reads mainly cost fields: at 10x about 40 % fewer alleles are certain at four fields, while
the two-field call stays right in 2476 of 2488 alleles. For a dependable fourth field, 15x or more
over the HLA genes is advisable.

## GIAB trio and one private sample, against the result at full depth

HG002, HG003, HG004 (30x) and one private 30x sample, each thinned twice with different seeds;
the certain call (`safe_call`) compared with the one at full depth of the same sample.

| depth | same | fewer fields | more fields | contradicts |
|---|---|---|---|---|
| ~20x | 339 | 34 | 1 | 2 |
| ~15x | 294 | 76 | 5 | 1 |
| ~10x | 187 | 182 | 7 | 0 |

The three contradictions are one case: TAP2 of HG004 at 20x and 15x named allele group 01 instead
of 02 (the trio shows 02), with a lead of 0.6 at field 2. Since this study, a group whose best
competitor of another group explains the reads about as well (lead below 1) is no longer reported
as certain. "More fields" at lower depth means a new site was not seen any more; the
depth-dependent minimum of reads for a new site addresses this. The genes that lose fields first
are E, F, DPB1, DRA, DRB1, DQA1 and DQB1, where the last field rests on few positions.
