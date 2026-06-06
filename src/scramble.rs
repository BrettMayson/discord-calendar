use std::sync::OnceLock;

static SECRET: OnceLock<u64> = OnceLock::new();

/// Gets the secret value from the SCRAMBLE_SECRET environment variable,
/// or falls back to the default if not set.
fn get_secret() -> u64 {
    *SECRET.get_or_init(|| {
        std::env::var("SCRAMBLE_SECRET")
            .ok()
            .and_then(|s| u64::from_str_radix(&s, 16).ok())
            .unwrap_or(0xDEADBEEFCAFEBABE)
    })
}

/// Encodes a guild ID in a way that's hard to guess but easy to reverse.
/// Uses XOR with a secret constant and bit rotation.
pub fn encode(id: u64) -> String {
    let secret = get_secret();
    let xored = id ^ secret;
    let rotated = xored.rotate_left(17);
    format!("{:x}", rotated)
}

/// Decodes a scrambled ID back to the original guild ID.
pub fn decode(encoded: &str) -> Option<u64> {
    let secret = get_secret();
    let rotated = u64::from_str_radix(encoded, 16).ok()?;
    let xored = rotated.rotate_right(17);
    let id = xored ^ secret;

    Some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let original_id = 123456789u64;
        let encoded = encode(original_id);
        let decoded = decode(&encoded).expect("should decode");
        assert_eq!(original_id, decoded);
    }

    #[test]
    fn test_encode_produces_hex() {
        let encoded = encode(123456789u64);
        // Should be valid hex
        u64::from_str_radix(&encoded, 16).expect("should be valid hex");
    }
}
