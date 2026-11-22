#!/bin/bash
# type the new HGSVC3 samples with the 1.1 build: extract short reads, type, delete the reads
cd /tmp/claude-0/hla; B=/tmp/claude-0/hla/rs/hla-wgs_1.1h; mkdir -p hgsvc_out
for s in HG00096 $(cat lr/hgsvc_todo.txt); do
  [ -s hgsvc_out/$s.json ] && continue
  for t in 1 2; do scripts/extract_sr.sh $s && break; sleep 20; done
  rm -f sr/$s.mhc.bam sr/$s.un.bam
  [ -s sr/$s.R1.fq.gz ] || { echo "$s extract_fail" >> lr/hgsvc_type.fail; continue; }
  $B bdtest sr/$s.R1.fq.gz sr/$s.R2.fq.gz hgsvc_out/$s 2 > hgsvc_out/$s.log 2>&1
  rm -f sr/$s.R1.fq.gz sr/$s.R2.fq.gz
done
echo done > lr/hgsvc_type.done
