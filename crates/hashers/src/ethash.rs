use byteorder::{ByteOrder, LittleEndian};
use sha3::{Digest, Keccak256, Keccak512};
use tracing::{debug, info};

use miner_core::algorithm::{Algorithm, Hasher};
use miner_core::error::{MinerError, Result};
use miner_core::types::{FoundNonce, MiningJob, Nonce};

const WORD_BYTES: usize = 4;
const DATASET_BYTES_INIT: usize = 1 << 30; // 1 GB
const DATASET_BYTES_GROWTH: usize = 1 << 23; // 8 MB
const CACHE_BYTES_INIT: usize = 1 << 24; // 16 MB
const CACHE_BYTES_GROWTH: usize = 1 << 17; // 128 KB
const MIX_BYTES: usize = 128;
const HASH_BYTES: usize = 64;
const CACHE_ROUNDS: usize = 3;
const ACCESSES: usize = 64;

// ETCHash uses epoch length 60000 (vs Ethash 30000)
const EPOCH_LENGTH: u64 = 60000;

pub struct EthashHasher {
    epoch: Option<u64>,
    cache: Vec<u8>,
    cache_size: usize,
    full_size: usize,
}

impl EthashHasher {
    pub fn new() -> Self {
        Self {
            epoch: None,
            cache: Vec::new(),
            cache_size: 0,
            full_size: 0,
        }
    }

    fn setup_epoch(&mut self, block_number: u64) -> Result<()> {
        let epoch = block_number / EPOCH_LENGTH;
        if self.epoch == Some(epoch) {
            return Ok(());
        }
        info!(epoch, block_number, "Generating ethash cache for epoch");

        self.cache_size = get_cache_size(epoch);
        self.full_size = get_full_size(epoch);

        let seed = get_seedhash(epoch);
        self.cache = make_cache(self.cache_size, &seed);
        self.epoch = Some(epoch);

        debug!(
            cache_size = self.cache_size,
            full_size = self.full_size,
            "Ethash epoch initialized"
        );
        Ok(())
    }
}

impl Hasher for EthashHasher {
    fn algorithm(&self) -> Algorithm {
        Algorithm::EtcHash
    }

    fn init(&mut self) -> Result<()> {
        Ok(())
    }

    fn hash(&self, job: &MiningJob, nonce: Nonce) -> Result<Vec<u8>> {
        if self.cache.is_empty() {
            return Err(MinerError::Hardware(
                "Ethash cache not initialized; need block height".into(),
            ));
        }

        let header_hash: [u8; 32] = job
            .blob
            .as_slice()
            .try_into()
            .map_err(|_| MinerError::Hardware("Invalid header hash length".into()))?;

        let (mix, result) =
            hashimoto_light(self.full_size, &self.cache, &header_hash, nonce.0);
        let _ = mix; // mix_hash is used for share submission
        Ok(result.to_vec())
    }

    fn hash_batch(
        &self,
        job: &MiningJob,
        start_nonce: u64,
        batch_size: u64,
    ) -> Result<Vec<FoundNonce>> {
        if self.cache.is_empty() {
            return Err(MinerError::Hardware(
                "Ethash cache not initialized; need block height".into(),
            ));
        }

        let header_hash: [u8; 32] = job
            .blob
            .as_slice()
            .try_into()
            .map_err(|_| MinerError::Hardware("Invalid header hash length".into()))?;

        let mut found = Vec::new();
        for i in 0..batch_size {
            let n = start_nonce.wrapping_add(i);
            let (_mix, result) = hashimoto_light(self.full_size, &self.cache, &header_hash, n);
            if self.meets_target(&result, &job.target) {
                found.push(FoundNonce {
                    nonce: n,
                    hash: result.to_vec(),
                });
            }
        }
        Ok(found)
    }

    fn meets_target(&self, hash: &[u8], target: &[u8]) -> bool {
        // Big-endian comparison
        for (h, t) in hash.iter().zip(target.iter()) {
            if h < t {
                return true;
            }
            if h > t {
                return false;
            }
        }
        true
    }

    fn preferred_batch_size(&self) -> u64 {
        1024
    }
}

fn is_prime(n: usize) -> bool {
    if n < 2 {
        return false;
    }
    if n < 4 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }
    let mut i = 3;
    while i * i <= n {
        if n % i == 0 {
            return false;
        }
        i += 2;
    }
    true
}

fn get_cache_size(epoch: u64) -> usize {
    let mut sz = CACHE_BYTES_INIT + CACHE_BYTES_GROWTH * epoch as usize;
    sz -= HASH_BYTES;
    while !is_prime(sz / HASH_BYTES) {
        sz -= 2 * HASH_BYTES;
    }
    sz
}

fn get_full_size(epoch: u64) -> usize {
    let mut sz = DATASET_BYTES_INIT + DATASET_BYTES_GROWTH * epoch as usize;
    sz -= MIX_BYTES;
    while !is_prime(sz / MIX_BYTES) {
        sz -= 2 * MIX_BYTES;
    }
    sz
}

fn get_seedhash(epoch: u64) -> [u8; 32] {
    let mut seed = [0u8; 32];
    for _ in 0..epoch {
        let hash = Keccak256::digest(&seed);
        seed.copy_from_slice(&hash);
    }
    seed
}

fn make_cache(cache_size: usize, seed: &[u8; 32]) -> Vec<u8> {
    let n = cache_size / HASH_BYTES;
    let mut cache = vec![0u8; cache_size];

    // Sequentially produce initial dataset
    let hash = Keccak512::digest(seed);
    cache[..HASH_BYTES].copy_from_slice(&hash);
    for i in 1..n {
        let prev_start = (i - 1) * HASH_BYTES;
        let prev = cache[prev_start..prev_start + HASH_BYTES].to_vec();
        let hash = Keccak512::digest(&prev);
        cache[i * HASH_BYTES..(i + 1) * HASH_BYTES].copy_from_slice(&hash);
    }

    // RandMemoHash
    for _ in 0..CACHE_ROUNDS {
        for i in 0..n {
            let v = LittleEndian::read_u32(&cache[i * HASH_BYTES..]) as usize % n;
            let prev_idx = if i == 0 { n - 1 } else { i - 1 };

            let mut xored = vec![0u8; HASH_BYTES];
            for j in 0..HASH_BYTES {
                xored[j] = cache[prev_idx * HASH_BYTES + j] ^ cache[v * HASH_BYTES + j];
            }
            let hash = Keccak512::digest(&xored);
            cache[i * HASH_BYTES..(i + 1) * HASH_BYTES].copy_from_slice(&hash);
        }
    }

    cache
}

fn calc_dataset_item(cache: &[u8], i: usize) -> [u8; HASH_BYTES] {
    let n = cache.len() / HASH_BYTES;
    let r = HASH_BYTES / WORD_BYTES;

    let mut mix = cache[(i % n) * HASH_BYTES..(i % n + 1) * HASH_BYTES].to_vec();
    let xored = LittleEndian::read_u32(&mix[0..4]) ^ i as u32;
    LittleEndian::write_u32(&mut mix[0..4], xored);
    let hash = Keccak512::digest(&mix);
    mix.copy_from_slice(&hash);

    for j in 0..256u32 {
        let cache_idx = fnv(
            i as u32 ^ j,
            LittleEndian::read_u32(&mix[(j as usize % r) * 4..]),
        ) as usize
            % n;
        for k in 0..HASH_BYTES {
            mix[k] = fnv_byte(mix[k], cache[cache_idx * HASH_BYTES + k]);
        }
    }

    let hash = Keccak512::digest(&mix);
    let mut result = [0u8; HASH_BYTES];
    result.copy_from_slice(&hash);
    result
}

fn hashimoto_light(
    full_size: usize,
    cache: &[u8],
    header: &[u8; 32],
    nonce: u64,
) -> ([u8; 32], [u8; 32]) {
    let n = full_size / HASH_BYTES;
    let w = MIX_BYTES / WORD_BYTES;

    // Combine header+nonce
    let mut s_input = Vec::with_capacity(40);
    s_input.extend_from_slice(header);
    s_input.extend_from_slice(&nonce.to_le_bytes());
    let s = Keccak512::digest(&s_input);
    let s_bytes: [u8; 64] = s.into();

    let mut mix = [0u8; MIX_BYTES];
    for i in 0..(MIX_BYTES / HASH_BYTES) {
        mix[i * HASH_BYTES..(i + 1) * HASH_BYTES].copy_from_slice(&s_bytes);
    }

    for i in 0..ACCESSES {
        let p = (fnv(
            i as u32,
            LittleEndian::read_u32(&mix[(i % w) * 4..]),
        ) as usize)
            % (n / (MIX_BYTES / HASH_BYTES));

        let mut newdata = [0u8; MIX_BYTES];
        for j in 0..(MIX_BYTES / HASH_BYTES) {
            let item = calc_dataset_item(cache, p * (MIX_BYTES / HASH_BYTES) + j);
            newdata[j * HASH_BYTES..(j + 1) * HASH_BYTES].copy_from_slice(&item);
        }

        for j in 0..MIX_BYTES {
            mix[j] = fnv_byte(mix[j], newdata[j]);
        }
    }

    // Compress mix
    let mut cmix = [0u8; 32];
    for i in 0..8 {
        let offset = i * 4 * WORD_BYTES;
        let mut val = LittleEndian::read_u32(&mix[offset..]);
        val = fnv(val, LittleEndian::read_u32(&mix[offset + 4..]));
        val = fnv(val, LittleEndian::read_u32(&mix[offset + 8..]));
        val = fnv(val, LittleEndian::read_u32(&mix[offset + 12..]));
        LittleEndian::write_u32(&mut cmix[i * 4..], val);
    }

    // Result
    let mut result_input = Vec::with_capacity(64 + 32);
    result_input.extend_from_slice(&s_bytes);
    result_input.extend_from_slice(&cmix);
    let result_hash = Keccak256::digest(&result_input);
    let mut result = [0u8; 32];
    result.copy_from_slice(&result_hash);

    (cmix, result)
}

fn fnv(v1: u32, v2: u32) -> u32 {
    (v1.wrapping_mul(0x01000193)) ^ v2
}

fn fnv_byte(v1: u8, v2: u8) -> u8 {
    // FNV operates on 32-bit words; for byte-level mixing we use the same idea
    let r = fnv(v1 as u32, v2 as u32);
    r as u8
}
