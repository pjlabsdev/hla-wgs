# stream a (gz) assembly fasta from stdin, write contigs that contain HLA k-mers
import sys
import os; W=os.environ.get("HLA_WORK",os.getcwd()); K=31
def load(path):
    name=None; seq=[]
    for line in open(path):
        if line.startswith(">"):
            if name: yield name,"".join(seq)
            name=line[1:].split()[1]; seq=[]
        else: seq.append(line.strip())
    if name: yield name,"".join(seq)
anch=set()
cnt={}
for n,s in load(f"{W}/hla_nuc.fasta"):
    g=n.split("*")[0]
    if g not in ("A","B","C","DRB1","DQB1","DPB1"): continue
    cnt[g]=cnt.get(g,0)+1
    if cnt[g]>40: continue
    for j in range(0,len(s)-K+1): anch.add(s[j:j+K])
def rc(s): return s[::-1].translate(str.maketrans("ACGT","TGCA"))
anch|= {rc(x) for x in anch}
out=open(sys.argv[1],"w")
def check(name,seq):
    if len(seq)<50000: return
    hits=sum(1 for j in range(0,len(seq)-K,1) if seq[j:j+K] in anch) if False else None
    # fast: test anchors by substring search on uppercase seq
    s=seq.upper(); h=0
    for a in list(anch)[:4000]:
        pass
    return
name=None; buf=[]
def flush():
    if name is None: return
    s="".join(buf).upper()
    h=0
    for j in range(0,len(s)-K,25):
        if s[j:j+K] in anch:
            h+=1
            if h>=20: break
    if h>=20:
        out.write(f">{name}\n{s}\n"); print("keep",name,len(s),file=sys.stderr)
for line in sys.stdin:
    if line.startswith(">"):
        flush(); name=line[1:].split()[0]; buf=[]
    else: buf.append(line.strip())
flush(); out.close()
