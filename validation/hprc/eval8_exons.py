#!/usr/bin/env python3
"""Exon-level check for calls certain to three or more fields: every exon of the called allele
must occur unchanged in the assembled haplotype it is assigned to (best of the two assignments)."""
import gzip, json, sys, os, glob, collections, itertools
D = sys.argv[1]
GENES = sys.argv[2].split(',') if len(sys.argv) > 2 else ["MICA", "MICB", "TAP1", "TAP2", "DMA", "DMB", "DOA", "DOB"]
db = json.load(gzip.open('db/hladb.json.gz'))
ex = {}
for g in db['genes']:
    if g['gene'] not in GENES:
        continue
    for a in g['alleles']:
        ex[a['name']] = [e[1] for e in a.get('exons', []) if len(e[1]) >= 20]
rc = lambda s: s[::-1].translate(str.maketrans('ACGT', 'TGCA'))
def load(fa):
    return ''.join(l.strip().upper() if not l.startswith('>') else 'N' for l in open(fa))
ST = collections.defaultdict(collections.Counter)
for s in open('newg/dev80.txt').read().split():
    haps = []
    for h in ('h1', 'h2', 'mat', 'pat'):
        f = f'truth/{s}.{h}.mhc.fa'
        if os.path.exists(f):
            x = load(f)
            haps.append((x, rc(x)))
    if len(haps) != 2 or not os.path.exists(f'{D}/{s}.conf.tsv'):
        continue
    conf = collections.defaultdict(list)
    for l in open(f'{D}/{s}.conf.tsv').read().splitlines()[1:]:
        r = l.split('\t')
        if r[0] in GENES:
            conf[r[0]].append((r[2], int(r[4])))
    for g, c in conf.items():
        if len(c) == 1:
            c = c * 2
        if len(c) != 2:
            continue
        miss = lambda name, hp: sum(1 for e in ex.get(name, []) if e not in hp[0] and e not in hp[1])
        m = [[miss(c[i][0], haps[j]) for j in (0, 1)] for i in (0, 1)]
        pm = min(((0, 1), (1, 0)), key=lambda p: m[0][p[0]] + m[1][p[1]])
        for i in (0, 1):
            name, res = c[i]
            k = ST[g]
            k['n'] += 1
            ok = m[i][pm[i]] == 0 and len(ex.get(name, [])) > 0
            k['exon_exact'] += ok
            if res >= 3:
                k['res3'] += 1; k['res3_exon_exact'] += ok
                if not ok and len(sys.argv) > 3:
                    print(s, g, name, res, 'exons missing', m[i][pm[i]], 'of', len(ex.get(name, [])))
            else:
                k['low'] += 1; k['low_exon_exact'] += ok
cols = ['n', 'exon_exact', 'res3', 'res3_exon_exact', 'low', 'low_exon_exact']
print('gene', *cols, sep='\t')
tot = collections.Counter()
for g in GENES:
    tot.update(ST[g]); print(g, *[ST[g][c] for c in cols], sep='\t')
print('ALL', *[tot[c] for c in cols], sep='\t')
