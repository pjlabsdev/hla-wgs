# Exon-level truth: map IMGT genomic alleles to the haplotype assembly, project the exon
# coordinates of the best hit onto the contig, read the assembly's own exon sequences and
# look them up among all IMGT alleles (incl. exon-only ones). Output appended per hap file.
# usage: truth_exon.py S.hap.mhc.fa ...   -> S.hap.exon.tsv
import mappy, pickle, sys, os, collections, re
import os; W=os.environ.get("HLA_WORK",os.getcwd())
GENES=["DMA","DMB","DOA","DOB","MICA","MICB","TAP1","TAP2"]
REF=pickle.load(open(f"{W}/proto/ref.pkl","rb"))
def rc(s): return s[::-1].translate(str.maketrans("ACGTN","TGCAN"))
full={g:[a for a in REF[g] if not a.get("imputed") and a["feat"][0][0]=="UTR" and a["feat"][-1][0]=="UTR"] for g in GENES}
print({g:len(v) for g,v in full.items()},flush=True)
# exon tables: per gene, exon number -> seq -> set(allele names)
ext={}
for g in GENES:
    t=collections.defaultdict(lambda: collections.defaultdict(set))
    for a in REF[g]:
        for k,s in a["exons"].items(): t[k][s].add(a["name"])
    ext[g]=t
def project(a,h,ctgseq):
    """return exon number -> contig sequence (allele orientation)"""
    L=len(a["genomic"]); out={}
    # walk cigar: query coords in the orientation used by minimap2 (rc for - strand)
    qpos=h.q_st if h.strand==1 else L-h.q_en
    rpos=h.r_st
    qmap={}
    for ln,op in h.cigar:
        if op in (0,7,8):
            for i in range(ln): qmap[qpos+i]=rpos+i
            qpos+=ln; rpos+=ln
        elif op==1: qpos+=ln
        elif op in (2,3): rpos+=ln
    for f in a["feat"]:
        if f[0]!="exon": continue
        s,e=f[2],f[3]
        if h.strand==-1: s,e=L-e,L-s
        if s not in qmap or (e-1) not in qmap: continue
        rs,re_=qmap[s],qmap[e-1]+1
        seg=ctgseq[rs:re_]
        out[f[1]]=seg if h.strand==1 else rc(seg)
    return out
for fa in sys.argv[1:]:
    seqs={n:s for n,s,_ in mappy.fastx_read(fa)}
    al=mappy.Aligner(fa,preset="asm20",best_n=20)
    outname=fa.replace(".mhc.fa",".exonz.tsv")
    rows=[]
    for g in GENES:
        loci=collections.defaultdict(list)
        for a in full[g]:
            s=a["genomic"]
            for h in al.map(s):
                if (h.q_en-h.q_st)/len(s)<0.9: continue
                ed=h.NM+(len(s)-(h.q_en-h.q_st))
                loci[(h.ctg,h.r_st//20000)].append((ed,a["name"],a,h))
        for (ctg,b),v in loci.items():
            v.sort(key=lambda x:x[0]); ed,name,a,h=v[0]
            exs=project(a,h,seqs[ctg])
            # which alleles agree on every exon they define?
            cnt=collections.Counter(); seen=collections.Counter()
            for k,sq in exs.items():
                for n in ext[g][k].get(sq,()): cnt[n]+=1
            match=[]
            for n,c in cnt.items():
                al_=next(x for x in REF[g] if x["name"]==n) if False else None
            byname={x["name"]:x for x in REF[g]}
            for n,c in cnt.items():
                ks=[k for k in byname[n]["exons"] if k in exs]
                if c==len(byname[n]["exons"]) and c==len(ks): match.append(n)
            fullm=[n for n in match if not byname[n]["partial"]]
            core=all(k in exs for k in ((2,3) if g in("A","B","C") else (2,)))
            rows.append((g,f"{ctg}:{h.r_st}",ed,name,",".join(sorted(fullm)) or "-",",".join(sorted(match)) or "-",len(exs),int(core),v[1][0] if len(v)>1 else -1))
    with open(outname+".tmp","w") as fh:
        for r in rows: fh.write("\t".join(map(str,r))+"\n")
    os.replace(outname+".tmp",outname)
    print(outname,len(rows),flush=True)
