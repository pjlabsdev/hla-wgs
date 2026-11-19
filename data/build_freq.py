import json,collections,sys
# build_freq.py OUT EXCLUDE_FILE...  (2-field prior from 1000G lab typing, ambiguity split evenly)
out=sys.argv[1]; ex=set()
for fn in sys.argv[2:]:
    ex|={l.split()[0].strip('"') for l in open(fn) if l.strip()}
rows=[l.rstrip("\n").split(" ") for l in open("20140702_hla_diversity.txt")]
hdr=[h.strip('"') for h in rows[0]]; genes=["A","B","C","DRB1","DQB1"]
cnt={g:collections.Counter() for g in genes}
for r in rows[1:]:
    r=[x.strip('"') for x in r]; s=r[0]
    if s in ex: continue
    for g in genes:
        for col in (g,g+".1"):
            v=r[hdr.index(col)]
            if v in("","NA"): continue
            al=list(dict.fromkeys(":".join(a.split(":")[:2]) for a in v.split("/")))
            for a in al: cnt[g][a]+=1/len(al)
f={g:{a:c/sum(cnt[g].values()) for a,c in cnt[g].items()} for g in genes}
json.dump(f,open(out,"w"))
