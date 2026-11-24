//! hla-wgs-validate: the validation and helper programs of the repository (evaluation against
//! assembled haplotypes and laboratory types, truth tables, read simulation, the software report).
//! The typing program `hla-wgs` itself does not need it. Every command keeps the input files,
//! environment variables and output format of the script it replaces, so earlier result files stay
//! comparable.
#[path = "../../aln.rs"]
#[allow(dead_code)]
mod aln;
#[path = "../../util.rs"]
#[allow(dead_code)]
mod util;

mod common;
mod eval;
mod long;
mod misc;
mod paper;
mod pyrand;
mod sim;
mod truth;

const USAGE: &str = "hla-wgs-validate COMMAND ...

evaluation (run in the work folder with db/, truth/ and the result folders):
  eval23 DIR                      2- and 3-field agreement with the assembly truth (T4OUT)
  eval6 DIR [v]                   exact evaluation of the 18 genes against both haplotypes
  eval8 DIR [v]                   the same for the eight added genes (DMA ... TAP2)
  eval8-exons DIR [GENES] [v]     exon-level check of calls certain to three or more fields
  eval24 [2014|2018|any]          1000 Genomes laboratory types against hla-wgs and T1K (HLA_WORK; calls in HLA_OUT, default lab/out24)
  trio23 [DIR]                    consistency of the two GIAB trios
  long DIR SAMPLES.txt [v]        long-read results: consensus vs assembly, exact and nearest calls (HLA_LTAG)
  eval-res DIR                    certain calls right at two and three fields (depth study)
  sim-eval TRUTH.tsv ASSIGN.tsv   gene assignment of simulated read pairs
truth from assemblies:
  truth4 / truth8                 best database alleles per haplotype and gene (SAMPLES, TRUTHSRC, T4OUT, T8OUT)
  truth-exon x|y|z FA...          exon projection of the database alleles onto haplotype assemblies
  truth-table x|y|z               truth/truthX.tsv from the exon projections
  mhc-crop FA...                  MHC window of the MHC contigs (HLA-A to DPB1 +-300 kb) -> FA as .mhc.fa
  allele-vs-assembly S HAP GENE PREFIX...
  mhc-contigs OUT.fa < ASSEMBLY.fa
helpers:
  sim DBDIR OUTPREFIX [--per N] [--cov X] [--seed S] [--profile P] [--div D]
  thin-pairs IN_PREFIX FRAC OUT_PREFIX
  build-freq OUT.json EXCLUDE...
  paper [--fonts DIR] [--version V] [--date D] [--doi DOI]   software report (HTML) to stdout
";

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let r: Vec<&str> = a.iter().map(|s| s.as_str()).collect();
    match r.first().copied() {
        Some("eval23") if r.len() >= 2 => eval::eval23(r[1]),
        Some("eval6") if r.len() >= 2 => eval::eval6(r[1], r.len() > 2, false),
        Some("eval8") if r.len() >= 2 => eval::eval6(r[1], r.len() > 2, true),
        Some("eval8-exons") if r.len() >= 2 => eval::eval8_exons(r[1], r.get(2).copied(), r.len() > 3),
        Some("eval24") => eval::eval24(r.get(1).copied().unwrap_or("2014")),
        Some("trio23") => eval::trio23(r.get(1).copied().unwrap_or("giab23")),
        Some("long") if r.len() >= 3 => long::long(r[1], r[2], r.len() > 3),
        Some("eval-res") if r.len() >= 2 => long::eval_res(r[1]),
        Some("sim-eval") if r.len() >= 3 => eval::sim_eval(r[1], r[2]),
        Some("truth4") => truth::truth4(false),
        Some("truth8") => truth::truth4(true),
        Some("truth-exon") if r.len() >= 2 => truth::truth_exon(r[1], &r[2..]),
        Some("truth-table") if r.len() >= 2 => truth::truth_table(r[1]),
        Some("mhc-crop") => truth::mhc_crop(&r[1..]),
        Some("allele-vs-assembly") if r.len() >= 5 => truth::allele_vs_assembly(r[1], r[2], r[3], &r[4..]),
        Some("mhc-contigs") if r.len() >= 2 => misc::mhc_contigs(r[1]),
        Some("sim") if r.len() >= 3 => sim::sim(&r[1..]),
        Some("thin-pairs") if r.len() >= 4 => misc::thin_pairs(r[1], r[2], r[3]),
        Some("build-freq") if r.len() >= 2 => misc::build_freq(r[1], &r[2..]),
        Some("paper") => paper::main(&r[1..]),
        _ => {
            eprint!("{USAGE}");
            std::process::exit(2)
        }
    }
}
