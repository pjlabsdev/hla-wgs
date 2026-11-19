# Output

`hla-wgs DBDIR INPUT... PREFIX [threads]` writes three files.

## PREFIX.tsv

One line per gene.

| column | meaning |
|---|---|
| `gene` | gene name without the `HLA-` prefix |
| `allele1`, `allele2` | best database alleles; `-` when the copy is absent |
| `call1`, `call2` | the same without the gene prefix |
| `gap` | log-likelihood margin of the best pair over the next pair |
| `reads` | read pairs assigned to the gene |
| `copies` | 0, 1 or 2; estimated for DRB3, DRB4, DRB5, H and K, otherwise 2 |

When `copies` is 0 the allele columns still show the best-scoring names; ignore them.

## PREFIX.conf.tsv

One line per allele (`slot` 1 or 2).

| column | meaning |
|---|---|
| `allele` | best database allele |
| `safe_call` | `allele` cut to `resolution` fields |
| `resolution` | number of fields supported by the reads, 1 to 4 |
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
            {"pos": 3468, "change": "T>A", "reads": 20, "depth": 33, "exon": false, "either_allele": true}
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

## Groups and pharmacogenetic alleles (JSON)

With `hla_nom_g.txt` and `hla_nom_p.txt` (folder `wmda` of the IPD-IMGT/HLA release) in the database
directory, every allele carries `g_group` and `p_group` (`null` when the allele is alone in its
group), and the top level has `"groups_loaded": true`. `expression` explains a suffix letter of the
allele name. `pharmacogenetic_alleles` lists B\*57:01, B\*15:02, A\*31:01 and B\*58:01 with the number
of copies called and `certain` (all calls concerned are certain at two fields or more). Research use
only.

## Environment switches

For experiments; the defaults are the validated configuration.

| variable | effect |
|---|---|
| `HLA_NONOVEL` | skip the new-allele examination |
| `HLA_NOREP` | skip the repeat-length examination |
| `HLA_IO_RETRIES` | number of retries after an input I/O error (default 6) |
| `HLA_SAVE_READS` | prefix for a FASTQ copy of the read pairs that pass the panel prefilter |
| `HLA_NOKNOWN` | do not rename a call to the database allele that its new sites describe |
| `HLA_NOBIG` | count long insertions and deletions per base instead of as one event |
| `HLA_UNPH` | also cut the fourth field when a sibling allele differs only at positions without phase information (fewer wrong, clearly fewer certain calls) |
| `HLA_PHASE`, `HLA_BREAK` | experimental phase and break-point rules, off by default |
