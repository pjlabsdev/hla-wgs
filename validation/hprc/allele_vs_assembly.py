#!/usr/bin/env python3
"""edits of named alleles against an assembled haplotype: allele_vs_assembly.py SAMPLE HAP GENE NAMEPREFIX..."""
import gzip, json, subprocess, sys
s, h, g, pre = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4:]
db = json.load(gzip.open('db/hladb.json.gz'))
q = '_alleles.fa'
with open(q, 'w') as o:
    for G in db['genes']:
        if G['gene'] != g: continue
        for a in G['alleles']:
            if a.get('imputed'): continue
            if any(a['name'].startswith(p) for p in pre):
                sq = a.get('orig') if a.get('extended') and a.get('orig') else a['genomic']
                o.write(f">{a['name']}\n{sq}\n")
res = subprocess.run(['minimap2', '-c', '-x', 'asm5', '-t2', '--secondary=no', f'truth/{s}.{h}.mhc.fa', q], capture_output=True, text=True).stdout
rows = []
for l in res.splitlines():
    r = l.split('\t'); ql = int(r[1]); qs, qe = int(r[2]), int(r[3])
    nm = int([x for x in r if x.startswith('NM:i:')][0][5:])
    rows.append((nm + qs + ql - qe, r[0], ql, r[5], r[7]))
for r in sorted(rows)[:6]: print(s, h, *r)
