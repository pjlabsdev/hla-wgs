import mappy, sys, collections, os
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)))
import os; W=os.environ.get("HLA_WORK",os.getcwd())
GENES=["A","B","C","DRB1","DRB3","DRB4","DRB5","DQA1","DQB1","DPA1","DPB1","DRA"]
def load(path):
    name=None; seq=[]
    for line in open(path):
        if line.startswith(">"):
            if name: yield name,"".join(seq)
            name=line[1:].split()[1]; seq=[]
        else: seq.append(line.strip())
    if name: yield name,"".join(seq)
nuc=[(n,s) for n,s in load(f"{W}/hla_nuc.fasta") if n.split("*")[0] in GENES]
gen=[x for g in GENES for x in load(f"{W}/gen/{g}_gen.fasta")]
def crop(fa):
    # keep only the MHC window of the contigs: locate HLA-A and DPB1 and cut +-300 kb
    probe=[x for x in gen if x[0] in ("A*01:01:01:01","B*07:02:01:01","C*07:01:01:01","DRA*01:01:01:01","DRB1*01:01:01:01","DQA1*01:01:01:01","DQB1*02:01:01:01","DPA1*01:03:01:01","DPB1*01:01:01:01","DPB1*04:01:01:01")]
    a=mappy.Aligner(fa,preset="asm20")
    pos={}
    for n,s in probe:
        for h in a.map(s):
            pos.setdefault(h.ctg,[]).append(h.r_st)
    if not pos: return fa
    seqs={}
    name=None
    for line in open(fa):
        if line.startswith(">"): name=line[1:].split()[0]; seqs[name]=[]
        else: seqs[name].append(line.strip())
    out=fa.replace(".fa",".mhc.fa")
    with open(out,"w") as fh:
        for c,ps in pos.items():
            s="".join(seqs[c]); lo=max(0,min(ps)-300000); hi=min(len(s),max(ps)+300000)
            fh.write(f">{c}_{lo}\n{s[lo:hi]}\n")
    return out
for fa in sys.argv[1:]:
    hp=fa
    outname=fa.replace(".fa",".hla.tsv")
    fa=crop(fa)
    if os.environ.get("CROPONLY"):
        print("cropped",fa,flush=True); continue
    outf=open(outname+".tmp","w")
    for kind,al,preset in [("gen",gen,"asm20"),("nuc",nuc,"splice:hq")]:
        a=mappy.Aligner(fa,preset=preset,best_n=20)
        loci=collections.defaultdict(list)
        for n,s in al:
            g=n.split("*")[0]
            for h in a.map(s):
                if (h.q_en-h.q_st)/len(s)<0.98: continue
                ed=h.NM+(len(s)-(h.q_en-h.q_st))
                loci[(g,h.ctg,h.r_st//20000)].append((ed,-len(s),n,h.r_st))
        
        for (g,ctg,b),v in sorted(loci.items()):
            v.sort(); best=v[0][0]
            if best>len(v) and best>30: continue
            top=[x[2] for x in v if x[0]==best and x[1]==v[0][1]]
            nxt=[x for x in v if x[0]>best]
            print(f"{kind}\t{g}\t{ctg}:{v[0][3]}\tedits={best}\tlen={-v[0][1]}\t{','.join(top[:5])}{' +'+str(len(top)-5) if len(top)>5 else ''}\tnext={nxt[0][2]+'('+str(nxt[0][0])+')' if nxt else '-'}",file=outf)
    outf.close(); os.replace(outname+".tmp", outname)
