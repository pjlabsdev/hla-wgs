# Output

`hla-wgs DBDIR INPUT... PREFIX [threads]` writes three files.

## PREFIX.tsv

One line per gene.

| column | meaning |
|---|---|
| `gene` | gene name without the `HLA-` prefix |
| `allele1`, `allele2` | best database alleles (full name); `-` for a copy that is absent |
| `call1`, `call2` | the safe call of each allele without the gene prefix: only the fields the reads support (as `safe_call` in `PREFIX.conf.tsv`) |
| `gap` | log-likelihood lead of the best allele pair over the next pair; high means clear. It is not a gap in coverage (stretches without reads are listed as `blindNbp` in `PREFIX.conf.tsv`) |
| `reads` | read pairs assigned to the gene |
| `copies` | 0, 1 or 2; estimated for DRB3, DRB4, DRB5, H and K, otherwise 2 |

When `copies` is 0 the gene is absent on both chromosomes: all four allele columns are `-`, and
`PREFIX.conf.tsv` has no line for it; in `PREFIX.json` it has `"called": true`, `"copies": 0` and no
alleles. When `copies` is 1 the second position is `-`. A gene without a result (too few reads) has
`-` in every column except `reads`, and `"called": false` in `PREFIX.json`.

## PREFIX.conf.tsv

One line per allele (`slot` 1 or 2).

| column | meaning |
|---|---|
| `allele` | best database allele |
| `safe_call` | `allele` cut to `resolution` fields; only the gene name when `resolution` is 0 |
| `resolution` | number of fields supported by the reads, 0 to 4; 0 when an allele of another allele group explains the reads about as well (lead at field 2 below 1) |
| `margin2`, `margin3`, `margin4` | margin to the best competitor that differs at field 2, 3 or 4; `-` when there is none |
| `alt2`, `alt3`, `alt4` | that competitor |
| `new_sites` | number of positions at which the reads differ from `allele`; `-` if not examined |
| `new_detail` | up to 20 sites as `position:change:reads/depth[:exon][:either]`; prefixed with `known:OLD+` when the call was renamed from `OLD` because `OLD` plus the sites equals `allele` |

A field counts as supported when its margin is at least 6 (fields 2 and 3) or 4 (field 4). A site in
an exon cuts the resolution to 1 field, any other site to 3 fields. Positions are 1-based on the
genomic sequence of `allele` in IPD-IMGT/HLA. `blindNbp` marks a stretch of N bases without any
read. `either` means the site could not be assigned to one of the two alleles.

## PREFIX.json

Schema `genome.hla.v1`.

```json
{
  "tool": "hla-wgs", "version": "1.0.0", "schema": "genome.hla.v1",
  "thresholds": {"2": 6.0, "3": 6.0, "4": 4.0},
  "genes": [
    {
      "gene": "DPB1", "called": true, "copies": 2, "reads": 1341, "gap": 1.52,
      "alleles": [
        {
          "allele": "DPB1*01:01:01:02",
          "safe_call": "DPB1*01:01:01",
          "resolution": 3,
          "renamed_from": null,
          "margin": {"2": 29.8, "3": 29.8, "4": 29.8},
          "alternative": {"2": "DPB1*979:01", "3": "DPB1*979:01", "4": "DPB1*01:01:12"},
          "new_sites": [
            {"pos": 3468, "change": "T>A", "reads": 20, "depth": 33, "exon": false, "either_allele": true,
             "region": "intron 2", "effect": "non-coding"}
          ]
        }
      ]
    }
  ]
}
```

Genes with an estimated copy number also carry `copy_ratio` (depth relative to the two-copy
genes). `margin` values are `null` when no competitor exists at that field; `new_sites` is `null`
when the allele was not examined.

## PREFIX.novel.fa

Written when at least one allele has new sites that can be applied. One record per such allele:

```
>B_slot1_new nearest=B*15:03:01:02 changes=4022:G>A
```

`incomplete=` lists sites that are not part of the sequence (blind stretches, repeat lengths,
unsupported positions that could not be applied); `phase=uncertain` says that at least one change
was seen only in reads that fit both alleles. The sequence is a proposal from short reads, not a
confirmed allele.

## PREFIX.novel.tsv

Written together with `PREFIX.novel.fa`: a worksheet for looking at a candidate allele and for
preparing a submission to IPD-IMGT/HLA. One line per difference from the nearest database allele.

| column | meaning |
|---|---|
| `gene`, `slot`, `record` | gene, allele slot and the name of the record in `PREFIX.novel.fa` |
| `nearest_allele` | database allele the sequence is derived from |
| `pos`, `change` | position on that allele's genomic sequence (from its first base) and the difference |
| `region` | `exon N`, `intron N`, `5' UTR` or `3' UTR` |
| `cds_pos` | position in the coding sequence, from the start codon (exonic sites) |
| `effect` | `non-coding`, `splice site` (the two intron bases next to an exon), `synonymous p.L45=`, `missense p.R68K`, `stop gained`, `stop lost`, `start codon lost`, `frameshift`, `in-frame length change`, `after an early stop codon`; codons count from the start codon, not from the mature protein, and each change is judged on its own |
| `reads`, `depth` | reads that show the difference and reads that cover the position |
| `assignment` | `this allele`, or `either allele` when the reads cannot tell which copy carries it |
| `in_sequence` | whether the difference is part of the FASTA record |
| `status` | per record: `complete`, `phase uncertain` (at least one site may belong to the other copy) or `incomplete` (a site is not in the sequence, or a stretch was not seen) |

The same `region` and `effect` are given for every entry of `new_sites` in the JSON. A candidate
from short reads is not a confirmed allele; the database's submission rules ask for more than a
short-read proposal, so treat the worksheet as the starting point for a confirmation (long reads or
Sanger sequencing of the separated allele).

## PREFIX.long.tsv and PREFIX.long.fa (`hla-wgs long`)

One line per gene sequence found in the long reads.

| column | meaning |
|---|---|
| `gene`, `slot` | gene and running number, most reads first |
| `allele` | nearest full-length database allele |
| `differences` | edits between this allele (over its whole length) and the sequence built from the reads; 0 means the four-field name is exact |
| `exact` | `yes`, `yes, with unsure sites` or `no` |
| `reads`, `spanning` | reads in the group, and how many of them cover the whole gene |
| `unsure_sites` | positions at which 30 % or more of the reads disagree with the sequence, nearly always long single-base runs |
| `equal_alternatives` | other alleles at the same distance |
| `note` | `minor`: a third sequence of a gene or one with few reads; not written to the FASTA |

One line for a gene means that both copies are identical or that there is one copy. Besides the typed
genes the table lists what else lies on the reads (for example HLA-J, L, DPA2, DPB2, DQA2).
`PREFIX.long.fa` holds each sequence over the extent of its nearest allele.

## Groups and pharmacogenetic alleles (JSON)

`gene_cap` is `true` when the resolution was limited by the gene and not by the reads: DMB and TAP2
are reported to three fields at most, because their fourth field was not reliable enough in validation.

With `hla_nom_g.txt` and `hla_nom_p.txt` (folder `wmda` of the IPD-IMGT/HLA release) in the database
directory, every allele carries `g_group` and `p_group` (`null` when the allele is alone in its
group), and the top level has `"groups_loaded": true`. `expression` explains a suffix letter of the
allele name. `pharmacogenetic_alleles` lists B\*57:01, B\*15:02, A\*31:01 and B\*58:01 with the number
of copies called and `certain` (all calls concerned are certain at two fields or more). Research use
only.

With `rel_dna_ser.txt` and `rel_ser_ser.txt` (folder `wmda`) in the database directory, every allele
supported to two fields or more carries `serology`: the WHO serological antigen, a split with its
broad antigen in brackets (`B60 (B40)`), `–` for a null allele; `serology_assumed` is `true` when the
assignment is assumed or by experts rather than tested; the top level has `serology_loaded`. HLA-DRB3,
-DRB4 and -DRB5 give DR52, DR53 and DR51; HLA-DQA1, -DPA1 and -DPB1 have none. `ligand` is the ligand of
the natural killer cell receptors, read from the protein (residues 77 to 83 of the mature chain):
`Bw4-80I` or `Bw4-80T` (HLA-A and -B), `Bw6` (HLA-B), `C1` or `C2` (HLA-C, residue 80).
`input` names the input files (without folders), `analysis_time` is the time of the run (seconds
since 1970, UTC). `depth_estimate` is a rough depth over the HLA genes (read pairs of ten genes always present in two copies, scaled to their median at 30x in the validation); below 18 the page says that fewer fields are supported and the fourth field is somewhat less often exact. `database_release` is the IPD-IMGT/HLA release of the database (`null` for a database built
before 1.1).

## Environment switches

For experiments; the defaults are the validated configuration.

| variable | effect |
|---|---|
| `HLA_NONOVEL` | skip the new-allele examination |
| `HLA_NOREP` | skip the repeat-length examination |
| `HLA_IO_RETRIES` | number of retries after an input I/O error (default 6); wrong or damaged content (no index, truncated file, not a FASTQ) is not retried but reported at once |
| `HLA_FULLKMERS` | set to use all panel 21-mers in the FASTQ prefilter, including those inside the repeat mask (slower, the behaviour before 1.1) |
| `HLA_SAVE_READS` | prefix for a FASTQ copy of the read pairs that pass the panel prefilter |
| `HLA_NOKNOWN` | do not rename a call to the database allele that its new sites describe |
| `HLA_NOBIG` | count long insertions and deletions per base instead of as one event |
| `HLA_UNPH` | also cut the fourth field when a sibling allele differs only at positions without phase information (fewer wrong, clearly fewer certain calls) |
| `HLA_PHASE`, `HLA_BREAK` | experimental phase and break-point rules, off by default |

## PREFIX.html

One self-contained page with the result for reading and printing, built from the values of
`PREFIX.json`, so it holds everything that `PREFIX.tsv` and `PREFIX.conf.tsv` hold: per gene the
safe call of each allele, the number of supported fields, the nearest full name where it goes
further, notes (one copy, gene absent, differences from the database allele, expression suffix), the
pharmacogenetic alleles, and the citation with DOI and the link to the research article. The section
"Details per gene" opens per gene the reads, copies, lead of the best pair and copy ratio, and per
allele the G and P groups, the lead at fields 2 to 4 against its threshold with the nearest competitor,
the reason why the safe call stops where it does, and every differing site.

The page holds German and English; the buttons at the top switch between them, the choice is kept in
the browser, and printing uses the chosen language with all details opened. It opens in German when
`HLA_WGS_LANG=de` is set or `LANG` starts with `de`, otherwise in English. It loads nothing from the
network.
