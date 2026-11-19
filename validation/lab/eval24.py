# eval.py: compare hla-wgs (lab/out) and T1K (t1kout) with 1000G lab SBT typing at 2 fields
import sys,os,glob,collections,itertools
import os; H=os.environ.get("HLA_WORK",os.getcwd()); G=["A","B","C","DRB1","DQB1"]
rows=open(f"{H}/20140702_hla_diversity.txt").read().replace('"','').splitlines()
hdr=rows[0].split(); truth={}; pop={}
for l in rows[1:]:
    r=l.split(); pop[r[0]]=r[1]
    truth[r[0]]={g:[set(":".join(a.split(":")[:2]) for a in r[hdr.index(c)].split("/")) if r[hdr.index(c)] not in("","NA") else None for c in (g,g+".1")] for g in G}
def amb(x):
    p=x.strip().rstrip("*").split("/"); fam=p[0].split(":")[0]
    return {":".join(q.split(":")[:2]) if ":" in q else fam+":"+q for q in p}
T18={}; pop18={}
for l in open(f"{H}/lab/hla2018.txt").read().splitlines()[1:]:
    r=l.split("\t"); r+=[""]*(13-len(r)); pop18[r[2]]=r[1]
    cols=dict(zip(["A","B","C","DQB1","DRB1"],[(3,4),(5,6),(7,8),(9,10),(11,12)]))
    T18[r[2]]={g:[(amb(r[i]) if r[i].strip() not in("","-","None") else None) for i in ij] for g,ij in cols.items()}
SRC=sys.argv[1] if len(sys.argv)>1 else "2014"
if SRC=="2018": truth=T18; pop=pop18
elif SRC=="any":
    for k,v in T18.items():
        if k not in truth: truth[k]=v; pop[k]=pop18[k]
def f2(x): return ":".join(x.split("*")[-1].split(":")[:2])
def ours(s):
    d={}; c={}
    for l in open(f"{H}/lab/out24/{s}.tsv").read().splitlines()[1:]:
        r=l.split("\t"); d[r[0]]=[f2(r[3]),f2(r[4]) if r[4]!="-" else f2(r[3])]
    for l in open(f"{H}/lab/out24/{s}.conf.tsv").read().splitlines()[1:]:
        r=l.split("\t"); c.setdefault(r[0],[]).append(int(r[4]))
    return d,c
def t1k(s):
    p=f"{H}/t1kout/{s}_genotype.tsv"
    if not os.path.exists(p): return None
    d={}
    for l in open(p):
        r=l.rstrip("\n").split("\t"); g=r[0].replace("HLA-","")
        a1=f2(r[2]) if r[2]!="." else None; a2=f2(r[5]) if len(r)>5 and r[5]!="." else a1
        d[g]=[a1,a2]
    return d
def score(call,tr):
    # returns number correct (0..2) under best pairing; truth slot None = untyped (skip)
    best=-1
    for a,b in ((0,1),(1,0)):
        n=sum(1 for x,t in ((call[a],tr[0]),(call[b],tr[1])) if t is not None and x in t)
        best=max(best,n)
    return best, sum(t is not None for t in tr)
res=collections.defaultdict(lambda:[0,0]); rt=collections.defaultdict(lambda:[0,0]); rpair=collections.defaultdict(lambda:[0,0])
safe=[0,0]; errs=[]; done=[]
for p in sorted(glob.glob(f"{H}/lab/out24/*.tsv")):
    s=os.path.basename(p)[:-4]
    if s.endswith(".conf") or not os.path.exists(f"{H}/lab/out24/{s}.conf.tsv"): continue
    if s not in truth: continue
    done.append(s); d,c=ours(s); t=t1k(s)
    for g in G:
        tr=truth[s][g]; k,n=score(d[g],tr); res[g][0]+=k; res[g][1]+=n
        if all(x is not None for x in tr) and min(c[g])>=2: safe[0]+=k; safe[1]+=n
        if k<n: errs.append((s,pop[s],g,"/".join(d[g]),tr, "/".join(map(str,t[g])) if t and g in t else "-"))
        if t:
            tk,tn=score(t.get(g,[None,None]),tr); rt[g][0]+=tk; rt[g][1]+=tn
            rpair[g][0]+=k; rpair[g][1]+=n
print("samples",len(done),"with T1K",sum(1 for s in done if t1k(s)))
tot=[sum(res[g][0] for g in G),sum(res[g][1] for g in G)]
for g in G: print(f"{g}\tours {res[g][0]}/{res[g][1]}\t| on T1K subset: ours {rpair[g][0]}/{rpair[g][1]} T1K {rt[g][0]}/{rt[g][1]}")
print("total ours",tot, "T1K subset ours",[sum(rpair[g][0] for g in G),sum(rpair[g][1] for g in G)],"T1K",[sum(rt[g][0] for g in G),sum(rt[g][1] for g in G)])
print("safe(>=2 field) ours",safe)
for e in errs: print("ERR",*e[:4],[ "/".join(sorted(x))[:60] if x else None for x in e[4]],"T1K",e[5])
