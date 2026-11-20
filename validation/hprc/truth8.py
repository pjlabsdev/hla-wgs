# full-gene truth: per haplotype and gene, the database alleles of the right 2-field groups with the
# fewest edits against the assembly (ties kept as a set). Alleles are compared over their own span.
import gzip,json,subprocess,collections,os,re,sys,glob
db=json.load(gzip.open('db/hladb.json.gz'))
G=collections.defaultdict(list)
for g in db['genes']:
    for a in g['alleles']:
        if a.get('imputed'): continue
        s=a.get('orig') if a.get('extended') and a.get('orig') else a['genomic']
        if len(s)>=200: G[g['gene']].append((a['name'],s))
two=lambda n:':'.join(re.sub(r'[A-Z]$','',n.split('*')[1]).split(':')[:2])
G8=["MICA","MICB","TAP1","TAP2","DMA","DMB","DOA","DOB"]
samples=open(os.environ.get('SAMPLES','newg/both.txt')).read().split()
done=set()
T4=os.environ.get('T8OUT','truth/truth8.tsv')
if os.path.exists(T4):
    for l in open(T4): done.add(tuple(l.split('\t')[:2]))
out=open(T4,'a')
for s in samples:
    for fa in sorted(glob.glob(f'truth/{s}.*.mhc.fa')):
        h=fa.split('.')[1]
        if (s,h) in done: continue
        q=f'pr/_t8_{os.getpid()}.fa'
        with open(q,'w') as o:
            for g in G8:
                for n,sq in G.get(g,[]):
                    o.write(f'>{n}\n{sq}\n')
        res=subprocess.run(['env/bin/minimap2','-c','-x','asm5','-t2','--secondary=no',fa,q],capture_output=True,text=True).stdout
        best=collections.defaultdict(dict)
        for l in res.splitlines():
            r=l.split('\t'); n=r[0]; ql=int(r[1]); qs,qe=int(r[2]),int(r[3])
            nm=int([x for x in r if x.startswith('NM:i:')][0][5:])
            d=nm+qs+(ql-qe)          # unaligned ends count as edits
            g=n.split('*')[0]
            if n not in best[g] or d<best[g][n][0]: best[g][n]=(d,ql,r[5],int(r[7]))
        for g in G8:
            if not best[g]: out.write(f"{s}\t{h}\t{g}\t-\t-\t-\n"); continue
            m=min(v[0] for v in best[g].values())
            names=sorted(n for n,v in best[g].items() if v[0]==m)
            # longest allele among the best defines the reference name
            names.sort(key=lambda n:-best[g][n][1])
            out.write(f"{s}\t{h}\t{g}\t{m}\t{','.join(names)}\t{len(best[g])}\n")
        out.write(f"{s}\t{h}\t_done\t\t\t\n"); out.flush()
        os.remove(q)
