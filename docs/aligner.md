# The aligner of hla-wgs

From 1.1 on, hla-wgs aligns reads with its own aligner in Rust (`src/aln.rs`) instead of the C
library minimap2. The program is now Rust only: building it needs no C compiler, and the binary
links only the system libraries (libc, libm and on Linux libgcc_s).

The aligner is written for the one job it has here: reads against many very similar sequences of
the same gene family (the 1195 allele-group sequences of the panel, and alleles against each other
when the database is built and when short alleles get the flanks of a longer one). Scores and
thresholds follow the minimap2 presets, so every threshold of the typer keeps its meaning.

A picture of the whole program, from the database download to the result page, is in
[`docs/ablauf.html`](ablauf.html) (German; open it in a browser).

## How a read is aligned

1. **Seeds.** The index holds every k-mer of every sequence (k = 21 for short reads, 19 for
   assemblies and long reads). Of the read, only minimizers are looked up: of every 11 consecutive
   k-mers the one with the smallest hash. Since the index keeps all k-mers, each looked-up k-mer
   finds all its matches. A read whose best chain covers less than 80 % of its length (variants,
   a new allele, the end of a gene) is looked up again with every k-mer, so the sampling costs no
   sensitivity where it matters.
2. **Candidates per gene.** The sequences of a gene (the name before `*`) are ranked by their
   number of anchors; per gene the 25 best and all that tie with the best are chained.
3. **Chaining.** Anchors on one sequence and strand are chained as in minimap2: an anchor gains up
   to k, a gap costs 0.01·k·d + 0.5·log2 d for a diagonal change d; consecutive anchors on the same
   diagonal are joined directly.
   Alleles of one gene give the same anchors, only shifted by where the stretch starts in each
   sequence; chaining depends only on distances, so the chains are computed once per anchor shape.
4. **Alignment.** Between anchors the alignment is global, beyond the first and last anchor it is an
   extension with z-drop and an end bonus. Gaps have two-piece affine costs (short reads: match 2,
   mismatch 8, gap 12 + 2 per base or 24 + 1 per base). Large gaps between anchors are split at
   13-mers that occur once on both sides, so only the pieces between them need the full matrix.
   Gaps are moved to their leftmost equivalent position, so the same event gets the same position
   in every read.
5. **Vector kernel.** The matrices are filled by anti-diagonals: the cells of one anti-diagonal do
   not depend on each other, so the inner loop compiles to vector instructions (AVX2 on x86,
   chosen at run time; NEON on ARM). The rows are summarised as they complete, so the best end,
   the z-drop stop and the traceback are exactly those of a row-by-row fill; the tests check this
   cell by cell.
6. **Memory by content.** Alleles of one gene share long stretches. An alignment of a large piece
   is stored under the content of both pieces; the next allele pair with the same pieces looks it
   up instead of computing it again. Small alignments are remembered per thread the same way. Per
   read, the alignment of a chain is stored under the stretch of the sequence it can reach, so
   alleles that agree there are aligned once.

## Read pairs and genes

Each mate is aligned on its own. A mate without a hit in the gene of its partner's best hit is
searched next to the partner (up to three genes within 40 points of the partner's best): the
stretch 800 bases around the partner, restricted to the two diagonals with the most shared
11-mers (a diagonal and its neighbours count as one). The local alignment there is computed in a
band of 24 bases around the diagonals of the shared 11-mers; a stretch that shares fewer than three
11-mers with the mate is not aligned at all (on HG00438 none of 282 such stretches gave a hit). A pair whose best same-gene pair score plus 33 (minimap2's pairing bonus) reaches the sum
of the mates' best scores goes to that gene with both mates.

A hit needs 70 points (about 35 matching bases), or 60 points when the read runs over the start
or end of the panel sequence: such a mate is only partly aligned but is still a read of the gene.
On the HGSVC3 samples this rule kept the reads at the ends of DQB1 and HLA-A that a flat
threshold of 80 lost, which had made DQB1\*05:02 and A\*02:01:01:01 less certain.

## Presets

| preset | used for | k | scores (match, mismatch, gap) | band |
|---|---|---|---|---|
| `sr` | read pairs against the panel | 21 | 2, 8, 12+2l / 24+l | 32 |
| `sr_single` | reads against one allele (new sites, exon refinement) | 15 | as `sr` | 32 |
| `asm10` | allele against allele (database, flanks) | 19 | 1, 9, 16+2l / 41+l | 500 |
| `asm5` | alleles against the consensus of a long-read haplotype | 19 | 1, 19, 39+3l / 81+l | 500 |
| `hifi` | long reads against the panel | 19, every 4th k-mer | 1, 4, 6+2l / 26+l | 100 |

## Validation

Same reads, same database (IPD-IMGT/HLA 3.66), only the aligner changed. Truth: allele sequences
read from the HPRC and HGSVC3 assemblies.

| | minimap2 (1.1 before) | own aligner |
|---|---|---|
| 80 HPRC samples, two fields | 2486 / 2489 | 2485 / 2489 |
| 80 HPRC, three fields | 2482 / 2489 | 2481 / 2489 |
| 80 HPRC, certain four-field calls exact | 1778 / 1801 | 1779 / 1798 |
| 80 HPRC, reported new sites that are true | 1382 / 1522 | 1385 / 1530 |
| 80 HPRC, eight added genes exact at exon level | 986 / 997 | 986 / 997 |
| 8 HPRC samples, time per sample (4 threads, machine otherwise idle) | 31.5 s wall, 116 s CPU | 22.2 s wall, 79 s CPU |
| 59 HGSVC3 samples, two fields | 1833 / 1833 | 1833 / 1833 |
| 59 HGSVC3, three fields | 1830 / 1833 | 1830 / 1833 |
| 59 HGSVC3, certain four-field calls exact | 1337 / 1354 | 1333 / 1349 |
| 59 HGSVC3, reported new sites that are true | 743 / 876 | 739 / 876 |

The one allele of the 80 that differs at two fields is HLA-A of HG01891 (homozygous A\*30:01 by
the assemblies): with the own aligner the reads between HLA-A and the pseudogenes HLA-Y and HLA-H
fall slightly differently, and A\*30:207 is called as the second allele.

On the 59 HGSVC3 samples (never used for development) the own aligner reports four fewer alleles
as certain at four fields and has one wrong certain call fewer (16 against 17). The new sites it
misses lie mostly near a gene end or in a short repeat, where a few reads decide whether a site
passes the share threshold; making those rules stricter was tested (`HLA_NOV_EDGE=75`) and not
taken, because on the 80 HPRC samples it added three wrong certain calls. One of the 17 wrong
certain calls of minimap2, HLA-V of NA20509, is a real 21-base duplication on one haplotype that
the own aligner sees and therefore reports to three fields only.

Simulated reads: 16,767 read pairs from 47 genes, half of them from alleles that are not in the
panel (as a new allele would be), 0.3 % substitutions. Pairs assigned to their own gene only:
93.0 % with minimap2 and 92.96 % with the own aligner; to a wrong gene 0.52 % and 0.54 %; the rest
are left out by the repeat mask or lie in genes outside the panel in both. The simulator is
`hla-wgs-validate sim` (scoring: `hla-wgs-validate sim-eval`).

## Self-checks

`cargo test` runs the aligner against an exhaustive full-matrix alignment on random sequences
(all four score presets, substitutions, small and large indels, N): global alignments must reach
the optimum and report a path that scores what they claim; banded alignments must reach it when
the path fits the band; extensions must end at the best cell; split gap fills must cost nothing on
allele-like pairs; gaps must sit at their leftmost position; the anti-diagonal kernels must equal
the row-wise ones cell by cell; reads of an allele must score best on that allele; mapping must
not depend on the number of threads. `HLA_PROPS=n` runs n times as many random cases. These checks
found one error, now fixed: the shortcut for an ungapped read end missed a better alignment with a
gap when the end bonus paid for it.

`hla-wgs-validate sim` simulates read pairs from IPD-IMGT/HLA alleles with profiles `illumina`,
`lowq` (errors rising to 4 % at the read end), `mgi` (DNB-like) and `hard` (plus 2 % chimeric pairs),
and `--div` adds divergence to every source allele, as for a new allele far from the panel.

## Measuring

`HLA_ALNSTAT=1` prints, after the panel step and at the end, how many alignment matrices were
computed, their cells and time, and the time spent seeding and chaining.

| variable | effect |
|---|---|
| `HLA_PANEL_S` | score a panel hit needs (default 70) |
| `HLA_PANEL_SE` | score of a hit that runs over the end of the panel sequence (default 60) |
| `HLA_PANEL_MW` | minimizer window for read seeds (default 11; 1 looks up every k-mer) |
| `HLA_PANEL_FULL` | reseed with every k-mer when the best chain covers less than this percentage of the read (default 80) |
| `HLA_PANEL_G` | chained sequences per gene (default 25) |
| `HLA_PANEL_B` | pairing bonus (default 33) |
| `HLA_PANEL_RESCUE=0` | no mate rescue |
| `HLA_PANEL_RS` | score a rescued mate needs (default as `HLA_PANEL_S`) |
| `HLA_PANEL_Q` | mismatches at bases below this quality cost less (default 0 = off; no gain on the simulated profiles) |
| `HLA_RESCUE_V` | shared 11-mers a rescue stretch needs (default 3) |
| `HLA_RESCUE_BAND` | indel margin of the banded rescue alignment (default 24; 0 = whole window) |
| `HLA_EXT_AD=0`, `HLA_RESCUE_AD=0` | row-wise instead of anti-diagonal kernels (same results, slower) |
| `HLA_ALNSTAT` | print alignment statistics |
