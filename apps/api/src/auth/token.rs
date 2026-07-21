use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const MIN_SECRET_BYTES: usize = 32;
const MAX_ASSERTION_LIFETIME_SECONDS: u64 = 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertionConfig {
    pub secret: String,
    pub issuer: String,
    pub audience: String,
    pub leeway_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssertionClaims {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub provider: String,
    pub tenant: String,
    pub sid: String,
    pub jti: String,
    pub iat: u64,
    pub nbf: u64,
    pub exp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AssertionError {
    #[error("internal assertion secret must contain at least 32 bytes")]
    WeakSecret,
    #[error("internal assertion issuer and audience must not be empty")]
    InvalidBoundary,
    #[error("authorization header must use one Bearer token")]
    InvalidAuthorizationHeader,
    #[error("internal assertion is invalid or expired")]
    InvalidToken,
    #[error("internal assertion contains an empty identity claim")]
    EmptyIdentityClaim,
}

#[derive(Clone)]
pub struct InternalAssertionVerifier {
    decoding_key: DecodingKey,
    validation: Validation,
    leeway_seconds: u64,
}

impl InternalAssertionVerifier {
    pub fn new(config: AssertionConfig) -> Result<Self, AssertionError> {
        if config.secret.len() < MIN_SECRET_BYTES {
            return Err(AssertionError::WeakSecret);
        }
        if config.issuer.trim().is_empty() || config.audience.trim().is_empty() {
            return Err(AssertionError::InvalidBoundary);
        }
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[config.issuer]);
        validation.set_audience(&[config.audience]);
        validation.leeway = config.leeway_seconds;
        validation.validate_nbf = true;
        validation.required_spec_claims = HashSet::from([
            "exp".into(),
            "iat".into(),
            "nbf".into(),
            "iss".into(),
            "aud".into(),
            "sub".into(),
            "jti".into(),
        ]);
        Ok(Self {
            decoding_key: DecodingKey::from_secret(config.secret.as_bytes()),
            validation,
            leeway_seconds: config.leeway_seconds,
        })
    }

    pub fn verify_header(&self, value: &str) -> Result<AssertionClaims, AssertionError> {
        let mut parts = value.split_whitespace();
        let scheme = parts.next();
        let token = parts.next();
        if scheme != Some("Bearer") || token.is_none() || parts.next().is_some() {
            return Err(AssertionError::InvalidAuthorizationHeader);
        }
        self.verify(token.expect("checked bearer token"))
    }

    pub fn verify(&self, token: &str) -> Result<AssertionClaims, AssertionError> {
        let claims = decode::<AssertionClaims>(token, &self.decoding_key, &self.validation)
            .map_err(|_| AssertionError::InvalidToken)?
            .claims;
        if [
            claims.sub.as_str(),
            claims.provider.as_str(),
            claims.tenant.as_str(),
            claims.sid.as_str(),
            claims.jti.as_str(),
        ]
        .iter()
        .any(|value| value.trim().is_empty())
        {
            return Err(AssertionError::EmptyIdentityClaim);
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| AssertionError::InvalidToken)?
            .as_secs();
        if claims.exp.saturating_sub(claims.iat) > MAX_ASSERTION_LIFETIME_SECONDS
            || claims.iat > now.saturating_add(self.leeway_seconds)
        {
            return Err(AssertionError::InvalidToken);
        }
        Ok(claims)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

    use super::*;

    const SECRET: &str = "test-internal-assertion-secret-at-least-32-bytes";

    fn config() -> AssertionConfig {
        AssertionConfig {
            secret: SECRET.into(),
            issuer: "test-web".into(),
            audience: "test-api".into(),
            leeway_seconds: 0,
        }
    }

    fn claims() -> AssertionClaims {
        let now = Utc::now().timestamp() as u64;
        AssertionClaims {
            iss: "test-web".into(),
            aud: "test-api".into(),
            sub: "user-1".into(),
            provider: "clerk".into(),
            tenant: "org-1".into(),
            sid: "session-1".into(),
            jti: "assertion-1".into(),
            iat: now,
            nbf: now,
            exp: now + 60,
        }
    }

    fn token(claims: &AssertionClaims, secret: &str) -> String {
        encode(
            &Header::new(Algorithm::HS256),
            claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap()
    }

    #[test]
    fn verifies_a_short_lived_bound_assertion() {
        let verifier = InternalAssertionVerifier::new(config()).unwrap();
        let claims = claims();
        assert_eq!(verifier.verify(&token(&claims, SECRET)).unwrap(), claims);
    }

    #[test]
    fn rejects_expired_wrong_audience_wrong_secret_and_empty_identity_claims() {
        let verifier = InternalAssertionVerifier::new(config()).unwrap();

        let mut expired = claims();
        expired.exp = (Utc::now().timestamp() - 10) as u64;
        assert!(matches!(
            verifier.verify(&token(&expired, SECRET)),
            Err(AssertionError::InvalidToken)
        ));

        let mut not_yet_valid = claims();
        not_yet_valid.nbf = (Utc::now().timestamp() + 60) as u64;
        assert!(matches!(
            verifier.verify(&token(&not_yet_valid, SECRET)),
            Err(AssertionError::InvalidToken)
        ));

        let mut long_lived = claims();
        long_lived.exp = long_lived.iat + MAX_ASSERTION_LIFETIME_SECONDS + 1;
        assert!(matches!(
            verifier.verify(&token(&long_lived, SECRET)),
            Err(AssertionError::InvalidToken)
        ));

        let mut audience = claims();
        audience.aud = "other-api".into();
        assert!(matches!(
            verifier.verify(&token(&audience, SECRET)),
            Err(AssertionError::InvalidToken)
        ));
        assert!(matches!(
            verifier.verify(&token(
                &claims(),
                "different-secret-at-least-thirty-two-bytes"
            )),
            Err(AssertionError::InvalidToken)
        ));

        let mut empty = claims();
        empty.tenant = " ".into();
        assert!(matches!(
            verifier.verify(&token(&empty, SECRET)),
            Err(AssertionError::EmptyIdentityClaim)
        ));
    }

    #[test]
    fn authorization_header_requires_exactly_one_bearer_token() {
        let verifier = InternalAssertionVerifier::new(config()).unwrap();
        assert!(matches!(
            verifier.verify_header("Basic credentials"),
            Err(AssertionError::InvalidAuthorizationHeader)
        ));
        assert!(matches!(
            verifier.verify_header("Bearer one two"),
            Err(AssertionError::InvalidAuthorizationHeader)
        ));
    }
}
