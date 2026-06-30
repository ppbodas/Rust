// Bloom filter using double-hashing: h_i(x) = h1(x) + i*h2(x)
// Serialized layout: num_bits: u32 | num_hashes: u32 | bit_array bytes

pub struct BloomFilter {
    bits: Vec<u8>,
    num_bits: usize,
    num_hashes: usize,
}

impl BloomFilter {
    pub fn new(capacity: usize, fp_rate: f64) -> Self {
        // Optimal bit count: m = -n * ln(p) / ln(2)^2
        let n = capacity.max(1) as f64;
        let num_bits = ((-n * fp_rate.ln()) / (2f64.ln().powi(2))).ceil() as usize;
        let num_bits = num_bits.max(64);
        // Optimal hash count: k = (m/n) * ln(2)
        let num_hashes = ((num_bits as f64 / n) * 2f64.ln()).ceil() as usize;
        let num_hashes = num_hashes.clamp(1, 8);

        BloomFilter {
            bits: vec![0u8; (num_bits + 7) / 8],
            num_bits,
            num_hashes,
        }
    }

    fn hashes(&self, key: &[u8]) -> impl Iterator<Item = usize> + '_ {
        // FNV-1a for h1, FNV-1a with different offset for h2
        let h1 = fnv1a(key, 14695981039346656037u64);
        let h2 = fnv1a(key, 1099511628211u64);
        (0..self.num_hashes).map(move |i| {
            (h1.wrapping_add((i as u64).wrapping_mul(h2)) as usize) % self.num_bits
        })
    }

    pub fn insert(&mut self, key: &[u8]) {
        for bit in self.hashes(key).collect::<Vec<_>>() {
            self.bits[bit / 8] |= 1 << (bit % 8);
        }
    }

    pub fn may_contain(&self, key: &[u8]) -> bool {
        self.hashes(key)
            .collect::<Vec<_>>()
            .into_iter()
            .all(|bit| self.bits[bit / 8] & (1 << (bit % 8)) != 0)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(8 + self.bits.len());
        out.extend_from_slice(&(self.num_bits as u32).to_le_bytes());
        out.extend_from_slice(&(self.num_hashes as u32).to_le_bytes());
        out.extend_from_slice(&self.bits);
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let num_bits = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
        let num_hashes = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
        BloomFilter {
            bits: bytes[8..].to_vec(),
            num_bits,
            num_hashes,
        }
    }
}

fn fnv1a(data: &[u8], offset: u64) -> u64 {
    let mut h = offset;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211u64);
    }
    h
}
