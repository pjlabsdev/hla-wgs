# exact evaluation: every called allele is aligned to both assembly haplotypes.
# d_call = edits of the called allele against its haplotype, d_min = best any database allele reaches (truth4).
import gzip,json,subprocess,collections,os,re,sys,itertools
D=sys.argv[1]; V=len(sys.argv)>2
db=json.load(gzip.open('db/hladb.json.gz'))
seq={}
for g in db['genes']:
    for a in g['alleles']:
        seq[a['name']]=a.get('orig') if a.get('extended') and a.get('orig') else a['genomic']
rc=lambda s:s[::-1].translate(str.maketrans('ACGTacgt','TGCAtgca'))
dmin=collections.defaultdict(dict)
for l in open('truth/truth4.tsv'):
    r=l.rstrip('\n').split('\t')
    if r[2]=='_done' or r[3]=='-': continue
    dmin[(r[0],r[2])][r[1]]=(int(r[3]),r[4].split(','))
def diffs(r,q):
    ql=len(q); qs,qe=int(r[2]),int(r[3]); p=0; out=[]
    Q=q[qs:qe] if r[4]=='+' else rc(q[qs:qe])
    cs=[x for x in r if x.startswith('cs:Z:')][0][5:]
    for op,v in re.findall(r'([:*+\-])([0-9a-z]+)',cs):
        if op==':': p+=int(v); continue
        ctxt=Q[max(0,p-8):p+8+len(v)].upper()
        hp=max((len(x.group()) for x in re.finditer(r'A+|C+|G+|T+',ctxt)),default=1)
        strx=bool(re.search(r'(..)\1\1|(...)\2\2|(....)\3\3',ctxt))
        pa=qs+p+1 if r[4]=='+' else qe-p
        if op=='*': kind='snv'; p+=1
        elif op=='+': kind='indel'; p+=len(v)
        else: kind='indel'
        out.append((pa,kind+('_hp' if kind=='indel' and hp>=5 else '_str' if kind=='indel' and strx else '')))
    return qs+(ql-qe),out
ST=collections.defaultdict(collections.Counter); EX=[]
for s in sorted(set(k[0] for k in dmin)):
    if not os.path.exists(f"{D}/{s}.conf.tsv"): continue
    conf=collections.defaultdict(list)
    for l in open(f"{D}/{s}.conf.tsv").read().splitlines()[1:]:
        r=l.split("\t"); det=set()
        if len(r)>12 and r[12]!='-':
            det={int(x.split(':')[0]) for x in ('' if r[12].startswith('known:') else r[12]).split(',') if x and 'blind' not in x and 'unphased' not in x}
        conf[r[0]].append((r[2],int(r[4]),det,int(r[11]) if len(r)>11 and r[11]!='-' else 0,r[12] if len(r)>12 else '-'))
    names=sorted({x[0] for v in conf.values() for x in v})
    hits={}
    for h in ('h1','h2','mat','pat'):
        fa=f'truth/{s}.{h}.mhc.fa'
        if not os.path.exists(fa): continue
        q=f'pr/_e6_{os.getpid()}.fa'
        open(q,'w').write(''.join(f'>{n}\n{seq[n]}\n' for n in names))
        res=subprocess.run(['env/bin/minimap2','-c','--cs','-x','asm5','-t2','--secondary=no',fa,q],capture_output=True,text=True).stdout
        for l in res.splitlines():
            r=l.split('\t'); nm=int([x for x in r if x.startswith('NM:i:')][0][5:])
            clip,dl=diffs(r,seq[r[0]]); d=nm+clip
            if (h,r[0]) not in hits or d<hits[(h,r[0])][0]: hits[(h,r[0])]=(d,dl)
        os.remove(q)
    hs=sorted({h for h,_ in hits})
    for g,c in conf.items():
        T=dmin.get((s,g),{})
        if len(c)==1 and len(T)==2: c=c*2
        hh=[h for h in hs if h in T]
        if len(c)!=len(hh) or not c: ST[g]['skip']+=1; continue
        best=min(itertools.permutations(range(len(c))),key=lambda pm:(sum(hits.get((h,c[i][0]),(999,[]))[0] for i,h in zip(pm,hh)),-sum(1 for i,h in zip(pm,hh) for p in c[i][2] if any(abs(p-q)<=2 for q,_ in hits.get((h,c[i][0]),(999,[]))[1])),sum(1 for i,h in zip(pm,hh) if c[i][1]>=4 and hits.get((h,c[i][0]),(999,[]))[0]>0)))
        for i,h in zip(best,hh):
            name,res,det,nn,detail=c[i]; dc,dl=hits.get((h,name),(999,[])); dm=T[h][0]; k=ST[g]
            k['n']+=1
            if os.environ.get('DUMP'): open(os.environ['DUMP'],'a').write('\t'.join(map(str,(s,g,name,res,dc,dm,nn,detail,';'.join(f'{p}:{t}' for p,t in dl[:8]))))+'\n')
            exact=dc==0; nearest=dc<=dm
            k['exact']+=exact; k['nearest']+=nearest
            if dm==0: k['known']+=1; k['known_ok']+=exact
            else: k['new']+=1; k['new_nearest']+=nearest
            if res>=4:
                k['safe4']+=1; k['safe4_exact']+=exact
                if not exact and nn==0: k['safe4_wrong_noflag']+=1
                if not exact and (dc>dm or any(t in('snv','indel') for _,t in dl) or not dl): k['safe4_wrong_real']+=1
            true_snv={p for p,t in dl if t=='snv'}; true_all={p for p,t in dl}
            tp=sum(1 for p in det if any(abs(p-q)<=2 for q in true_all))
            k['site_det']+=len(det); k['site_det_true']+=tp
            k['site_snv']+=len(true_snv); k['site_snv_found']+=sum(1 for q in true_snv if any(abs(p-q)<=2 for p in det))
            if exact and nn>0: k['flag_on_exact']+=1
            if not exact and nn>0: k['flag_on_inexact']+=1
            if V and ((res>=4 and not exact) or (exact and nn>0) or (dm==0 and not exact)):
                EX.append((s,g,name,'res',res,'d_call',dc,'d_min',dm,T[h][1][:2],'true',dl[:5],'det',sorted(det)[:5]))
cols=['n','exact','nearest','known','known_ok','new','new_nearest','safe4','safe4_exact','safe4_wrong_noflag','safe4_wrong_real','flag_on_exact','flag_on_inexact','site_snv','site_snv_found','site_det','site_det_true','skip']
print('gene',*cols,sep='\t'); tot=collections.Counter()
for g in ["A","B","C","DRB1","DRB3","DRB4","DRB5","DQA1","DQB1","DPA1","DPB1","DRA","E","F","G","H","K","V"]:
    tot.update(ST[g]); print(g,*[ST[g][c] for c in cols],sep='\t')
print('ALL',*[tot[c] for c in cols],sep='\t')
t=tot
print(f"known alleles 4F exact {t['known_ok']}/{t['known']} ({100*t['known_ok']/max(t['known'],1):.1f}%) | new haplotypes {t['new']}, nearest allele chosen {t['new_nearest']} | safe4 exact {t['safe4_exact']}/{t['safe4']}, inexact unflagged {t['safe4_wrong_noflag']}, inexact beyond hp/STR {t['safe4_wrong_real']} | SNV sites found {t['site_snv_found']}/{t['site_snv']} | reported sites true {t['site_det_true']}/{t['site_det']}")
for e in EX: print(*e)
