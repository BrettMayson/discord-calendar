use siphasher::sip::SipHasher13;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

static SECRET: OnceLock<u64> = OnceLock::new();

fn get_secret() -> u64 {
    *SECRET.get_or_init(|| {
        std::env::var("SCRAMBLE_SECRET")
            .ok()
            .and_then(|s| u64::from_str_radix(&s, 16).ok())
            .unwrap_or(0xDEADBEEFCAFEBABE)
    })
}

const ROUNDS: usize = 8;

fn round_function(secret: u64, round: usize, value: u32) -> u32 {
    let mut hasher = SipHasher13::new_with_keys(secret, round as u64);
    value.hash(&mut hasher);
    (hasher.finish() & 0xFFFF_FFFF) as u32
}

fn feistel_encrypt(value: u64, secret: u64) -> u64 {
    let mut left = (value >> 32) as u32;
    let mut right = value as u32;

    for round in 0..ROUNDS {
        let f = round_function(secret, round, right);

        let new_left = right;
        let new_right = left ^ f;

        left = new_left;
        right = new_right;
    }

    ((left as u64) << 32) | (right as u64)
}

fn feistel_decrypt(value: u64, secret: u64) -> u64 {
    let mut left = (value >> 32) as u32;
    let mut right = value as u32;

    for round in (0..ROUNDS).rev() {
        let f = round_function(secret, round, left);

        let new_right = left;
        let new_left = right ^ f;

        left = new_left;
        right = new_right;
    }

    ((left as u64) << 32) | (right as u64)
}

/// Encodes a guild ID into a deterministic, reversible hex string.
pub fn encode(id: u64) -> String {
    let secret = get_secret();
    let encrypted = feistel_encrypt(id, secret);

    // Fixed-width output avoids leaking leading zeros.
    format!("{:016x}", encrypted)
}

/// Decodes an encoded guild ID back to the original.
pub fn decode(encoded: &str) -> Option<u64> {
    let secret = get_secret();

    let encrypted = u64::from_str_radix(encoded, 16).ok()?;

    Some(feistel_decrypt(encrypted, secret))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let original = 123_456_789_u64;

        let encoded = encode(original);
        let decoded = decode(&encoded).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_deterministic() {
        let id = 123_456_789_u64;

        assert_eq!(encode(id), encode(id));
    }

    #[test]
    fn test_different_inputs() {
        assert_ne!(encode(1), encode(2));
    }
}
