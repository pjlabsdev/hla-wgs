#!/bin/bash
# GIAB samples from the public S3 mirror: remote region+unmapped extraction, then hla-wgs (v9, full DB)
W=${HLA_WORK:-$PWD}; cd $W; S=env/bin/samtools
B=https://giab.s3.amazonaws.com/data
while read name frac url; do
  [ -s giab23/$name.json ] && grep -q 'called\": *true' giab23/$name.json && continue
  U="$B/$url"; sub=""; [ "$frac" != "1" ] && sub="-s $frac"
  t0=$(date +%s)
  $S view -H "$U" 2>/dev/null | awk '$1=="@SQ"{sub("SN:","",$2); print $2}' > giab/$name.sq; rm -f $W/*.bai
  awk 'NR==FNR{h[$1]=1; next} ($1 in h)' giab/$name.sq ref/regions.bed > giab/$name.bed
  $S view -@2 -b $sub -M -L giab/$name.bed -X "$U" "$U.bai" -o giab/$name.mhc.bam || { echo "$name fail_mhc" >> giab/fail.txt; continue; }
  $S view -u -F 0x900 giab/$name.mhc.bam | $S collate -u -O - giab/$name.tmp | $S fastq -n -1 giab/$name.a1.fq.gz -2 giab/$name.a2.fq.gz -s /dev/null -0 /dev/null - 2>/dev/null
  # unplaced unmapped pairs are adjacent in these BAMs: stream them straight to FASTQ, nothing stored
  $S view -@2 -u $sub -F 0x900 -X "$U" "$U.bai" '*' | $S fastq -n -1 giab/$name.b1.fq.gz -2 giab/$name.b2.fq.gz -s /dev/null -0 /dev/null - 2>> giab/$name.un.log || { echo "$name fail_un" >> giab/fail.txt; continue; }
  cat giab/$name.a1.fq.gz >> giab/$name.b1.fq.gz && rm -f giab/$name.a1.fq.gz && mv giab/$name.b1.fq.gz giab/$name.R1.fq.gz; cat giab/$name.a2.fq.gz >> giab/$name.b2.fq.gz && rm -f giab/$name.a2.fq.gz && mv giab/$name.b2.fq.gz giab/$name.R2.fq.gz
  echo -e "$name\textract_sec\t$(( $(date +%s)-t0 ))\tmhc\t$($S view -c giab/$name.mhc.bam)\t$(tail -1 giab/$name.un.log)" >> giab/extract.log
  rm -f giab/$name.mhc.bam giab/$name.a1.fq.gz giab/$name.a2.fq.gz giab/$name.b1.fq.gz giab/$name.b2.fq.gz
  { time HLA_DECOYS=/nonexistent nice hla-wgs bdtest giab/$name.R1.fq.gz giab/$name.R2.fq.gz giab23/$name 4 ; } 2> giab23/$name.log
  [ -s giab23/$name.json ] || { echo "$name type_fail" >> giab/fail.txt; break; }
  rm -f giab/$name.R1.fq.gz giab/$name.R2.fq.gz
done < giab/list23.txt
echo giab_done
