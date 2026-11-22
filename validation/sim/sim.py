# simulate short read pairs from IPD-IMGT/HLA genomic alleles with known gene of origin
# usage: python3 sim.py DBDIR OUTPREFIX [--per N] [--cov X] [--seed S] [--profile P] [--div D]
#   --profile  illumina: 0.3 % substitutions, few indels, flat quality (default)
#              lowq:     error rate rising from 0.2 % to 4 % along the read, qualities to match
#              mgi:      DNB-like: substitutions rising towards the 3' end, almost no indels
#              hard:     lowq errors plus 2 % of pairs chimeric (mates from two genes)
#   --div      extra divergence of each source allele (0.01 = one difference per 100 bases,
#              a third of them small indels), as a new allele far from the panel would be
import argparse, gzip, json, math, random

ap = argparse.ArgumentParser()
ap.add_argument("db")
ap.add_argument("out")
ap.add_argument("--per", type=int, default=4, help="alleles per gene, half of them not in the panel")
ap.add_argument("--cov", type=float, default=5)
ap.add_argument("--seed", type=int, default=7)
ap.add_argument("--profile", default="illumina", choices=["illumina", "lowq", "mgi", "hard"])
ap.add_argument("--div", type=float, default=0.0)
a_ = ap.parse_args()
rnd = random.Random(a_.seed)
panel = set(l[1:].split()[0] for l in open(f"{a_.db}/panel.fa") if l.startswith(">"))
j = json.load(gzip.open(f"{a_.db}/hladb.json.gz"))
comp = str.maketrans("ACGTN", "TGCAN")
rc = lambda s: s.translate(comp)[::-1]
L = 150


def rate(i):
    """substitution, insertion, deletion probability at read position i"""
    p = a_.profile
    if p == "illumina":
        return 0.003, 0.0001, 0.0001
    if p in ("lowq", "hard"):
        e = 0.002 + 0.038 * (i / (L - 1)) ** 3
        return e, e / 30, e / 30
    # mgi
    e = 0.002 + 0.012 * (i / (L - 1)) ** 2
    return e, e / 200, e / 200


def qual(i):
    e = sum(rate(i))
    return chr(33 + max(2, min(41, int(-10 * math.log10(e)))))


def err(s):
    o, q = [], []
    for i, c in enumerate(s):
        ps, pi, pd = rate(len(o))
        x = rnd.random()
        if x < ps:
            o.append(rnd.choice([b for b in "ACGT" if b != c]))
        elif x < ps + pd:
            continue
        elif x < ps + pd + pi:
            o.append(c)
            o.append(rnd.choice("ACGT"))
        else:
            o.append(c)
    s = ("".join(o) + "A" * L)[:L]
    return s, "".join(qual(i) for i in range(L))


def diverge(s):
    if a_.div <= 0:
        return s
    o = []
    i = 0
    while i < len(s):
        if rnd.random() < a_.div:
            k = rnd.random()
            if k < 2 / 3:
                o.append(rnd.choice([b for b in "ACGT" if b != s[i]]))
                i += 1
            elif k < 5 / 6:
                o.append(s[i])
                o.append("".join(rnd.choice("ACGT") for _ in range(rnd.randint(1, 6))))
                i += 1
            else:
                i += rnd.randint(1, 6)
        else:
            o.append(s[i])
            i += 1
    return "".join(o)


f1, f2 = gzip.open(a_.out + ".R1.fq.gz", "wt"), gzip.open(a_.out + ".R2.fq.gz", "wt")
meta = open(a_.out + ".truth.tsv", "w")
src = []
for g in j["genes"]:
    al = [a for a in g["alleles"] if a["genomic"] and not a["extended"] and not a["imputed"] and not a["partial"] and len(a["genomic"]) > 1200]
    if not al:
        continue
    inp = [a for a in al if a["name"] in panel]
    outp = [a for a in al if a["name"] not in panel]
    pick = rnd.sample(inp, min(a_.per // 2, len(inp))) + rnd.sample(outp, min(a_.per - a_.per // 2, len(outp)))
    for a in pick:
        src.append((g["gene"], a["name"], int(a["name"] in panel), diverge(a["genomic"].upper())))
n = 0
for gene, name_, inp, s in src:
    npairs = int(a_.cov * len(s) / 300)
    for _ in range(npairs):
        fl = max(160, int(rnd.gauss(400, 80)))
        if fl >= len(s):
            continue
        st = rnd.randrange(0, len(s) - fl + 1)
        fr = s[st:st + fl]
        r1, r2 = fr[:L], rc(fr)[:L]
        g2 = gene
        if a_.profile == "hard" and rnd.random() < 0.02:
            # chimeric pair: the second mate from another gene; truth stays with the first mate's gene
            o = rnd.choice(src)
            if len(o[3]) > L:
                p2 = rnd.randrange(0, len(o[3]) - L + 1)
                r2 = rc(o[3][p2:p2 + L])
                g2 = o[0]
        if rnd.random() < 0.5:
            r1, r2 = r2, r1
        name = f"s{n}"
        meta.write(f"{name}\t{gene}\t{name_}\t{inp}\t{st}\t{g2}\n")
        for f, r in ((f1, r1), (f2, r2)):
            seq, q = err(r)
            f.write(f"@{name}\n{seq}\n+\n{q}\n")
        n += 1
print(n, "pairs")
f1.close()
f2.close()
meta.close()
