import sys, collections
truth = {l.split()[0]: l.split() for l in open(sys.argv[1])}
asg = collections.defaultdict(set)
for l in open(sys.argv[2]):
    p = l.rstrip("\n").split("\t")
    if len(p) < 3 or not p[1].isdigit():
        continue
    for r in p[2].split(","):
        if r:
            asg[r].add(p[0])
st = collections.Counter()
for r, (n, g, a, inp, pos, *g2) in truth.items():
    if g2 and g2[0] != g:
        inp = "chim"
    s = asg.get(r, set())
    k = "unass" if not s else ("only" if s == {g} else ("plus" if g in s else "wrong"))
    st[(inp, k)] += 1
    st[("all", k)] += 1
for grp in ["all", "1", "0", "chim"]:
    tot = sum(v for (x, _), v in st.items() if x == grp)
    if not tot:
        continue
    print({"all": "all", "1": "in panel", "0": "unseen", "chim": "chimeric"}[grp], tot, " ".join(f"{k}={st[(grp,k)]} ({100*st[(grp,k)]/tot:.2f}%)" for k in ["only", "plus", "wrong", "unass"]))
