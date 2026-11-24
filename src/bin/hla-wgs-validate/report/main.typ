// hla-wgs software report. Marked values come from validation/results (hla-wgs-validate paper).
#set document(title: "«title»", author: "Peter Jungwirth")
#set page(
  paper: "a4",
  margin: (top: 18mm, bottom: 20mm, left: 17mm, right: 17mm),
  footer: context align(center, text(size: 7.5pt, fill: rgb("#56544C"))[#counter(page).display() / #counter(page).final().first()]),
)
#set text(font: "IBM Plex Mono", size: 8.6pt, lang: "en", fill: rgb("#141413"))
#set par(justify: true, leading: 0.62em, spacing: 0.95em)
#set heading(numbering: none)
#show heading.where(level: 1): it => block(above: 1.4em, below: 0.7em, text(size: 11pt, weight: 500, it.body))
#show heading.where(level: 2): it => block(above: 1.1em, below: 0.55em, text(size: 9.2pt, weight: 500, fill: rgb("#a88600"), it.body))
#show strong: set text(weight: 500)
#show figure.caption: it => align(left, text(size: 7.6pt, it))
#set figure(gap: 0.7em)
#set table(stroke: (x, y) => if y == 0 { (bottom: 0.6pt + rgb("#141413")) } else { (bottom: 0.3pt + rgb("#D8D5CC")) }, inset: (x: 4pt, y: 3pt))
#show table: set text(size: 7.4pt)

#grid(columns: (1fr, auto), align: (left + horizon, right + horizon),
  text(size: 11pt, weight: 500)[PJ Labs],
  text(size: 7.6pt, fill: rgb("#56544C"))[Software report · preprint \ Version «version» · «date»])
#line(length: 100%, stroke: 1.2pt + rgb("#a88600"))

#v(0.6em)
#text(size: 15pt, weight: 500)[«title»]
#v(0.3em)
*Peter Jungwirth, MSc* · PJ Labs, Austria \
#text(size: 7.6pt, fill: rgb("#56544C"))[Software: https://github.com/pjlabsdev/hla-wgs · DOI: «doi» · Licence: MIT]

#v(0.6em)
#block(fill: rgb("#F6F6F4"), inset: 9pt, radius: 2pt, width: 100%)[
*Summary.* HLA alleles are named with up to four fields, and typing programs for whole-genome sequencing usually print all four. Whether the fourth field is right is rarely known, because laboratory typing ends at two fields and because many individuals carry alleles that are absent from the IPD-IMGT/HLA database at full gene length. _hla-wgs_ is a single Rust program, with its own read aligner, that types 18 HLA genes from short-read whole-genome sequencing and reports for every allele the best database name, the number of fields that the reads support and the positions at which the reads differ from that name. A second mode reads PacBio HiFi reads and assembles each allele. On «n_ho» held-out samples it made «h2_err» two-field errors where T1K made «h2_err_t1k», and called the copy number of DRB3, DRB4 and DRB5 correctly in «hc_hla» of «hc_n» cases (T1K «hc_t1k»). Of the alleles it reported as certain at four fields, «a6_s4e» of «a6_s4» were identical to the assembled sequence in 80 development genomes and «u6_s4e» of «u6_s4» in 59 genomes never used in development. Against laboratory typing of «l18_n_samples» 1000 Genomes samples it agreed with «l18_ok» of «l18_n» alleles at two fields (T1K on a subset: «l18_t1k» against «l18_sub» for hla-wgs).

*Availability.* Source code, the validation program and all result tables are available under the MIT licence at https://github.com/pjlabsdev/hla-wgs. The reference is built locally from the IPD-IMGT/HLA release; no database sequences are distributed.
]

#block(stroke: (left: 2pt + rgb("#a88600")), inset: (left: 8pt, y: 4pt))[
*Highlights*
- One program without external tools at run time: own aligner, FASTQ, BAM or CRAM in, 18 genes out.
- Every allele comes with the number of fields that the reads support and with the sites that contradict its name.
- «pct_new» of gene copies in 80 assembled genomes are new alleles that no four-field name can describe.
- «pct_s4» of four-field statements exact in development, «pct_u4» in 59 unseen genomes.
- «l18_ok» of «l18_n» alleles agree with laboratory typing of «l18_n_samples» 1000 Genomes samples.
- HiFi reads: «lr_ok» of «lr_n» assembled allele sequences reproduced base for base.
- Research use only.
]

#block(fill: rgb("#FBF4D9"), inset: 7pt, radius: 2pt, width: 100%)[*Research use only.* hla-wgs is not a medical device and has not been clinically validated. Its output must not be used for transplantation, pharmacogenetic or any other medical decision without confirmation by an accredited HLA laboratory.]

#figure(image("fig1.svg", width: 100%), caption: [*How hla-wgs works.* Once per installation, the reference is built from the IPD-IMGT/HLA release and a repeat mask from GRCh38. For every sample, reads are mapped to a panel of representative alleles with the program's own aligner, candidate pairs are ranked per gene, and the reads are realigned to the called pair; consistent differences and stretches without reads limit the number of fields reported as certain. HiFi reads take a separate path that assembles each allele before naming it. The same picture as an interactive page: `docs/ablauf.html`.]) <fig1>

= Introduction
The HLA genes in the major histocompatibility complex on chromosome 6 are the most polymorphic genes of the human genome. Their alleles determine transplant compatibility, are associated with autoimmune and infectious diseases and predict severe drug hypersensitivity. Alleles are named with up to four colon-separated fields: the first two describe the protein, the third synonymous changes in the coding sequence and the fourth changes in introns and untranslated regions [@nomen]. The IPD-IMGT/HLA database collects the known alleles [@imgt].

Whole-genome sequencing (WGS) covers the full length of every HLA gene, and several programs derive HLA types from short reads, among them OptiType [@optitype], HLA-HD [@hlahd], xHLA [@xhla], Kourami [@kourami], HLA\*LA [@hlala] and T1K [@t1k]. They are evaluated at two fields against laboratory typing, because that is the resolution of the available reference data [@gourraud, @abirached]. Their output nevertheless often carries three or four fields, and a user cannot tell whether the further fields are supported by the data, are the first of several equally good database entries, or describe the nearest known allele of a sequence that is not in the database.

Haplotype-resolved assemblies of the Human Pangenome Reference Consortium (HPRC) [@hprc] and of the Human Genome Structural Variation Consortium (HGSVC) [@hgsvc3] now provide the complete sequence of both copies of each HLA gene for samples that also have 30× short-read WGS from the 1000 Genomes Project [@kgp]. This allows two things that were not possible before: measuring four-field accuracy, and counting how often an individual carries an allele that the database does not contain. _hla-wgs_ was written with that reference in mind. Its aim is a correct type and, per allele, an explicit statement of how far the name can be trusted.

= Results
== Design
hla-wgs is one Rust program without run-time dependencies (@fig1). It reads paired FASTQ files or a coordinate-sorted BAM or CRAM file on GRCh38 or GRCh37; from alignments it takes the MHC region, the ALT and HLA contigs and the unmapped reads. It types HLA-A, -B, -C, -DRB1, -DRB3, -DRB4, -DRB5, -DQA1, -DQB1, -DPA1, -DPB1, -DRA, -E, -F, -G and the pseudogenes HLA-H, -K and -V. For DRB3, DRB4, DRB5, H and K, which are absent from many haplotypes, it estimates the copy number. All alignment is done by the program's own aligner (minimizer anchors, chaining and an affine-gap dynamic programme with left-aligned gaps); it replaced the minimap2 library [@minimap2] of version 1.0 at equal accuracy and lower run time.

For each allele the output gives the best database allele, a _safe call_ cut to the number of fields that the reads support (_resolution_, 1 to 4), the lead over the best competing pair at fields two, three and four, and a list of _new sites_: positions at which the reads consistently differ from the called allele, or stretches that no read covers. If the called allele together with its new sites equals another database allele, the call is renamed to that allele. The output is two tables, JSON, a list of new sites and a result page for reading and printing.

== Two-field accuracy and copy number on held-out samples
Development followed a frozen-version protocol: each version was fixed before the next batch of samples was downloaded, and «n_ho» 1000 Genomes samples with HPRC release 2 assemblies were evaluated in four such rounds. T1K 1.0.10 [@t1k] was run on the same reads with the same IPD-IMGT/HLA release. For the eight classical genes with two copies, hla-wgs called «h2_ok» of «h2_n» alleles correctly at two fields («h2_pct») and T1K «h2_t1k» («h2_t1k_pct»). The copy number of DRB3, DRB4 and DRB5 (0, 1 or 2) was correct in «hc_hla» of «hc_n» cases for hla-wgs and in «hc_t1k» for T1K (@fig2). These rounds were run with the prototype of the algorithm; the current program agrees with the assemblies at two fields for «a23_ok2» of «a23_n» haplotypes of the 80 development samples.

#figure(image("fig2.svg", width: 100%), caption: [*Held-out comparison with T1K.* (A) Wrong two-field calls per gene in the 150 samples of rounds 1 to 3. (B) Share of correct DRB3/4/5 copy numbers by continental group. Truth: alleles of both assembled haplotypes.]) <fig2>

== Many individuals carry alleles that are not in the database
For 80 HPRC samples the sequence of every HLA gene was cut from both assembled haplotypes and compared with all database alleles of that gene at full genomic length. Of «a6_n» gene copies, «a6_known» were identical to a database allele and «a6_new» («pct_new») were not. New alleles were frequent in DRB1 («drb1_new» of «drb1_n»), in DRB3 and DRB4 and in the pseudogenes K and V, and rare in HLA-A and HLA-C («a_new» and «c_new» of 160). «one_diff» of them differ from the nearest database allele at a single position, «many_diff» at ten or more. For all of these a four-field name is necessarily wrong.

== Four-field statements
On the 80 development samples hla-wgs agreed with the assembly at two fields for «a23_ok2» and at three fields for «a23_ok3» of «a23_n» haplotypes. Of the «a6_known» known alleles, «a6_kok» were called exactly at four fields («pct_kok»). The program reported «a6_s4» alleles as certain at four fields; «a6_s4e» of them were identical to the assembled sequence («pct_s4»; @fig3, @tab1). Of the new-site positions it reported, «a6_det_true» of «a6_det» were confirmed by the assembly.

#figure(image("fig3.svg", width: 100%), caption: [*What is reported as certain, 80 development samples.* Each bar is the set of assembled gene copies of one gene. 4F: four fields. Dark green: certain at four fields and identical to the assembly. Red: certain but not identical. Light green: identical, but reported with fewer certain fields. Grey: new or different allele, reported with fewer certain fields.]) <fig3>

#figure(kind: table, caption: [80 HPRC development samples against the alleles of both assembled haplotypes. Two and three fields: the call shares these fields with a nearest database allele of its haplotype. Known exact: haplotypes identical to a database allele that were called with all four fields right. Certain 4F: reported as certain at four fields.], «table_dev») <tab1>

HLA-E is capped at three fields, because its fourth-field differences lie in repeats of the untranslated regions. For DRB3 and DRB4 four-field statements are rare: the new-allele examination finds unsupported positions or uncovered stretches in most cases. The copy numbers of DRB3, DRB4, DRB5, H and K agreed with the assemblies in all 80 samples.

== Fifty-nine genomes never used in development
The 59 HGSVC samples with 1000 Genomes short reads that are not among the HPRC samples were typed with the current program and compared with their own assemblies, built in the same way. hla-wgs agreed at two fields for «u23_ok2» and at three fields for «u23_ok3» of «u23_n» haplotypes. It reported «u6_s4» alleles as certain at four fields; «u6_s4e» were exact («pct_u4»; @fig4). Reported new-site positions were confirmed for «u6_det_true» of «u6_det».

#figure(image("fig4.svg", width: 100%), caption: [*What is reported as certain, 59 unseen HGSVC samples.* Colours as in @fig3.]) <fig4>

== Laboratory typing of 1,101 samples
All 1000 Genomes samples with laboratory types of HLA-A, -B, -C, -DRB1 and -DQB1 [@gourraud, @abirached] were typed from their 30× CRAM files (two samples have an empty index and were left out). Because the weak frequency prior of the program is counted from the same laboratory data, the samples were split in two halves and each half was typed with the prior of the other. Against the 2018 typing («l18_n_samples» samples) hla-wgs agreed with «l18_ok» of «l18_n» alleles at two fields («l18_pct»). T1K was run on «l18_t1k_samples» of these samples, where hla-wgs agreed with «l18_sub» and T1K with «l18_t1k» alleles. Against the older 2014 typing (all «l14_n_samples» samples, with more ambiguous entries) the agreement was «l14_ok» of «l14_n».

Each of the «l18_diff» differences to the 2018 typing was examined. In «l18_ard» the two alleles are identical in the exons that the laboratory typed (exons 2 and 3 of class I, exon 2 of class II), so the laboratory could not tell them apart; one is an old name that IPD-IMGT/HLA no longer contains, and two come from a parse error in the laboratory file. «l18_real» are real differences in the typed exons. Where an assembly is available for such a sample, it has so far agreed with hla-wgs. In the GIAB trios [@giabdata] (HG002 to HG004 at 60×, HG005 to HG007 at 100×) the child's alleles were consistent with Mendelian inheritance at two and three fields in every gene present («tr_a3» of «tr_an» and «tr_h3» of «tr_hn»).

== Lower sequencing depth
Leads shrink with the number of reads. When the 80 development samples were thinned to 15× and 10×, the calls themselves stayed almost unchanged, but many correct calls stopped at fewer certain fields than they deserved. Fields two and three are therefore certain from a lead of 4; a four-field statement still needs a lead of 6 at fields two and three and of 4 at field four. @tab2 shows the effect. The number of certain but wrong statements stayed the same except for one three-field call at 10×.

#figure(kind: table, caption: [Certain calls right / all at two and three fields, 80 development samples, by depth and by the lead required for fields two and three.], «table_depth») <tab2>

== Long reads
In its long-read mode hla-wgs takes PacBio HiFi reads of the MHC region, groups them by haplotype, builds a consensus per group by plurality vote, and names the nearest database allele by its exons first. For the 26 samples among the 80 with HPRC HiFi alignments, «lr_ok» of «lr_n» consensus sequences of all genes it found were identical to an assembled haplotype over their whole length, and «lr_hap» assembled gene copies were reproduced exactly. Of the calls it reported as exact database alleles, «lr_ex_ok» of «lr_ex_n» were in the truth set; of the calls with differences, the nearest allele was right at two fields in «lr_near_ok» of «lr_near_n». Most of the remaining differences are one to three bases in long single-base runs and short repeats, and in about half of them all reads agree with the hla-wgs consensus rather than with the assembly.

== Run time
On four cores and the extracted reads of a 30× genome (MHC region, ALT and HLA contigs, unmapped reads), hla-wgs needed «t_hla_wall» s wall time and «t_hla_cpu» s CPU time per sample, T1K «t_t1k_wall» s and «t_t1k_cpu» s (mean of «t_n» samples, same machine, run one after the other). Building the reference from the database release takes about four minutes and the repeat mask about one minute, once.

= Discussion
Three results stand out. First, a typer built around full-length alleles and an explicit copy-number model for the DRB paralogues makes fewer two-field errors than a current general-purpose tool and almost never misjudges whether DRB3, DRB4 or DRB5 is present. Second, a substantial share of gene copies carry alleles outside the database. For these, a four-field name is necessarily wrong, however accurate the typer. A program that always prints four fields therefore gives a wrong fourth field for about one in six gene copies, and gives no indication which ones. Third, the certainty statements held on genomes that played no part in development, and the laboratory comparison on 1,101 samples left only a handful of real disagreements.

hla-wgs separates the name from its certainty. The price is that it declines to state four fields for many alleles that are in the database and could have been named. In return, a four-field statement that is made was correct in «pct_s4» of cases in development and «pct_u4» on unseen genomes. For applications that rely on the non-coding sequence, the list of new sites is more informative than a name.

The remaining errors have identifiable causes. Homopolymer and repeat lengths are at the limit of short-read accuracy and are the main source of four-field errors. Variants at the far 3′ end of a gene cannot be assigned to a haplotype when no heterozygous position lies within reach of a read pair; the program marks such sites as belonging to either allele where it recognises the situation. With HiFi reads, the same repeats limit the agreement with the assemblies, which were built from the same reads.

= Limitations of the study
- Short-read WGS (2×100 to 2×150 bp) validated at 10× to 100×, and PacBio HiFi reads of the MHC region. Exome, amplicon, RNA and other long-read data are not supported.
- Assembly-based truth is itself not error-free, and its definition favours shorter database entries when two alleles are equally near. HiFi agreement shows that the program reads the data like the assembler, which used the same reads.
- The held-out comparison with T1K was run with the prototype of the algorithm; T1K was run with default settings and may perform better with tuned parameters. T1K also types KIR genes and RNA-seq data, which hla-wgs does not.
- Laboratory typing covers five genes at two fields. For DRA, E, F, G, H, K and V the only reference is the assemblies.
- The repeat mask assumes GRCh38. The program has not been validated for clinical use.

= Methods
== Reference
`hla-wgs build-db` reads the IPD-IMGT/HLA `hla.dat` file (release «db» in this report) and writes the allele sequences with exon coordinates and a panel of representative full-length allele sequences. Alleles without a full genomic sequence are extended with the sequence of their nearest full-length relative. `hla-wgs build-mask` counts 21-mers of the panel in the genome outside the MHC and masks panel stretches that occur there three times or more.

== Typing
Reads are mapped to the panel with the program's own aligner; a pair counts for a gene only if both mates align or the partner can be rescued next to it, and reads anchored only in masked sequence are dropped. Per gene, alleles are scored by a k-mer likelihood, and an expectation-maximisation step selects candidates. All reads of the gene are aligned to all candidates, and pairs of candidates are compared on the resulting edit-distance matrix; a refinement step re-examines the exons, for DRB1 with the reads that the panel gave to DRB3, DRB4 and DRB5. A weak prior from two-field allele frequencies of the 1000 Genomes laboratory typing separates otherwise equal candidates of five genes. DRB1, DRB3, DRB4 and DRB5 are modelled jointly by depth relative to the single-copy genes.

== Certainty
For fields two, three and four the lead is the log-likelihood difference to the best pair that differs at that field. Fields two and three are certain from a lead of 4. A four-field statement needs a lead of 6 at fields two and three and of 4 at field four (1 when the fourth field is 01, the first-described allele of its group). The reads are then realigned to the two called alleles. A position becomes a new site when enough reads (five, four or three by median depth) and a clear majority of the reads of that allele carry the same difference. A stretch of 30 bp or more without any read is reported as blind when such stretches sum to 300 bp or more. A new site in an exon reduces the resolution to the field that it changes.

== Long reads
HiFi reads of each gene are clustered into haplotype groups, extended, polished by plurality vote over the reads that span each position, and named by the nearest database allele, comparing exons first. Groups that name the same allele are polished jointly unless the joint sequence gains more than two unsure positions.

== Truth and evaluation
Assemblies of HPRC release 2 and HGSVC3 were downloaded, contigs with HLA sequence extracted, and each gene located by alignment of the database alleles. The truth for a haplotype is the nearest database allele at full genomic length together with the list of differences. Reads were taken from the 1000 Genomes 30× CRAM files. A four-field call counts as exact only if the assembled sequence is identical to the called allele. All evaluation is done by `hla-wgs-validate`, part of the repository; every number in this report is computed by it from the files under `validation/results`, and this report itself is produced by `hla-wgs-validate paper`.

= Resource availability
*Repository:* https://github.com/pjlabsdev/hla-wgs · *Archived version:* DOI «doi» · *Operating systems:* macOS, Linux, Windows · *Packages:* ready-to-run ZIP files per release, with a start file that asks only for the input file · *Licence:* MIT. The IPD-IMGT/HLA database is distributed by its owners under CC BY-ND and is not included; the start file downloads it and builds the reference on first use.

```
hla-wgs build-db hla.dat data/freq_1000g_2field.json hladb
hla-wgs build-mask hladb GRCh38.fa
hla-wgs hladb sample.cram out/sample 4 GRCh38.fa
hla-wgs long hladb sample.hifi.bam out/sample 4
```

= Acknowledgements
This work used the IPD-IMGT/HLA database, assemblies of the Human Pangenome Reference Consortium and of the Human Genome Structural Variation Consortium, sequencing data of the 1000 Genomes Project (New York Genome Center) and of the Genome in a Bottle Consortium, and the published HLA typing of 1000 Genomes samples.

= Declaration of interests
None declared.

= References
«refs»
