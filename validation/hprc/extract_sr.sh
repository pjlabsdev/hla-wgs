#!/bin/bash
# extract MHC+alts+HLA decoys+unmapped from a 1000G 30x CRAM -> sr/S.fq.gz pairs
W=${HLA_WORK:-$PWD}; cd $W; s=$1; S=env/bin/samtools

[ -s sr/$s.R1.fq.gz ] && exit 0
r=$(awk -v s=$s '$1==s' cram.map | head -1); d=$(echo "$r"|cut -f2); f=$(echo "$r"|cut -f3)
U="https://1000genomes.s3.amazonaws.com/1000G_2504_high_coverage/additional_698_related/data/$d/$f"
curl -sS -o /dev/null -w "%{http_code}" "$U.crai" | grep -q 200 || U="https://1000genomes.s3.amazonaws.com/1000G_2504_high_coverage/data/$d/$f"
t0=$(date +%s)
$S view -@2 -b -T ref/full.fa -M -L ref/regions.bed -X "$U" "$U.crai" -o sr/$s.mhc.bam || exit 1
$S view -@2 -b -T ref/full.fa -X "$U" "$U.crai" '*' -o sr/$s.un.bam || exit 1
$S cat sr/$s.mhc.bam sr/$s.un.bam | $S view -u -F 0x900 - | $S collate -u -O - | $S fastq -n -1 sr/$s.R1.fq.gz -2 sr/$s.R2.fq.gz -s /dev/null -0 /dev/null - 2>/dev/null
echo -e "$s\textract_sec\t$(( $(date +%s)-t0 ))\tmhc_reads\t$($S view -c sr/$s.mhc.bam)\tunmapped\t$($S view -c sr/$s.un.bam)" >> sr/extract.log
rm -f sr/$s.un.bam
