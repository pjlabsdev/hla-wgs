# The 30 check samples with the 1.1 development version

The 30 samples of `../check30/` (typed there once with the released 1.0.0) were typed again with the
1.1 development version (repeat-length check, 26 genes); same truth, same scripts (`eval6.tsv` for four
fields, `eval23.tsv` for two and three fields).

| | 1.0.0 | 1.1 (development) |
|---|---|---|
| two fields correct | 935 / 935 | 935 / 935 |
| three fields correct | 934 / 935 | 934 / 935 |
| known alleles, four fields exact | 785 / 795 | 785 / 795 |
| reported as certain at four fields | 621 | 615 |
| of these exact | 607 | 607 |
| of these not exact | 14 | 8 |
| not exact for a reason other than a homopolymer or repeat length | 5 | 4 |

The calls themselves did not change; six alleles that were wrongly reported as certain at four fields
are now cut to three fields (DRB5 three, DRB1, DQA1 and DPB1 one each). No exact four-field call lost
its certainty. These samples were not used to develop the repeat-length check, but they are the same
30 samples as before, so this is a repeat measurement and not a new independent test.
