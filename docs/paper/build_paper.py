#!/usr/bin/env python3
"""Build the software report (HTML with inline SVG figures) from the validation result files.

Every number and figure is read from the files under validation/results so that the text cannot
drift from the data. Usage:

    python3 docs/paper/build_paper.py [--fonts DIR] > docs/paper/hla-wgs-report.html

--fonts points to a directory with the IBM Plex Mono and Instrument Sans woff2 files (fontsource
layout); they are embedded so that the PDF does not depend on installed fonts.
"""
import argparse
import base64
import glob
import html
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
RES = os.path.join(ROOT, "validation", "results")


INK, SUB, LINE, PAPER = "#141413", "#56544C", "#D8D5CC", "#FAF9F5"
GOLD = "#A57A27"                       # brand mark only, never a chart colour
OK, BAD, WARN, BLUE, GREY = "#2A7350", "#B3261E", "#8F5B00", "#2F5D8A", "#9A978E"
MONO = "'IBM Plex Mono', ui-monospace, Menlo, monospace"
SANS = "'Instrument Sans', 'Helvetica Neue', Arial, sans-serif"

LOGO = """<svg xmlns="http://www.w3.org/2000/svg" height="26" viewBox="0 0 600 95" role="img" aria-label="PJ Labs">
<g><path d="M 60,18.5 L 70,23.1 L 80,29.4 L 90,36.5 L 100,44.2 L 110,51.8 L 120,59.2 L 130,65.8 L 140,71.4 L 150,75.6 L 160,78.2 L 170,79.1 L 180,78.2 L 190,75.6 L 200,71.4 L 210,65.8 L 220,59.2 L 230,51.8 L 240,44.2 L 250,36.5" fill="none" stroke="#A57A27" stroke-width="13" stroke-linecap="round" stroke-linejoin="round"/>
<path d="M 80,70.6 L 90,63.5 L 100,55.8 L 110,48.2 L 120,40.8 L 130,34.2 L 140,28.6 L 150,24.4 L 160,21.8 L 170,20.9 L 180,21.8 L 190,24.4 L 200,28.6 L 210,34.2 L 220,40.8 L 230,48.2 L 240,55.8 L 250,63.5 L 260,70.6 L 270,76.9 L 280,81.5" fill="none" stroke="#A57A27" stroke-width="13" stroke-linecap="round" stroke-linejoin="round" opacity="0.35"/>
<line x1="144" y1="40.92" x2="144" y2="59.08" stroke="#A57A27" stroke-width="5" stroke-linecap="round"/>
<line x1="170" y1="34.90" x2="170" y2="65.10" stroke="#A57A27" stroke-width="5" stroke-linecap="round"/>
<line x1="196" y1="40.92" x2="196" y2="59.08" stroke="#A57A27" stroke-width="5" stroke-linecap="round"/></g>
<text x="315" y="72" font-family="'IBM Plex Mono', ui-monospace, Menlo, monospace" font-weight="500" font-size="46" fill="#141413">PJ Labs</text></svg>"""



def esc(text):
    return html.escape(str(text), quote=True)


def text(x, y, s, size=8, anchor="start", fill=INK, mono=False, weight=400, italic=False, rotate=None):
    style = f"font-family:{MONO if mono else SANS};font-size:{size}px;font-weight:{weight};" + ("font-style:italic;" if italic else "")
    tr = f' transform="rotate({rotate} {x} {y})"' if rotate is not None else ""
    return f'<text x="{x:.1f}" y="{y:.1f}" text-anchor="{anchor}" fill="{fill}" style="{style}"{tr}>{esc(s)}</text>'


def svg(width, height, body):
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="100%" '
            f'role="img">{"".join(body)}</svg>')



def fonts_css(directory):
    if not directory:
        return ""
    css = []
    wanted = [("IBM Plex Mono", "ibm-plex-mono", ("400-normal", "500-normal", "400-italic")),
              ("Instrument Sans", "instrument-sans", ("400-normal", "500-normal", "400-italic", "500-italic"))]
    for family, slug, variants in wanted:
        for variant in variants:
            weight, style = variant.split("-")
            for subset in ("latin", "latin-ext"):
                hits = glob.glob(os.path.join(directory, "**", f"{slug}-{subset}-{variant}.woff2"), recursive=True)
                if not hits:
                    continue
                data = base64.b64encode(open(hits[0], "rb").read()).decode()
                rng = "U+0000-00FF,U+2013-2014,U+2018-201D,U+2022,U+2026,U+2032,U+2212,U+2264-2265" if subset == "latin" else "U+0100-024F,U+1E00-1EFF"
                css.append(f"@font-face{{font-family:'{family}';font-weight:{weight};font-style:{style};"
                           f"src:url(data:font/woff2;base64,{data}) format('woff2');unicode-range:{rng};}}")
    return "\n".join(css)


def pct(k, n):
    return f"{100 * k / n:.1f} %"



def number_citations(page):
    """Replace [@key, @key] by numbers in order of first appearance and build the reference list."""
    order = []

    def cite(match):
        out = []
        for key in re.findall(r"@([a-z0-9]+)", match.group(0)):
            if key not in REFS:
                raise KeyError(f"unknown reference {key}")
            if key not in order:
                order.append(key)
            n = order.index(key) + 1
            out.append(f'<a class="cite" href="#ref-{n}">{n}</a>')
        return "[" + ", ".join(out) + "]"

    page = re.sub(r"\[@[a-z0-9]+(?:, @[a-z0-9]+)*\]", cite, page)
    unused = [k for k in REFS if k not in order]
    if unused:
        raise ValueError(f"references never cited: {unused}")
    items = "".join(f'<li id="ref-{n}">{REFS[k]}</li>\n' for n, k in enumerate(order, 1))
    return page.replace("{REFS}", f'<ol class="refs">\n{items}</ol>')


def style(fonts):
    return f"""
{fonts_css(fonts)}
@page {{ size: A4; margin: 17mm 16mm 18mm 16mm; }}
* {{ box-sizing: border-box; }}
body {{ font-family: {SANS}; font-size: 9.2pt; line-height: 1.42; color: {INK}; margin: 0; }}
.mast {{ display: flex; justify-content: space-between; align-items: center; border-bottom: 1.2px solid {INK}; padding-bottom: 6pt; margin-bottom: 12pt; }}
.mast .kind {{ font-family: {MONO}; font-size: 7.8pt; color: {SUB}; text-align: right; line-height: 1.35; }}
h1 {{ font-family: {MONO}; font-weight: 500; font-size: 16.5pt; line-height: 1.22; margin: 0 0 8pt; letter-spacing: -0.2pt; }}
h2 {{ font-family: {MONO}; font-weight: 500; font-size: 10.6pt; margin: 13pt 0 4pt; break-after: avoid; }}
h3 {{ font-family: {MONO}; font-weight: 500; font-size: 9.2pt; margin: 9pt 0 2pt; break-after: avoid; }}
p {{ margin: 0 0 5.5pt; text-align: justify; hyphens: auto; }}
.meta {{ font-size: 9.4pt; margin-bottom: 3pt; }} .meta strong {{ font-weight: 500; }}
.small {{ font-family: {MONO}; font-size: 7.6pt; color: {SUB}; line-height: 1.4; }}
.top {{ display: grid; grid-template-columns: 1fr 58mm; gap: 6mm; margin: 10pt 0 6pt; }}
.summary {{ border-top: 1px solid {LINE}; padding-top: 6pt; }}
.summary p {{ font-size: 9pt; }}
.highlights {{ background: #F3F1EA; border-radius: 5px; padding: 8pt 9pt; font-size: 8.3pt; line-height: 1.38; }}
.highlights h2 {{ margin: 0 0 4pt; font-size: 9pt; }} .highlights ul {{ margin: 0; padding-left: 11pt; }} .highlights li {{ margin-bottom: 3.5pt; }}
.ruo {{ border: 1px solid {WARN}; border-left-width: 3px; border-radius: 4px; padding: 5pt 8pt; font-size: 8.2pt; margin: 6pt 0 8pt; }}
.ruo strong {{ font-family: {MONO}; font-weight: 500; color: {WARN}; }}
.cols {{ column-count: 2; column-gap: 7mm; }}
figure {{ margin: 8pt 0 9pt; break-inside: avoid; column-span: all; }}
figure .frame {{ border: 1px solid {LINE}; border-radius: 5px; padding: 6pt; background: #FFFFFF; }}
figcaption, caption {{ font-size: 8pt; line-height: 1.35; color: {INK}; text-align: left; margin-top: 4pt; }}
figcaption strong, caption strong {{ font-family: {MONO}; font-weight: 500; }}
table {{ border-collapse: collapse; width: 100%; font-size: 7.9pt; margin: 4pt 0 9pt; break-inside: avoid; }}
.wide {{ column-span: all; }}
caption {{ caption-side: top; margin: 0 0 3pt; }}
td:first-child {{ white-space: nowrap; }}
th, td {{ border-bottom: 0.6px solid {LINE}; padding: 2.4pt 4pt; text-align: left; vertical-align: top; }}
th {{ font-family: {MONO}; font-weight: 500; font-size: 7.4pt; color: {SUB}; border-bottom: 1px solid {INK}; }}
code {{ font-family: {MONO}; font-size: 7.9pt; }}
pre {{ font-family: {MONO}; font-size: 7.4pt; background: #F3F1EA; padding: 5pt 6pt; border-radius: 4px; white-space: pre-wrap; }}
ul {{ margin: 0 0 5.5pt; padding-left: 12pt; }} li {{ margin-bottom: 2pt; }}
a.cite {{ color: inherit; text-decoration: none; }}
ol.refs {{ font-size: 7.6pt; padding-left: 14pt; line-height: 1.33; }} ol.refs li {{ margin-bottom: 1.5pt; }}
"""


REFS = {
    'imgt': 'Barker DJ, Maccari G, Georgiou X, et al. The IPD-IMGT/HLA Database. Nucleic Acids Res. 2023;51(D1):D1053–D1060.',
    'nomen': 'Marsh SGE, Albert ED, Bodmer WF, et al. Nomenclature for factors of the HLA system, 2010. Tissue Antigens. 2010;75(4):291–455.',
    'optitype': 'Szolek A, Schubert B, Mohr C, Sturm M, Feldhahn M, Kohlbacher O. OptiType: precision HLA typing from next-generation sequencing data. Bioinformatics. 2014;30(23):3310–3316.',
    'hlahd': 'Kawaguchi S, Higasa K, Shimizu M, Yamada R, Matsuda F. HLA-HD: an accurate HLA typing algorithm for next-generation sequencing data. Hum Mutat. 2017;38(7):788–797.',
    'xhla': 'Xie C, Yeo ZX, Wong M, et al. Fast and accurate HLA typing from short-read next-generation sequence data with xHLA. Proc Natl Acad Sci USA. 2017;114(30):8059–8064.',
    'kourami': 'Lee H, Kingsford C. Kourami: graph-guided assembly for novel human leukocyte antigen allele discovery. Genome Biol. 2018;19:16.',
    'hlala': 'Dilthey AT, Mentzer AJ, Carapito R, et al. HLA*LA: HLA typing from linearly projected graph alignments. Bioinformatics. 2019;35(21):4394–4396.',
    't1k': 'Song L, Bai G, Liu XS, Li B, Li H. Efficient and accurate KIR and HLA genotyping with massively parallel sequencing data. Genome Res. 2023;33(6):923–931.',
    'hprc': 'Liao WW, Asri M, Ebler J, et al. A draft human pangenome reference. Nature. 2023;617:312–324.',
    'kgp': 'Byrska-Bishop M, Evani US, Zhao X, et al. High-coverage whole-genome sequencing of the expanded 1000 Genomes Project cohort including 602 trios. Cell. 2022;185(18):3426–3440.',
    'gourraud': 'Gourraud PA, Khankhanian P, Cereb N, et al. HLA diversity in the 1000 Genomes dataset. PLoS One. 2014;9(7):e97282.',
    'abirached': 'Abi-Rached L, Gouret P, Yeh JH, et al. Immune diversity sheds light on missing variation in worldwide genetic diversity panels. PLoS One. 2018;13(10):e0206512.',
    'giabdata': 'Zook JM, Catoe D, McDaniel J, et al. Extensive sequencing of seven human genomes to characterize benchmark reference materials. Sci Data. 2016;3:160025.',
    'minimap2': 'Li H. Minimap2: pairwise alignment for nucleotide sequences. Bioinformatics. 2018;34(18):3094–3100.',
}

GENES = ["A", "B", "C", "DRB1", "DRB3", "DRB4", "DRB5", "DQA1", "DQB1", "DPA1", "DPB1", "DRA", "E", "F", "G", "H", "K", "V"]


def rows(path):
    with open(path) as handle:
        lines = [l.rstrip("\n").split("\t") for l in handle if l.strip()]
    return [dict(zip(lines[0], l)) for l in lines[1:] if len(l) == len(lines[0])]


def frac(s):
    a, b = s.split("/")
    return int(a), int(b)


class Data:
    def __init__(self):
        self.s = json.load(open(os.path.join(RES, "summary.json")))
        self.strata = rows(os.path.join(RES, "heldout_vs_t1k", "strata.tsv"))
        self.dev6 = {r["gene"]: {k: int(v) for k, v in r.items() if k != "gene"} for r in rows(os.path.join(RES, "dev80", "eval6.tsv"))}
        self.dev23 = {r["gene"]: {k: int(v) for k, v in r.items() if k != "gene"} for r in rows(os.path.join(RES, "dev80", "eval23.tsv"))}
        nd = []
        for l in open(os.path.join(RES, "dev80", "truth4.tsv")):
            r = l.rstrip("\n").split("\t")
            if len(r) >= 6 and r[2] != "_done" and r[3].isdigit() and int(r[3]) > 0:
                nd.append(int(r[3]))
        self.one, self.many = sum(1 for x in nd if x == 1), sum(1 for x in nd if x >= 10)
        u = os.path.join(RES, "check30", "eval6.tsv")
        self.un6 = self.un23 = None
        if os.path.exists(u):
            self.un6 = {r["gene"]: {k: int(v) for k, v in r.items() if k != "gene"} for r in rows(u)}
            self.un23 = {r["gene"]: {k: int(v) for k, v in r.items() if k != "gene"} for r in rows(os.path.join(RES, "check30", "eval23.tsv"))}

    def st(self, group, tool, col):
        r = next(r for r in self.strata if r["group"] == group and r["tool"] == tool)
        return frac(r[col])

    def total(self, tool, col):
        """150 samples of rounds 1 to 3 plus the 34 of round 4."""
        k, n = self.st("all", tool, col)
        k4, n4 = self.s["heldout_round4"][col][tool]
        return k + k4, n + n4


# ------------------------------------------------------------------ figure 1: workflow
def fig_workflow():
    w, h = 720, 196
    b = []

    def box(x, y, bw, bh, title, sub, fill="#FFFFFF", stroke=LINE):
        b.append(f'<rect x="{x}" y="{y}" width="{bw}" height="{bh}" rx="5" fill="{fill}" stroke="{stroke}"/>')
        b.append(text(x + bw / 2, y + 15, title, 8.5, "middle", mono=True, weight=500))
        for i, line in enumerate(sub):
            b.append(text(x + bw / 2, y + 27 + 10 * i, line, 7.2, "middle", SUB))

    def arrow(x1, y1, x2, y2):
        b.append(f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{SUB}" stroke-width="1" marker-end="url(#ah)"/>')

    b.append(f'<defs><marker id="ah" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="6" markerHeight="6" orient="auto">'
             f'<path d="M0,0 L8,4 L0,8 z" fill="{SUB}"/></marker></defs>')
    top = [("Input", ["FASTQ, BAM or CRAM", "MHC, ALT, unmapped"]),
           ("Panel mapping", ["one sequence per", "allele group, repeat mask"]),
           ("Candidates", ["k-mer likelihood,", "expectation maximisation"]),
           ("Best pair", ["edit matrix of all reads,", "margin per field"]),
           ("Copy number", ["DRB1/3/4/5 jointly,", "H and K"])]
    x = 8
    for i, (t, sub) in enumerate(top):
        box(x, 12, 128, 52, t, sub)
        if i:
            arrow(x - 16, 38, x - 1, 38)
        x += 144
    arrow(648, 64, 648, 115)
    low = [("Report", ["allele, safe call,", "resolution 1 to 4, sites"], "#F3F1EA"),
           ("Known allele?", ["called allele + sites", "= database allele: rename"], "#FFFFFF"),
           ("New-allele sites", ["consistent differences,", "stretches without reads"], "#FFFFFF"),
           ("Realignment", ["reads against the", "two called alleles"], "#FFFFFF")]
    x = 584 - 3 * 144
    for i, (t, sub, fill) in enumerate(low):
        box(x, 116, 128, 52, t, sub, fill=fill)
        if i:
            arrow(x - 1, 142, x - 15, 142)
        x += 144
    b.append(text(8, 186, "Research use only. A call must be confirmed by an accredited HLA laboratory before any medical use.", 7.4, fill=WARN))
    return svg(w, h, b)


# ------------------------------------------------------------------ figure 2: held-out comparison
def fig_heldout(d):
    w, h = 720, 232
    b = []
    genes = ["A", "B", "C", "DRB1", "DQA1", "DQB1", "DPA1", "DPB1"]
    b.append(text(8, 14, "A  Two-field errors per gene (150 held-out samples, 2 alleles each)", 8.5, mono=True, weight=500))
    x0, y0, ph = 40, 196, 150
    errs = {t: [frac(next(r for r in d.strata if r["group"] == "gene " + g and r["tool"] == t)["two_field"]) for g in genes] for t in ("hla-wgs", "T1K")}
    top = max(n - k for t in errs for k, n in errs[t]) or 1
    for v in range(0, top + 1, 2):
        y = y0 - ph * v / top
        b.append(f'<line x1="{x0}" y1="{y:.1f}" x2="{x0 + 304}" y2="{y:.1f}" stroke="{LINE}" stroke-width="0.6"/>')
        b.append(text(x0 - 5, y + 3, v, 7, "end", SUB, mono=True))
    for i, g in enumerate(genes):
        gx = x0 + 8 + i * 37
        for j, (t, col) in enumerate((("hla-wgs", BLUE), ("T1K", GREY))):
            k, n = errs[t][i]
            e = n - k
            bh = ph * e / top
            b.append(f'<rect x="{gx + j * 13}" y="{y0 - bh:.1f}" width="12" height="{max(bh, 0.6):.1f}" fill="{col}"/>')
            b.append(text(gx + j * 13 + 6, y0 - bh - 3, e, 6.8, "middle", INK, mono=True))
        b.append(text(gx + 12.5, y0 + 11, g, 7.2, "middle", INK, mono=True))
    b.append(text(380, 14, "B  DRB3/4/5 copy number correct (%)", 8.5, mono=True, weight=500))
    pops = ["EUR", "AFR", "EAS", "SAS", "AMR"]
    x1 = 420
    for v in (0, 25, 50, 75, 100):
        y = y0 - ph * v / 100
        b.append(f'<line x1="{x1}" y1="{y:.1f}" x2="{x1 + 290}" y2="{y:.1f}" stroke="{LINE}" stroke-width="0.6"/>')
        b.append(text(x1 - 5, y + 3, v, 7, "end", SUB, mono=True))
    for i, p in enumerate(pops):
        gx = x1 + 12 + i * 56
        for j, (t, col) in enumerate((("hla-wgs", BLUE), ("T1K", GREY))):
            k, n = d.st(p, t, "copy_number")
            bh = ph * k / n
            b.append(f'<rect x="{gx + j * 19}" y="{y0 - bh:.1f}" width="18" height="{bh:.1f}" fill="{col}"/>')
            b.append(text(gx + j * 19 + 9, y0 - bh - 3, f"{100 * k / n:.0f}", 6.8, "middle", INK, mono=True))
        b.append(text(gx + 18.5, y0 + 11, p, 7.2, "middle", INK, mono=True))
    for j, (t, col) in enumerate((("hla-wgs", BLUE), ("T1K 1.0.10", GREY))):
        b.append(f'<rect x="{x0 + j * 80}" y="{y0 + 22}" width="9" height="9" fill="{col}"/>')
        b.append(text(x0 + j * 80 + 13, y0 + 30, t, 7.4, fill=INK))
    return svg(w, h, b)


# ------------------------------------------------------------------ figure 3: four-field statements
def fig_four(e6, title):
    w = 720
    rh = 13
    h = 40 + rh * len(GENES) + 26
    b = [text(8, 14, title, 8.5, mono=True, weight=500)]
    x0, bw = 62, 520
    cols = [("certain at four fields, exact", OK), ("certain at four fields, not exact", BAD),
            ("not reported as certain, exact", "#A9C7B8"), ("not reported as certain, new or different allele", "#D9D6CC")]
    for i, g in enumerate(GENES):
        r = e6[g]
        y = 26 + i * rh
        n = r["n"]
        parts = [r["safe4_exact"], r["safe4"] - r["safe4_exact"], r["exact"] - r["safe4_exact"]]
        parts.append(n - sum(parts))
        b.append(text(x0 - 6, y + 9, g, 7.4, "end", INK, mono=True))
        x = x0
        for v, (_, col) in zip(parts, cols):
            ww = bw * v / n if n else 0
            if ww > 0:
                b.append(f'<rect x="{x:.1f}" y="{y}" width="{ww:.1f}" height="{rh - 3}" fill="{col}"/>')
            x += ww
        b.append(text(x0 + bw + 8, y + 9, f"{r['safe4_exact']}/{r['safe4']} of {n}", 7, fill=SUB, mono=True))
    y = 26 + len(GENES) * rh + 12
    x = x0
    for label, col in cols:
        b.append(f'<rect x="{x}" y="{y - 8}" width="9" height="9" fill="{col}"/>')
        b.append(text(x + 13, y, label, 7.2, fill=INK))
        x += 22 + 4.1 * len(label)
    return svg(w, h, b)


def gene_table(e6, e23):
    out = ""
    for g in GENES + ["ALL"]:
        r, t = e6[g], e23[g]
        name = "All genes" if g == "ALL" else f"HLA-{g}"
        out += (f"<tr><td>{name}</td><td>{r['n']}</td><td>{t['ok2']} / {t['n']}</td><td>{t['ok3']} / {t['n']}</td>"
                f"<td>{r['known_ok']} / {r['known']}</td><td>{r['new']}</td><td>{r['safe4']}</td><td>{r['safe4_exact']}</td></tr>")
    return out


def build(d, fonts, version, date, doi):
    s = d.s
    a6, a23 = d.dev6["ALL"], d.dev23["ALL"]
    h2 = {t: d.total(t, "two_field") for t in ("hla-wgs", "T1K")}
    hc = {t: d.total(t, "copy_number") for t in ("hla-wgs", "T1K")}
    n_ho = 150 + s["heldout_round4"]["samples"]
    lab, tr = s["lab"], s["trios"]
    ld = lab["differences"]
    ix = s["dev80_inexact_safe4"]
    tm = s["timing"]
    rel = tm.get("release_1.0.0")
    rel_txt = (f" The released Rust program needs {rel['wall_s']} s and {rel['peak_gb']} GB for the same kind of input." if rel else "")
    if d.un6:
        u6, u23 = d.un6["ALL"], d.un23["ALL"]
        n_un = s["check30"]["samples"]
        unseen_hl = (f"<li>On {n_un} further assembled genomes that played no part in developing the certainty rules, {u6['safe4_exact']} of {u6['safe4']} alleles reported as certain "
                     f"at four fields were exact ({pct(u6['safe4_exact'], u6['safe4'])}).</li>")
        unseen_sum = (f" On {n_un} further genomes that played no part in developing these rules the figure was {u6['safe4_exact']} of {u6['safe4']} "
                      f"({pct(u6['safe4_exact'], u6['safe4'])}).")
        unseen = f"""
<h3>Samples not used for the certainty rules</h3>
<p>The certainty rules were developed on the 80 samples above. To test them, {n_un} further HPRC release 2 samples were typed once with the released version 1.0.0. These samples had formed the first held-out round of the two-field typer and were available during its later development, but they were never looked at while the new-allele examination and the certainty rules were written. Two-field agreement was {u23['ok2']} of {u23['n']} and three-field agreement {u23['ok3']} of {u23['n']} haplotypes. Of {u6['known']} known alleles, {u6['known_ok']} were called exactly ({pct(u6['known_ok'], u6['known'])}); {u6['new']} of {u6['n']} haplotypes ({pct(u6['new'], u6['n'])}) carried a new allele. {u6['safe4']} alleles were reported as certain at four fields, and {u6['safe4_exact']} of them were exact ({pct(u6['safe4_exact'], u6['safe4'])}; Figure 4, Table 3). Of the {u6['safe4'] - u6['safe4_exact']} that were not, {s['check30']['inexact_safe4']['homopolymer_or_str_only']} differed only in a homopolymer or short tandem repeat length. The rate is slightly below that of the development samples, as expected for rules tuned on those.</p>
<figure><div class="frame">{fig_four(d.un6, f"Four-field statements per gene, {n_un} further samples")}</div>
<figcaption><strong>Figure 4. Samples not used for the certainty rules.</strong> As Figure 3.</figcaption></figure>
<table class="wide"><caption><strong>Table 3.</strong> Thirty samples not used for the certainty rules, columns as in Table 2.</caption>
<tr><th>Gene</th><th>Haplotypes</th><th>Two fields</th><th>Three fields</th><th>Known alleles exact</th><th>New alleles</th><th>Certain at four fields</th><th>of these exact</th></tr>
{gene_table(d.un6, d.un23)}</table>
"""
        unseen_lim = "<li>The 30 samples used to check the certainty rules were not untouched: they had been available during development of the two-field typer. A test on genomes never used at any stage is still missing.</li>"
    else:
        unseen_hl = unseen_sum = unseen = ""
        unseen_lim = "<li>The four-field certainty rules were developed on the same 80 samples on which they are reported. An independent test on unseen assembled genomes is not part of this version.</li>"
    title = "hla-wgs: HLA typing from whole-genome sequencing that states how many fields of each allele are certain"
    return f"""<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8">
<title>{title}</title>
<style>{style(fonts)}</style></head><body>

<div class="mast"><div>{LOGO}</div><div class="kind">Software report · preprint<br>Version {esc(version)} · {esc(date)}</div></div>

<h1>{title}</h1>
<div class="meta"><strong>Peter Jungwirth, MSc</strong> · PJ Labs, Austria</div>
<div class="small">Software: https://github.com/pjlabsdev/hla-wgs · DOI: <span id="doi">{esc(doi)}</span> · Licence: MIT</div>

<div class="top">
<div class="summary">
<p><strong>Summary.</strong> HLA alleles are named with up to four fields, and typing programs for whole-genome sequencing (WGS) usually print all four. Whether the fourth field is right is rarely known, because laboratory reference typing ends at two fields and because many individuals carry alleles that are absent from the IPD-IMGT/HLA database at full gene length. <i>hla-wgs</i> is a single Rust program that types 18 HLA genes from short-read WGS and reports, for every allele, the best database name, the number of fields that the reads support, and the positions at which the reads differ from that name. On {n_ho} held-out 1000 Genomes samples with assembly-based truth it made {h2['hla-wgs'][1] - h2['hla-wgs'][0]} two-field errors in {h2['hla-wgs'][1]:,} alleles (T1K: {h2['T1K'][1] - h2['T1K'][0]}) and determined the DRB3/4/5 copy number in {hc['hla-wgs'][0]} of {hc['hla-wgs'][1]} cases (T1K: {hc['T1K'][0]}). In 80 haplotype-resolved genomes, {pct(a6['new'], a6['n'])} of all gene copies carried a new allele; {a6['safe4_exact']:,} of {a6['safe4']:,} alleles that hla-wgs reported as certain at four fields were identical to the assembled gene sequence ({pct(a6['safe4_exact'], a6['safe4'])}).{unseen_sum}</p>
<p><strong>Availability.</strong> Source code, validation scripts and result tables are available under the MIT licence at https://github.com/pjlabsdev/hla-wgs. The reference is built locally from the IPD-IMGT/HLA release; no database sequences are distributed.</p>
</div>
<div class="highlights"><h2>Highlights</h2><ul>
<li>One program, no external aligner at run time; 18 genes including DRA, E, F, G, H, K and V from FASTQ, BAM or CRAM.</li>
<li>Every allele comes with the number of fields that the reads support and with the sites that contradict the database name.</li>
<li>{h2['hla-wgs'][0]:,} of {h2['hla-wgs'][1]:,} held-out alleles correct at two fields; DRB3/4/5 copy number correct in {hc['hla-wgs'][0]} of {hc['hla-wgs'][1]} cases.</li>
<li>{pct(a6['new'], a6['n'])} of gene copies in 80 assembled genomes are new alleles, which no four-field name can describe.</li>
<li>{pct(a6['safe4_exact'], a6['safe4'])} of the alleles reported as certain at four fields were exact.</li>
{unseen_hl}
<li>Research use only.</li>
</ul></div>
</div>

<div class="ruo"><strong>Research use only.</strong> hla-wgs is not a medical device and has not been clinically validated. Its output must not be used for transplantation, pharmacogenetic or any other medical decision without confirmation by an accredited HLA laboratory.</div>

<figure><div class="frame">{fig_workflow()}</div>
<figcaption><strong>Figure 1. Method.</strong> Reads are assigned to genes on a panel of representative full-length allele sequences, candidates are selected by a k-mer likelihood and compared on an edit-distance matrix. The reads are then realigned to the two called alleles; consistent differences and stretches without reads limit the number of fields that is reported as certain.</figcaption></figure>

<div class="cols">
<h2>Introduction</h2>
<p>The HLA genes in the major histocompatibility complex on chromosome 6 are the most polymorphic genes of the human genome. Their alleles determine transplant compatibility, are associated with autoimmune and infectious diseases, and predict severe drug hypersensitivity. Alleles are named with up to four colon-separated fields: the first two describe the protein, the third synonymous changes in the coding sequence, and the fourth changes in introns and untranslated regions [@nomen]. The IPD-IMGT/HLA database collects the known alleles [@imgt].</p>
<p>WGS covers the full length of every HLA gene, and several programs derive HLA types from short reads, among them OptiType [@optitype], HLA-HD [@hlahd], xHLA [@xhla], Kourami [@kourami], HLA*LA [@hlala] and T1K [@t1k]. They are generally evaluated at two fields against laboratory typing, because that is the resolution of the available reference data [@gourraud, @abirached]. Their output nevertheless often carries three or four fields. A user cannot tell whether the additional fields are supported by the data, are the first of several equally good database entries, or describe the nearest known relative of an allele that is not in the database.</p>
<p>Haplotype-resolved assemblies from the Human Pangenome Reference Consortium (HPRC) [@hprc] now provide the complete sequence of both copies of each HLA gene for samples that also have 30× short-read WGS from the 1000 Genomes Project [@kgp]. This allows two things that were not possible before: measuring four-field accuracy, and counting how often an individual carries an allele that the database does not contain.</p>
<p><i>hla-wgs</i> was written with that reference in mind. Its aim is not only a correct two-field type but an explicit statement, per allele, of how far the name can be trusted.</p>

<h2>Results</h2>
<h3>Design</h3>
<p>hla-wgs is one Rust program without run-time dependencies (Figure 1). It reads paired FASTQ files or a coordinate-sorted BAM or CRAM file on GRCh38 or GRCh37; from alignments it takes the MHC region, the ALT and HLA contigs and the unmapped reads. It types HLA-A, -B, -C, -DRB1, -DRB3, -DRB4, -DRB5, -DQA1, -DQB1, -DPA1, -DPB1, -DRA, -E, -F, -G and the pseudogenes HLA-H, -K and -V. For DRB3, DRB4, DRB5, H and K, which are absent from many haplotypes, it estimates the copy number.</p>
<p>For each allele the output gives the best database allele, a <i>safe call</i> cut to the number of fields that the reads support (<i>resolution</i>, 1 to 4), the margin to the best competing allele at fields two, three and four, and a list of <i>new sites</i>: positions at which the reads consistently differ from the called allele, or stretches that no read covers. If the called allele together with its new sites equals another database allele, the call is renamed to that allele. Output is written as two tables and as JSON.</p>

<h3>Two-field accuracy and copy number on held-out samples</h3>
<p>Development followed a frozen-version protocol: each version was fixed before the next batch of samples was downloaded, and {n_ho} 1000 Genomes samples with HPRC release 2 assemblies were evaluated in four such rounds. T1K 1.0.10 [@t1k] was run on the same reads with the same IPD-IMGT/HLA release. For the eight classical genes with two copies, hla-wgs called {h2['hla-wgs'][0]:,} of {h2['hla-wgs'][1]:,} alleles correctly at two fields ({pct(*h2['hla-wgs'])}) and T1K {h2['T1K'][0]:,} ({pct(*h2['T1K'])}). The copy number of DRB3, DRB4 and DRB5 (0, 1 or 2) was correct in {hc['hla-wgs'][0]} of {hc['hla-wgs'][1]} cases with hla-wgs and in {hc['T1K'][0]} with T1K, counted from the alleles it reports (Figure 2). The advantage was present in all five continental groups.</p>
<figure><div class="frame">{fig_heldout(d)}</div>
<figcaption><strong>Figure 2. Held-out comparison with T1K.</strong> (A) Number of wrong two-field calls per gene in the 150 samples of rounds 1 to 3. (B) Share of correct DRB3/4/5 copy numbers by continental group. Truth: alleles of both assembled haplotypes.</figcaption></figure>
<p>These rounds were run with the prototype of the algorithm (a Python driver around the Rust kernel). The released program is a port of that algorithm; on the 80 samples of the next section its two-field agreement was {a23['ok2']:,} of {a23['n']:,}.</p>

<h3>Many individuals carry alleles that are not in the database</h3>
<p>For 80 HPRC samples the sequence of every HLA gene was cut from both assembled haplotypes and compared with all database alleles of that gene at full genomic length. Of {a6['n']:,} gene copies, {a6['known']:,} were identical to a database allele and {a6['new']} ({pct(a6['new'], a6['n'])}) were not. New alleles were frequent in DRB1 ({d.dev6['DRB1']['new']} of {d.dev6['DRB1']['n']}), in DRB3 and DRB4, and in the pseudogenes K and V, and rare in the class I genes A and C ({d.dev6['A']['new']} and {d.dev6['C']['new']} of 160). {d.one} of them differ from the nearest database allele at a single position and {d.many} at ten or more; the latter are concentrated in K and V. For such a gene copy the first fields of the nearest allele usually remain meaningful, while a correct fourth field does not exist.</p>

<h3>Four-field statements</h3>
<p>On these 80 samples, hla-wgs agreed with the assembly at two fields for {a23['ok2']:,} and at three fields for {a23['ok3']:,} of {a23['n']:,} haplotypes. Of the {a6['known']:,} known alleles, {a6['known_ok']:,} were called exactly at four fields ({pct(a6['known_ok'], a6['known'])}). The program reported {a6['safe4']:,} alleles as certain at four fields; {a6['safe4_exact']:,} of them were identical to the assembled sequence ({pct(a6['safe4_exact'], a6['safe4'])}; Figure 3, Table 2). Of the {a6['safe4'] - a6['safe4_exact']} that were not, {ix['homopolymer_or_str_only']} differed only in the length of a homopolymer or short tandem repeat, {ix['b_3prime_phase']} were phase swaps near the 3′ end of HLA-B, where the distinguishing variant lies further from the next heterozygous position than a read pair reaches, and {ix['other']} were single missed variants in other genes. Of {a6['site_snv']:,} single-nucleotide differences between new alleles and their nearest database allele, {a6['site_snv_found']:,} were reported as new sites ({pct(a6['site_snv_found'], a6['site_snv'])}), and {a6['site_det_true']:,} of {a6['site_det']:,} reported sites were true ({pct(a6['site_det_true'], a6['site_det'])}).</p>
<figure><div class="frame">{fig_four(d.dev6, "Four-field statements per gene, 80 HPRC samples")}</div>
<figcaption><strong>Figure 3. What is reported as certain.</strong> Each bar is the set of assembled gene copies of one gene. Dark green: reported as certain at four fields and identical to the assembly. Red: reported as certain but not identical. Light green: identical, but reported with fewer certain fields. Grey: new or different allele, reported with fewer certain fields. Numbers: exact / certain of all copies.</figcaption></figure>
<table class="wide"><caption><strong>Table 2.</strong> 80 HPRC samples against the alleles of both assembled haplotypes. Two and three fields: the call shares these fields with a nearest database allele of its haplotype. Known alleles exact: haplotypes identical to a database allele that were called with all four fields right.</caption>
<tr><th>Gene</th><th>Haplotypes</th><th>Two fields</th><th>Three fields</th><th>Known alleles exact</th><th>New alleles</th><th>Certain at four fields</th><th>of these exact</th></tr>
{gene_table(d.dev6, d.dev23)}</table>
<p>For HLA-E the resolution is capped at three fields, because its fourth-field differences lie in repeats of the untranslated regions and were miscalled with large margins during development. For DRB3 and DRB4 no four-field statement was made in these samples; the new-allele examination found unsupported positions or uncovered stretches in every case. The copy numbers of DRB3, DRB4, DRB5, H and K agreed with the assemblies in all 80 samples.</p>
{unseen}
<h3>Laboratory typing and family trios</h3>
<p>Forty-five further 1000 Genomes samples with laboratory typing of HLA-A, -B, -C, -DRB1 and -DQB1 [@gourraud, @abirached] were typed with a frequency prior that excluded them. {lab['agree']} of {lab['total']} alleles agreed at two fields. Of the {lab['total'] - lab['agree']} differences, {ld['method_resolution']} involve allele pairs that the laboratory method does not separate because they differ outside the typed exons, {ld['probable_lab_error']} are B*27:05 calls against a laboratory B*27:03 in European samples, where B*27:05 is the common allele and the laboratory result is the probable error, and {ld['open']} remain unexplained.</p>
<p>In the Genome in a Bottle trios [@giabdata] (Ashkenazi: HG002, HG003, HG004 at 60×; Han Chinese: HG005, HG006, HG007 at 100×) the child's alleles were consistent with Mendelian inheritance at two and three fields in every gene that is present ({tr['ashkenazi']['consistent_3']} of {tr['ashkenazi']['genes_present']} and {tr['han']['consistent_3']} of {tr['han']['genes_present']}). At four fields {tr['ashkenazi']['genes_present'] - tr['ashkenazi']['consistent_4']} and {tr['han']['genes_present'] - tr['han']['consistent_4']} genes were inconsistent; in each of them at least one call was not reported as certain. All genes in which every call of the three family members was certain at four fields were consistent ({tr['ashkenazi']['all_safe_consistent']} of {tr['ashkenazi']['all_safe_genes']} and {tr['han']['all_safe_consistent']} of {tr['han']['all_safe_genes']}).</p>

<h3>Run time</h3>
<p>On four cores and the MHC and unmapped reads of a 30× genome, the prototype needed {tm['prototype_v16']['wall_s']} s and {tm['prototype_v16']['peak_gb']} GB, T1K {tm['T1K_1.0.10']['wall_s']} s and {tm['T1K_1.0.10']['peak_gb']} GB.{rel_txt} Building the reference from the database release takes about four minutes and the repeat mask about one minute, once.</p>

<h2>Discussion</h2>
<p>Two results stand out. First, a typer built around full-length alleles and an explicit copy-number model for the DRB paralogues makes fewer two-field errors than a current general-purpose tool and almost never misjudges whether DRB3, DRB4 or DRB5 is present. Second, a substantial share of gene copies carry alleles outside the database. For these, a four-field name is necessarily wrong, however accurate the typer. A program that always prints four fields therefore gives a wrong fourth field for about one in six gene copies, and gives no indication which ones.</p>
<p>hla-wgs addresses this by separating the name from its certainty. The price is that it declines to state four fields for {pct(a6['known'] - a6['safe4_exact'], a6['known'])} of the alleles that are in fact in the database and could have been named. In return, a four-field statement that is made was correct in {pct(a6['safe4_exact'], a6['safe4'])} of cases. For applications that rely on the non-coding sequence, such as expression studies or fine-mapping, the list of new sites is more informative than a name.</p>
<p>The remaining errors have identifiable causes. Homopolymer and repeat lengths are at the limit of short-read accuracy. Variants at the far 3′ end of a gene cannot be assigned to a haplotype when no heterozygous position lies within reach of a read pair; this is a property of the data, and the program marks such sites as belonging to either allele where it recognises the situation.</p>

<h2>Limitations of the study</h2>
<ul>
<li>Short-read WGS only (2×100 to 2×150 bp, validated at 30× and above). Exome, amplicon, RNA and long-read data are not supported.</li>
<li>Assembly-based truth exists for individuals of the HPRC. It is itself not error-free, and its definition favours shorter database entries when two alleles are equally near.</li>
{unseen_lim}
<li>The held-out comparison with T1K was run with the prototype of the algorithm; T1K was run with default settings and may perform better with tuned parameters. T1K also types KIR genes and RNA-seq data, which hla-wgs does not.</li>
<li>Laboratory reference typing covers five genes at two fields. For DRA, E, F, G, H, K and V the only reference is the assemblies.</li>
<li>Four-field statements reported as certain were not always exact; homopolymer lengths and the 3′ end of HLA-B are known weak points.</li>
<li>The repeat mask assumes GRCh38. The program has not been validated for clinical use.</li>
</ul>

<h2>Methods</h2>
<h3>Reference</h3>
<p><code>hla-wgs build-db</code> reads the IPD-IMGT/HLA <code>hla.dat</code> file (release 3.66 in this report) and writes the allele sequences with exon coordinates, and a panel of representative full-length allele sequences. Alleles without a full genomic sequence are extended with the sequence of their nearest full-length relative. <code>hla-wgs build-mask</code> counts 21-mers of the panel in the genome outside the MHC and masks panel stretches that occur there three times or more.</p>
<h3>Typing</h3>
<p>Reads are mapped to the panel with the minimap2 library [@minimap2]; reads anchored only in masked sequence are dropped. Per gene, alleles are scored by a k-mer likelihood, and an expectation-maximisation step selects candidates. All reads of the gene are aligned to all candidates, and pairs of candidates are compared on the resulting edit-distance matrix; a refinement step re-examines the exons. A weak prior from two-field allele frequencies of the 1000 Genomes laboratory typing separates otherwise equal candidates of five genes. DRB1, DRB3, DRB4 and DRB5 are modelled jointly by depth relative to the two-copy genes, as are H and K.</p>
<h3>Certainty</h3>
<p>For fields two, three and four the margin is the log-likelihood difference to the best pair that differs at that field. A field is supported when its margin is at least 6 (fields two and three) or 4 (field four). The reads are then realigned to the two called alleles. A position becomes a new site when at least five reads and a clear majority of the reads assigned to that allele carry the same difference; long insertions and deletions count as one event. A stretch of 30 bp or more without any read is reported as blind when such stretches sum to 300 bp or more. A new site in an exon reduces the resolution to one field, any other site to three.</p>
<h3>Truth and evaluation</h3>
<p>Assemblies of HPRC release 2 were downloaded, contigs with HLA sequence extracted, and each gene located by alignment of the database alleles. The truth for a haplotype is the nearest database allele at full genomic length together with the list of differences. Reads were taken from the 1000 Genomes 30× CRAM files (MHC region, ALT and HLA contigs, unmapped reads). A four-field call counts as exact only if the assembled sequence is identical to the called allele. All scripts are in the repository under <code>validation/</code>.</p>

<h2>Resource availability</h2>
<p><strong>Repository:</strong> https://github.com/pjlabsdev/hla-wgs · <strong>Archived version:</strong> DOI {esc(doi)} · <strong>Operating systems:</strong> Linux, macOS · <strong>Requirements:</strong> Rust 1.85 or newer, a C compiler; IPD-IMGT/HLA <code>hla.dat</code> and a GRCh38 FASTA to build the reference · <strong>Licence:</strong> MIT. The IPD-IMGT/HLA database is distributed by its owners under CC BY-ND and is not included.</p>
<pre>hla-wgs build-db hla.dat data/freq_1000g_2field.json hladb
hla-wgs build-mask hladb GRCh38.fa
hla-wgs hladb sample.cram out/sample 4 GRCh38.fa</pre>

<h2>Acknowledgements</h2>
<p>This work used the IPD-IMGT/HLA database, assemblies of the Human Pangenome Reference Consortium, sequencing data of the 1000 Genomes Project (New York Genome Center) and of the Genome in a Bottle Consortium, and the published HLA typing of 1000 Genomes samples.</p>
<h2>Declaration of interests</h2>
<p>None declared.</p>

<h2>References</h2>
{{REFS}}
</div>
</body></html>
"""


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--fonts")
    p.add_argument("--version", default="1.0.0")
    p.add_argument("--date", default="19 November 2026")
    p.add_argument("--doi", default="10.5281/zenodo.24367470")
    args = p.parse_args(argv)
    sys.stdout.write(number_citations(build(Data(), args.fonts, args.version, args.date, args.doi)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
