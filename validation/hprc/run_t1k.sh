#!/bin/bash
# T1K 1.0.10 on one sample; appends sample, wall s, CPU s and peak MB to t1k_time.tsv (needs GNU time)
W=${HLA_WORK:-$PWD}; s=$1; mkdir -p $W/t1kout; cd $W/t1kout
[ -s ${s}_genotype.tsv ] && exit 0
/usr/bin/time -o $s.time -f "%e %U %S %M" $W/t1kenv/bin/run-t1k -1 $W/sr/$s.R1.fq.gz -2 $W/sr/$s.R2.fq.gz \
  --preset hla-wgs -f $W/t1kref/hlaidx/hlaidx_dna_seq.fa -t 4 -o $s > /dev/null 2> $s.err
awk -v s=$s '{ printf "%s\t%.1f\t%.1f\t%.0f\n", s, $1, $2 + $3, $4 / 1024 }' $s.time >> t1k_time.tsv && rm -f $s.time
