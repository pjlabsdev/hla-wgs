# HLA-E at four fields

Up to 1.0 the safe call of HLA-E was capped at three fields. With the 1.1 development version the
fourth field was checked against the assembly truth wherever the reads support it (lead at field 4 at
least 4, no differing site):

| set | E alleles reported certain at four fields | exact |
|---|---|---|
| 80 HPRC development samples (typed again without the cap) | 146 | 146 |
| 30 check samples (`../check30_v1.1`, cap removed on the reported leads) | 58 | 58 |
| 38 unseen HGSVC3 samples (`../hgsvc3_v1.1`, cap removed on the reported leads) | 71 | 70 |
| all | 275 | 274 |

The cap was removed in 1.1.
