#!/bin/bash
W=${HLA_WORK:-$PWD}; s=$1; mkdir -p $W/t1kout; cd $W/t1kout
[ -s ${s}_genotype.tsv ] && exit 0
t0=$(date +%s.%N)
python3 - "$s" <<'PY' 
import subprocess,resource,sys,time
s=sys.argv[1]; W=os.environ.get("HLA_WORK",os.getcwd())
t=time.time()
subprocess.run([f"{W}/t1kenv/bin/run-t1k","-1",f"{W}/sr/{s}.R1.fq.gz","-2",f"{W}/sr/{s}.R2.fq.gz","--preset","hla-wgs","-f",f"{W}/t1kref/hlaidx/hlaidx_dna_seq.fa","-t","4","-o",s],stdout=subprocess.DEVNULL,stderr=open(f"{s}.err","w"))
r=resource.getrusage(resource.RUSAGE_CHILDREN)
open("t1k_time.tsv","a").write(f"{s}\t{time.time()-t:.1f}\t{r.ru_utime+r.ru_stime:.1f}\t{r.ru_maxrss/1024:.0f}\n")
PY
