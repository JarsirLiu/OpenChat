use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Debug)]
pub struct ResourceTokenService {
    secret: Vec<u8>,
}

impl ResourceTokenService {
    pub fn new(secret: impl AsRef<[u8]>) -> Self {
        Self {
            secret: secret.as_ref().to_vec(),
        }
    }

    pub fn sign(&self, resource_kind: &str, resource_id: &str, ttl_seconds: i64) -> String {
        let expires_at = current_unix_seconds().saturating_add(ttl_seconds);
        let signature = self.signature(resource_kind, resource_id, expires_at);
        format!("{expires_at}.{signature}")
    }

    pub fn verify(&self, resource_kind: &str, resource_id: &str, token: &str) -> bool {
        let Some((expires_at_raw, signature)) = token.split_once('.') else {
            return false;
        };
        let Ok(expires_at) = expires_at_raw.parse::<i64>() else {
            return false;
        };
        if expires_at < current_unix_seconds() {
            return false;
        }

        let Ok(decoded) = URL_SAFE_NO_PAD.decode(signature) else {
            return false;
        };
        let mut mac = self.new_mac();
        mac.update(b"openchat:v2:");
        mac.update(resource_kind.as_bytes());
        mac.update(b":");
        mac.update(resource_id.as_bytes());
        mac.update(b":");
        mac.update(expires_at_raw.as_bytes());
        mac.verify_slice(decoded.as_slice()).is_ok()
    }

    fn signature(&self, resource_kind: &str, resource_id: &str, expires_at: i64) -> String {
        let mut mac = self.new_mac();
        mac.update(b"openchat:v2:");
        mac.update(resource_kind.as_bytes());
        mac.update(b":");
        mac.update(resource_id.as_bytes());
        mac.update(b":");
        mac.update(expires_at.to_string().as_bytes());
        URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
    }

    fn new_mac(&self) -> HmacSha256 {
        HmacSha256::new_from_slice(&self.secret).expect("HMAC can accept arbitrary key lengths")
    }
}

fn current_unix_seconds() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .try_into()
        .unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::ResourceTokenService;

    #[test]
    fn verifies_valid_token() {
        let signer = ResourceTokenService::new("test-secret");
        let token = signer.sign("media", "uploads/users/user_1/file.png", 60);
        assert!(signer.verify("media", "uploads/users/user_1/file.png", token.as_str()));
    }

    #[test]
    fn rejects_token_for_another_resource() {
        let signer = ResourceTokenService::new("test-secret");
        let token = signer.sign("media", "uploads/users/user_1/file.png", 60);
        assert!(!signer.verify("media", "uploads/users/user_2/file.png", token.as_str()));
    }

    #[test]
    fn rejects_expired_token() {
        let signer = ResourceTokenService::new("test-secret");
        let token = signer.sign("media", "uploads/users/user_1/file.png", -1);
        assert!(!signer.verify("media", "uploads/users/user_1/file.png", token.as_str()));
    }
}
