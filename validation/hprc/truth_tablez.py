# truth/truthz.tsv from exon-projection files (S.hap.exon.tsv): sample hap gene allele4 gen_edits set2 set3
import glob, re, os
def f(a,n): return ":".join(re.sub(r"[A-Z]$","",a.split("*")[1]).split(":")[:n])
out=open("truth/truthz.tsv","w"); nov=open("truth/novelz.tsv","w")
for p in sorted(x for x in glob.glob("truth/*.exonz.tsv") if ".mhc." not in x):
    s,h=os.path.basename(p).split(".")[:2]
    rows=[]
    for line in open(p):
        g,loc,ed,best,fullm,match,nex,core,nxt=line.rstrip("\n").split("\t")
        ctg,pos=loc.rsplit(":",1)
        rows.append((ctg,int(pos),g,int(ed),best,[] if fullm=="-" else fullm.split(","),[] if match=="-" else match.split(","),int(core)))
    rows.sort(key=lambda x:(x[0],x[1]))
    cl=[]
    for r in rows:
        if cl and cl[-1][-1][0]==r[0] and abs(r[1]-cl[-1][0][1])<3000: cl[-1].append(r)
        else: cl.append([r])
    win=sorted((min(c,key=lambda r:(r[3],-len(r[5]))) for c in cl),key=lambda r:(r[0],r[2],r[1]))
    mg=[]
    for r in win:   # same gene twice within 12 kb on one contig = one locus split by the 20 kb bucketing
        if mg and mg[-1][0]==r[0] and mg[-1][2]==r[2] and abs(r[1]-mg[-1][1])<12000:
            mg[-1]=min(mg[-1],r,key=lambda r:(r[3],-len(r[5])))
        else: mg.append(r)
    for ctg,pos,g,ed,best,fullm,match,core in mg:
        if ed>150 or not core: continue
        if ed==0: a=best
        elif fullm: a=fullm[0]
        elif match: a=match[0]
        else: a=best
        s3={f(x,3) for x in fullm}
        s2={f(x,2) for x in (fullm or match)}
        if not (fullm or match):
            print(s,h,g,best,ed,file=nov); ed=99
        elif not fullm: s3=set()
        out.write(f"{s}\t{h}\t{g}\t{a}\t{ed}\t{','.join(sorted(s2))}\t{','.join(sorted(s3))}\n")
out.close(); nov.close()
