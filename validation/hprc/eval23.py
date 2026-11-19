# 2- and 3-field agreement of the calls with the full-gene assembly truth (truth/truth4.tsv):
# a call agrees if its first k fields occur among the best-matching database alleles of its haplotype.
import sys,collections,itertools,os,re
D=sys.argv[1]
T=collections.defaultdict(dict)
for l in open('truth/truth4.tsv'):
    r=l.rstrip('\n').split('\t')
    if r[2]=='_done' or r[3]=='-': continue
    T[(r[0],r[2])][r[1]]=r[4].split(',')
cut=lambda n,k:':'.join(re.sub(r'[A-Z]$','',n.split('*')[1]).split(':')[:k])
ST=collections.defaultdict(collections.Counter); bad=[]
for s in sorted({k[0] for k in T}):
    if not os.path.exists(f'{D}/{s}.conf.tsv'): continue
    conf=collections.defaultdict(list)
    for l in open(f'{D}/{s}.conf.tsv').read().splitlines()[1:]:
        r=l.split('\t'); conf[r[0]].append(r[2])
    for g,c in conf.items():
        t=T.get((s,g),{})
        if len(c)==1 and len(t)==2: c=c*2
        hh=sorted(t)
        if len(c)!=len(hh) or not c or '-' in c: continue
        for k in (2,3):
            ok=lambda n,h: cut(n,k) in {cut(x,k) for x in t[h] if len(x.split(':'))>=k} or not any(len(x.split(':'))>=k for x in t[h])
            best=max(itertools.permutations(range(len(c))),key=lambda pm:sum(ok(c[i],h) for i,h in zip(pm,hh)))
            for i,h in zip(best,hh):
                ST[g][f'n{k}']+=1; o=ok(c[i],h); ST[g][f'ok{k}']+=o
                if not o: bad.append((k,s,g,c[i],t[h][:3]))
tot=collections.Counter()
print('gene','n','ok2','ok3',sep='\t')
for g in ST: tot.update(ST[g]); print(g,ST[g]['n2'],ST[g]['ok2'],ST[g]['ok3'],sep='\t')
print('ALL',tot['n2'],tot['ok2'],tot['ok3'],sep='\t')
for b in bad: print(*b)
