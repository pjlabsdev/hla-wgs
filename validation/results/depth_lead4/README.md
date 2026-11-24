# Certain two- and three-field calls at 30x, 15x and 10x (lead of 4)

The 80 HPRC development samples (`hprc/samples_dev80.txt`), typed at full depth (about 30x) and after
thinning the read pairs to one half (15x) and one third (10x) with `hla-wgs-validate thin-pairs`.
`hla-wgs-validate eval-res DIR` counts the calls that are certain to at least two and at least three
fields and how many of them agree with the assembly truth (`truth/truth4.tsv`) at that many fields.

`eval_res.txt`, one line per run (right / certain):

| depth | lead 6 (before) | lead 4 (now) |
|---|---|---|
| 30x | two fields 2344 / 2350, three fields 2251 / 2268 | 2353 / 2359, 2262 / 2279 |
| 15x | 2255 / 2259, 2138 / 2154 | 2306 / 2310, 2206 / 2222 |
| 10x | 1917 / 1921, 1724 / 1738 | 2218 / 2222, 2073 / 2088 |

Runs: before = main after the DRB1 change (`newg_dr2`, `newg_l15`, `newg_l10`), now = the same with
`t_group()` = 4 (`newg_tg`, `newg_tg15`, `newg_tg10`). `HLA_TGRP=6` gives the old threshold.
