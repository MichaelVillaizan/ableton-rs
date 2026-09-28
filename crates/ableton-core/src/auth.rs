//! Password authentication with the Remote Script.

use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Hex-encoded HMAC-SHA256 of `salt` keyed with `password`, as expected by
/// `internal.authenticate` in `Socket.py`.
pub(crate) fn hmac_sha256_hex(password: &str, salt: &str) -> String {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(password.as_bytes()).expect("HMAC accepts keys of any size");
    mac.update(salt.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_python_hmac_for_a_short_key() {
        assert_eq!(
            hmac_sha256_hex("secret", "abc123"),
            "5ae5ac802a1a5c94fb683e1bfa121f9f700a26995213ff2fc1c503eb43ec71c6"
        );
    }

    #[test]
    fn hashes_keys_longer_than_one_block() {
        assert_eq!(
            hmac_sha256_hex(&"x".repeat(80), "msg"),
            "a4c011685c0698d5626fd453a2073673550d6b4f9e01b8693a4ace5f6920d651"
        );
    }
}
