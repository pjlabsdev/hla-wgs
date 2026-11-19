#!/bin/bash
# fetch HPRC R2 haplotype assemblies, keep MHC contigs, crop to MHC window -> truth/S.{pat,mat|h1,h2}.mhc.fa
W=${HLA_WORK:-$PWD}; s=$1; cd $W
python3 -c "
import csv,sys
for r in csv.DictReader(open('r2.index.csv')):
    if r['sample_id']=='$s':
        n=r['assembly_name']; h='pat' if '_pat_' in n else 'mat' if '_mat_' in n else 'h'+r['haplotype']
        print(h, r['assembly'].replace('s3://human-pangenomics/','https://human-pangenomics.s3.amazonaws.com/'))
" | while read h u; do
  [ -s truth/$s.$h.mhc.fa ] && continue
  for i in 1 2 3 4 5; do curl -sS -C - --retry 5 -o dl_$s.$h.fa.gz "$u" && break; sleep 5; done
  gunzip -c dl_$s.$h.fa.gz | python3 scripts/mhc_contigs.py truth/$s.$h.fa 2> truth/$s.$h.log
  rm -f dl_$s.$h.fa.gz
  [ -s truth/$s.$h.fa ] && CROPONLY=1 python3 scripts/asm_type2.py truth/$s.$h.fa >/dev/null 2>&1 && rm -f truth/$s.$h.fa
done
