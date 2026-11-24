# 1000 Genomes laboratory typing, 1101 samples

All 1000 Genomes 30x samples with laboratory HLA types (Gourraud et al. 2014: 1101 samples, A, B, C,
DRB1, DQB1; Abi-Rached et al. 2018: 967 of them) except HG00619 and HG00620, whose CRAM index at
1000 Genomes is empty. Version: main ee05999 (2026-11-22, before the DRB1 change of the unreleased
version). Reads from the MHC region, ALT/HLA contigs and unmapped reads, `lab/run24.sh`.

`calls.tar.gz` holds `SAMPLE.tsv` and `SAMPLE.conf.tsv` of every sample. Unpack it and run

    HLA_OUT=DIR hla-wgs-validate eval24 2018     # -> eval24_2018.txt
    HLA_OUT=DIR hla-wgs-validate eval24 2014     # -> eval24_2014.txt

T1K was run on a subset (83 and 86 samples) for comparison.

| laboratory set | hla-wgs | on the T1K subset: hla-wgs | T1K |
|---|---|---|---|
| 2018 (two fields) | 9051 / 9090 | 774 / 776 | 764 / 776 |
| 2014 | 10754 / 10985 | 837 / 860 | 827 / 860 |

The 39 differing alleles in the 2018 set were checked one by one (a sample can count twice, see the `ERR` lines of `eval24_2018.txt`):

- in 22 cases the two alleles are identical in the exons the laboratory typed (exons 2 and 3 for class I, exon 2 for class
  II), so the laboratory could not tell them apart (for example C*04:82 against C*04:01, which
  differ only by a 9-base insertion in exon 5);
- 1 is an old name not in IPD-IMGT/HLA 3.66 (A*24:01), 1 is a parse error in the laboratory file
  (HG00592, "0,084");
- 13 are real differences in the typed exons, among them NA18620 C*04:81 (31 reads support it; T1K
  says 04:82) and NA18606 C*03:04 against 03:03.

Where a laboratory type and an HPRC assembly disagree, the assembly has so far agreed with hla-wgs
(HG01358: DRB1*04:92, laboratory 04:07).

The 2014 set is older and has more ambiguous and lower-resolution entries; most of its extra
differences are of the same kinds.
