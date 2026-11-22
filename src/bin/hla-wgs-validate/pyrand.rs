//! Python's random module (Mersenne Twister and the methods the simulator uses), reproduced exactly
//! so that a seed gives the same simulated reads as the earlier Python simulator.
pub struct PyRandom {
    mt: [u32; 624],
    idx: usize,
    gauss_next: Option<f64>,
}

impl PyRandom {
    /// random.Random(seed) for a non-negative integer seed
    pub fn new(seed: u64) -> PyRandom {
        let mut key: Vec<u32> = vec![];
        let mut s = seed;
        while s > 0 {
            key.push(s as u32);
            s >>= 32;
        }
        if key.is_empty() {
            key.push(0);
        }
        let mut r = PyRandom { mt: [0; 624], idx: 624, gauss_next: None };
        r.init_genrand(19650218);
        let (mut i, mut j) = (1usize, 0usize);
        let n = 624usize;
        let mut k = n.max(key.len());
        while k > 0 {
            let prev = r.mt[i - 1];
            r.mt[i] = (r.mt[i] ^ (prev ^ (prev >> 30)).wrapping_mul(1664525)).wrapping_add(key[j]).wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= n {
                r.mt[0] = r.mt[n - 1];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
            k -= 1;
        }
        k = n - 1;
        while k > 0 {
            let prev = r.mt[i - 1];
            r.mt[i] = (r.mt[i] ^ (prev ^ (prev >> 30)).wrapping_mul(1566083941)).wrapping_sub(i as u32);
            i += 1;
            if i >= n {
                r.mt[0] = r.mt[n - 1];
                i = 1;
            }
            k -= 1;
        }
        r.mt[0] = 0x8000_0000;
        r
    }

    fn init_genrand(&mut self, s: u32) {
        self.mt[0] = s;
        for i in 1..624 {
            let p = self.mt[i - 1];
            self.mt[i] = 1812433253u32.wrapping_mul(p ^ (p >> 30)).wrapping_add(i as u32);
        }
        self.idx = 624;
    }

    fn genrand(&mut self) -> u32 {
        const UPPER: u32 = 0x8000_0000;
        const LOWER: u32 = 0x7fff_ffff;
        const MAT_A: u32 = 0x9908_b0df;
        if self.idx >= 624 {
            for kk in 0..624 {
                let y = (self.mt[kk] & UPPER) | (self.mt[(kk + 1) % 624] & LOWER);
                self.mt[kk] = self.mt[(kk + 397) % 624] ^ (y >> 1) ^ if y & 1 == 1 { MAT_A } else { 0 };
            }
            self.idx = 0;
        }
        let mut y = self.mt[self.idx];
        self.idx += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }

    /// random.random()
    pub fn random(&mut self) -> f64 {
        let a = (self.genrand() >> 5) as f64;
        let b = (self.genrand() >> 6) as f64;
        (a * 67108864.0 + b) * (1.0 / 9007199254740992.0)
    }

    fn getrandbits(&mut self, k: u32) -> u64 {
        if k <= 32 {
            return (self.genrand() >> (32 - k)) as u64;
        }
        // little-endian words, the last one shortened
        let mut r: u64 = 0;
        let mut left = k;
        let mut shift = 0;
        while left > 0 {
            let w = self.genrand() as u64;
            let take = left.min(32);
            r |= (w >> (32 - take)) << shift;
            shift += 32;
            left -= take;
        }
        r
    }

    fn randbelow(&mut self, n: u64) -> u64 {
        let k = 64 - n.leading_zeros();
        let mut r = self.getrandbits(k);
        while r >= n {
            r = self.getrandbits(k);
        }
        r
    }

    /// index for random.choice on a sequence of length n
    pub fn randbelow_pub(&mut self, n: usize) -> usize {
        self.randbelow(n as u64) as usize
    }

    /// random.randrange(start, stop)
    pub fn randrange(&mut self, start: i64, stop: i64) -> i64 {
        start + self.randbelow((stop - start) as u64) as i64
    }

    /// random.randint(a, b)
    pub fn randint(&mut self, a: i64, b: i64) -> i64 {
        self.randrange(a, b + 1)
    }

    /// random.choice(seq)
    pub fn choice<T: Clone>(&mut self, seq: &[T]) -> T {
        seq[self.randbelow(seq.len() as u64) as usize].clone()
    }

    /// random.sample(population, k)
    pub fn sample<T: Clone>(&mut self, pop: &[T], k: usize) -> Vec<T> {
        let n = pop.len();
        let mut setsize = 21usize;
        if k > 5 {
            setsize += 4usize.pow(((k as f64 * 3.0).ln() / 4f64.ln()).ceil() as u32);
        }
        let mut out = Vec::with_capacity(k);
        if n <= setsize {
            let mut pool: Vec<T> = pop.to_vec();
            for i in 0..k {
                let j = self.randbelow((n - i) as u64) as usize;
                out.push(pool[j].clone());
                pool[j] = pool[n - i - 1].clone();
            }
        } else {
            let mut sel = std::collections::HashSet::new();
            for _ in 0..k {
                let mut j = self.randbelow(n as u64) as usize;
                while sel.contains(&j) {
                    j = self.randbelow(n as u64) as usize;
                }
                sel.insert(j);
                out.push(pop[j].clone());
            }
        }
        out
    }

    /// random.gauss(mu, sigma)
    pub fn gauss(&mut self, mu: f64, sigma: f64) -> f64 {
        let z = match self.gauss_next.take() {
            Some(z) => z,
            None => {
                let x2pi = self.random() * std::f64::consts::TAU;
                let g2rad = (-2.0 * (1.0 - self.random()).ln()).sqrt();
                self.gauss_next = Some(x2pi.sin() * g2rad);
                x2pi.cos() * g2rad
            }
        };
        mu + z * sigma
    }
}
