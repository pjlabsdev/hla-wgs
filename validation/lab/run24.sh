#!/bin/bash
cd ${HLA_WORK:-$PWD}; s=$1
[ -s lab/out24/$s.tsv ] && exit 0
for t in 1 2; do scripts/extract_sr.sh $s && break; sleep 20; done
rm -f sr/$s.mhc.bam
[ -s sr/$s.R1.fq.gz ] || { echo "$s extract_fail" >> lab/fail24.txt; exit 0; }
k=$(awk -v s=$s '$1==s{print $2}' lab/folds.tsv)
HLA_DECOYS=/nonexistent nice hla-wgs bdtest_f$k sr/$s.R1.fq.gz sr/$s.R2.fq.gz lab/out24/$s 2 > lab/out24/$s.log 2>&1
rm -f sr/$s.R1.fq.gz sr/$s.R2.fq.gz
exit 0
