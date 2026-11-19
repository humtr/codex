//! Streaming SHA-256 as specified in FIPS 180-4.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

const INITIAL_STATE: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];
const ROUND_CONSTANTS: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

pub(super) struct Sha256 {
    state: [u32; 8],
    block: [u8; 64],
    buffered: usize,
    bit_len: u64,
}

impl Sha256 {
    pub(super) fn new() -> Self {
        Self {
            state: INITIAL_STATE,
            block: [0; 64],
            buffered: 0,
            bit_len: 0,
        }
    }

    pub(super) fn update(&mut self, mut bytes: &[u8]) {
        self.bit_len += u64::try_from(bytes.len())
            .ok()
            .and_then(|len| len.checked_mul(8))
            .filter(|len| *len <= u64::MAX - self.bit_len)
            .expect("SHA-256 input exceeds the FIPS 180-4 length limit");
        if self.buffered != 0 {
            let copied = bytes.len().min(self.block.len() - self.buffered);
            self.block[self.buffered..self.buffered + copied].copy_from_slice(&bytes[..copied]);
            self.buffered += copied;
            bytes = &bytes[copied..];
            if self.buffered < self.block.len() {
                return;
            }
            Self::compress(&mut self.state, &self.block);
        }
        let (blocks, tail) = bytes.as_chunks::<64>();
        for block in blocks {
            Self::compress(&mut self.state, block);
        }
        self.block[..tail.len()].copy_from_slice(tail);
        self.buffered = tail.len();
    }

    pub(super) fn finish(mut self) -> String {
        self.block[self.buffered] = 0x80;
        self.block[self.buffered + 1..].fill(0);
        if self.buffered >= 56 {
            Self::compress(&mut self.state, &self.block);
            self.block.fill(0);
        }
        self.block[56..].copy_from_slice(&self.bit_len.to_be_bytes());
        Self::compress(&mut self.state, &self.block);
        let mut hex = String::with_capacity(64);
        for word in self.state {
            use std::fmt::Write as _;
            write!(hex, "{word:08x}").expect("writing into a String cannot fail");
        }
        hex
    }

    fn compress(state: &mut [u32; 8], block: &[u8; 64]) {
        let mut schedule = [0u32; 64];
        for (word, bytes) in schedule.iter_mut().zip(block.as_chunks::<4>().0) {
            *word = u32::from_be_bytes(*bytes);
        }
        for i in 16..64 {
            let x = schedule[i - 15];
            let y = schedule[i - 2];
            let sigma0 = x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3);
            let sigma1 = y.rotate_right(17) ^ y.rotate_right(19) ^ (y >> 10);
            schedule[i] = schedule[i - 16]
                .wrapping_add(sigma0)
                .wrapping_add(schedule[i - 7])
                .wrapping_add(sigma1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
        for (word, constant) in schedule.into_iter().zip(ROUND_CONSTANTS) {
            let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(sum1)
                .wrapping_add(choose)
                .wrapping_add(constant)
                .wrapping_add(word);
            let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let t2 = sum0.wrapping_add(majority);
            [a, b, c, d, e, f, g, h] = [t1.wrapping_add(t2), a, b, c, d.wrapping_add(t1), e, f, g];
        }
        for (word, sum) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *word = word.wrapping_add(sum);
        }
    }
}

/// Returns the lowercase SHA-256 digest of a byte slice.
pub fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(bytes);
    hash.finish()
}

/// Streams a file and returns its lowercase SHA-256 digest.
pub fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        match file.read(&mut buffer) {
            Ok(0) => return Ok(hash.finish()),
            Ok(count) => hash.update(&buffer[..count]),
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nist_sha256_vectors_with_single_and_split_updates() {
        for (message, expected) in [
            (&b""[..], "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
            (&b"abc"[..], "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
            (&b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"[..], "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"),
            (&b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu"[..], "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1"),
        ] {
            assert_eq!(sha256_bytes(message), expected);
            for chunk_size in [1, 3, 55, 56, 63, 64, 65] {
                let mut hash = Sha256::new();
                for chunk in message.chunks(chunk_size) {
                    hash.update(chunk);
                    hash.update(&[]);
                }
                assert_eq!(hash.finish(), expected, "chunk size {chunk_size}");
            }
        }
    }

    #[test]
    fn nist_million_a_vector_streamed_from_file() {
        let root = crate::tests::temp_dir("sha256");
        let path = root.join("million-a");
        std::fs::write(&path, vec![b'a'; 1_000_000]).unwrap();
        assert_eq!(
            sha256_file(&path).unwrap(),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
        assert_eq!(
            sha256_file(&root.join("missing")).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
