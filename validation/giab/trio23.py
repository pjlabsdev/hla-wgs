import collections,sys
D=sys.argv[1] if len(sys.argv)>1 else 'giab23'
def load(s):
    c=collections.defaultdict(list)
    for l in open(f'{D}/{s}.conf.tsv').read().splitlines()[1:]:
        r=l.split('\t'); c[r[0]].append((r[2],int(r[4]),r[12] if len(r)>12 else '-'))
    return c
def cut(n,k):
    if n=='-': return '-'
    return ':'.join(n.split('*')[1].split(':')[:k])
def cons(ka,f,m):
    if len(ka)<2: ka=ka*2
    return (ka[0] in f and ka[1] in m) or (ka[1] in f and ka[0] in m)
for kid,fa,mo in (('HG002','HG003','HG004'),('HG005_BGI','HG006','HG007')):
    K,F,M=load(kid),load(fa),load(mo)
    for k in (2,3,4):
        ok=0; badl=[]
        for g in K:
            ka=[cut(x[0],k) for x in K[g]]; f=[cut(x[0],k) for x in F[g]]; m=[cut(x[0],k) for x in M[g]]
            if cons(ka,f,m): ok+=1
            else: badl.append((g,ka,f,m,[x[1] for x in K[g]+F[g]+M[g]]))
        print(kid,k,'fields consistent',ok,'of',len(K))
        for b in badl: print('   ',b)
    ok=n=0
    for g in K:
        if all(x[1]>=4 for x in K[g]+F[g]+M[g]):
            n+=1; c=cons([cut(x[0],4) for x in K[g]],[cut(x[0],4) for x in F[g]],[cut(x[0],4) for x in M[g]]); ok+=c
            if not c: print('   SAFE4 conflict',g)
    print(kid,'genes where all calls are safe at 4 fields: consistent',ok,'of',n)
