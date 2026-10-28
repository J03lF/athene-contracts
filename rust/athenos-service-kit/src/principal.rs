//! Identity once at the edge (Q-009 full fix; BE-07, SEC-03).
//!
//! athenos-api resolves the caller's user for every authenticated request.
//! Before 3.6 the core then resolved the same user a second time. Now the
//! gateway forwards what it resolved, signed, and the core takes it instead
//! of asking auth-service again:
//!
//! - The gateway **strips** every client-sent `x-athenos-principal*` header
//!   ([`StripPrincipalLayer`], before any handler runs). After its own
//!   resolution it adds ([`PrincipalSigner`]):
//!   - `x-athenos-principal`: base64url (no padding) of the JSON
//!     `{"v":1,"iat":<unix s>,"exp":<unix s, at most 30 s later>,
//!     "tok":<hex SHA-256 of the bearer token>,"user":<the user>}`;
//!   - `x-athenos-principal-sig`: `v1=<base64url HMAC-SHA256 over the exact
//!     header value>`.
//! - The key ([`PrincipalKey`]) is derived from a configured principal key
//!   or, without one, from the internal service token both sides already
//!   share (`HMAC-SHA256(secret, "athenos-principal-key/v1")`, so the token
//!   itself never keys a MAC directly).
//! - The core ([`PrincipalGate`]) trusts it only on a request that passed
//!   its service-token check ([`crate::AuthenticatedService`]), with a valid
//!   signature, `iat <= now + 5 s`, `exp > now`, `exp - iat <= 30 s`, and
//!   `tok` equal to the SHA-256 of that request's own bearer token — a
//!   principal cannot be replayed with another token, and logout/refresh
//!   keep evicting by token hash. Anything else falls back to the core's
//!   own lookup (logged with the reason).
//! - The bearer token keeps travelling: the core still needs it for the
//!   calls it forwards to auth-service.

use std::fmt;
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use hmac::{Hmac, Mac};
use http::header::{HeaderName, HeaderValue};
use http::{Extensions, HeaderMap, Request};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tower::{Layer, Service};

use crate::config::Secret;
use crate::rate_limit::hex;
use crate::service_auth::{constant_time_eq, AuthenticatedService};

type HmacSha256 = Hmac<Sha256>;

/// Prefix of every reserved principal header.
pub const PRINCIPAL_HEADER_PREFIX: &str = "x-athenos-principal";
/// The encoded principal.
pub const PRINCIPAL_HEADER: &str = "x-athenos-principal";
/// The principal's signature.
pub const PRINCIPAL_SIGNATURE_HEADER: &str = "x-athenos-principal-sig";
/// Upper bound on `exp - iat`.
pub const MAX_LIFETIME: Duration = Duration::from_secs(30);
/// How far `iat` may lie in the future (clock skew between the services).
pub const CLOCK_SKEW: Duration = Duration::from_secs(5);

const VERSION: u8 = 1;
const SIGNATURE_PREFIX: &str = "v1=";
const KEY_LABEL: &[u8] = b"athenos-principal-key/v1";
/// Upper bound on the encoded header, far above any real user.
const MAX_HEADER_LEN: usize = 8 * 1024;

/// Whether a header name is one only the gateway may set.
pub fn is_reserved(name: &str) -> bool {
    name.get(..PRINCIPAL_HEADER_PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(PRINCIPAL_HEADER_PREFIX))
}

/// Removes every reserved header from `headers`; how many there were.
pub fn strip_reserved(headers: &mut HeaderMap) -> usize {
    let reserved: Vec<HeaderName> = headers
        .keys()
        .filter(|name| is_reserved(name.as_str()))
        .cloned()
        .collect();
    let mut removed = 0;
    for name in reserved {
        removed += headers.get_all(&name).iter().count();
        headers.remove(name);
    }
    removed
}

/// Hex SHA-256 of a bearer token, as `tok` carries it.
pub fn token_hash(token: &str) -> String {
    hex(&Sha256::digest(token.trim().as_bytes()))
}

/// The MAC key. `Debug` never shows it.
#[derive(Clone)]
pub struct PrincipalKey([u8; 32]);

impl fmt::Debug for PrincipalKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PrincipalKey(<redacted>)")
    }
}

impl PrincipalKey {
    /// The key derived from `secret`.
    pub fn derive(secret: &Secret) -> Self {
        let mut mac = hmac_with(secret.expose().as_bytes());
        mac.update(KEY_LABEL);
        Self(mac.finalize().into_bytes().into())
    }

    /// The configured principal key, else one derived from the internal
    /// service token; `None` without either (no principal is signed or
    /// trusted then, and the core keeps resolving users itself).
    pub fn resolve(explicit: Option<&Secret>, service_token: Option<&Secret>) -> Option<Self> {
        explicit.or(service_token).map(Self::derive)
    }

    fn mac(&self) -> HmacSha256 {
        hmac_with(&self.0)
    }
}

#[expect(
    clippy::expect_used,
    reason = "HMAC takes keys of any length: new_from_slice never fails"
)]
fn hmac_with(key: &[u8]) -> HmacSha256 {
    HmacSha256::new_from_slice(key).expect("HMAC accepts any key length")
}

#[derive(Serialize, Deserialize)]
struct Claims<U> {
    v: u8,
    iat: u64,
    exp: u64,
    tok: String,
    user: U,
}

/// A principal could not be signed.
#[derive(Debug, thiserror::Error)]
pub enum SignError {
    #[error("the principal cannot be encoded: {0}")]
    Encode(String),
    #[error("the clock is before 1970")]
    Clock,
}

/// The two headers the gateway adds.
#[derive(Debug, Clone)]
pub struct SignedPrincipal {
    pub principal: HeaderValue,
    pub signature: HeaderValue,
}

impl SignedPrincipal {
    /// Replaces whatever principal headers `headers` has with these.
    pub fn apply(&self, headers: &mut HeaderMap) {
        strip_reserved(headers);
        headers.insert(
            HeaderName::from_static(PRINCIPAL_HEADER),
            self.principal.clone(),
        );
        headers.insert(
            HeaderName::from_static(PRINCIPAL_SIGNATURE_HEADER),
            self.signature.clone(),
        );
    }
}

/// Signs principals (the gateway's side).
#[derive(Debug, Clone)]
pub struct PrincipalSigner {
    key: PrincipalKey,
    lifetime: Duration,
}

impl PrincipalSigner {
    pub fn new(key: PrincipalKey) -> Self {
        Self {
            key,
            lifetime: MAX_LIFETIME,
        }
    }

    /// A shorter lifetime (capped at [`MAX_LIFETIME`], at least a second).
    #[must_use]
    pub fn with_lifetime(mut self, lifetime: Duration) -> Self {
        self.lifetime = lifetime.clamp(Duration::from_secs(1), MAX_LIFETIME);
        self
    }

    /// `user`, resolved for the request bearing `token`, signed at `now`.
    pub fn sign<U: Serialize>(
        &self,
        user: &U,
        token: &str,
        now: SystemTime,
    ) -> Result<SignedPrincipal, SignError> {
        let iat = unix_seconds(now).ok_or(SignError::Clock)?;
        let claims = Claims {
            v: VERSION,
            iat,
            exp: iat.saturating_add(self.lifetime.as_secs()),
            tok: token_hash(token),
            user,
        };
        let json = serde_json::to_vec(&claims).map_err(|err| SignError::Encode(err.to_string()))?;
        let value = URL_SAFE_NO_PAD.encode(json);
        let mut mac = self.key.mac();
        mac.update(value.as_bytes());
        let signature = format!(
            "{SIGNATURE_PREFIX}{}",
            URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
        );
        Ok(SignedPrincipal {
            principal: HeaderValue::from_str(&value)
                .map_err(|err| SignError::Encode(err.to_string()))?,
            signature: HeaderValue::from_str(&signature)
                .map_err(|err| SignError::Encode(err.to_string()))?,
        })
    }
}

/// Why a principal was not taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalRejection {
    /// No principal header (the request simply did not carry one).
    Missing,
    /// The request did not pass the service-token check.
    UntrustedCaller,
    /// This side has no key to check a signature with.
    NoKey,
    Malformed,
    BadSignature,
    UnsupportedVersion,
    /// `exp - iat` is negative or above [`MAX_LIFETIME`].
    BadLifetime,
    NotYetValid,
    Expired,
    /// Signed for a different bearer token.
    TokenMismatch,
}

impl PrincipalRejection {
    pub fn reason(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::UntrustedCaller => "untrusted_caller",
            Self::NoKey => "no_key",
            Self::Malformed => "malformed",
            Self::BadSignature => "bad_signature",
            Self::UnsupportedVersion => "unsupported_version",
            Self::BadLifetime => "bad_lifetime",
            Self::NotYetValid => "not_yet_valid",
            Self::Expired => "expired",
            Self::TokenMismatch => "token_mismatch",
        }
    }
}

/// Verifies principals (the core's side).
#[derive(Debug, Clone)]
pub struct PrincipalVerifier {
    key: PrincipalKey,
}

impl PrincipalVerifier {
    pub fn new(key: PrincipalKey) -> Self {
        Self { key }
    }

    /// The user in `headers`, if it is signed with this key, current, and
    /// bound to `token` (the request's own bearer token).
    pub fn verify<U: DeserializeOwned>(
        &self,
        headers: &HeaderMap,
        token: &str,
        now: SystemTime,
    ) -> Result<U, PrincipalRejection> {
        let value = single(headers, PRINCIPAL_HEADER)?.ok_or(PrincipalRejection::Missing)?;
        let signature =
            single(headers, PRINCIPAL_SIGNATURE_HEADER)?.ok_or(PrincipalRejection::Malformed)?;
        if value.len() > MAX_HEADER_LEN {
            return Err(PrincipalRejection::Malformed);
        }
        let signature = signature
            .strip_prefix(SIGNATURE_PREFIX)
            .ok_or(PrincipalRejection::Malformed)?;
        let signature = URL_SAFE_NO_PAD
            .decode(signature)
            .map_err(|_| PrincipalRejection::Malformed)?;
        let mut mac = self.key.mac();
        mac.update(value.as_bytes());
        mac.verify_slice(&signature)
            .map_err(|_| PrincipalRejection::BadSignature)?;

        let json = URL_SAFE_NO_PAD
            .decode(value)
            .map_err(|_| PrincipalRejection::Malformed)?;
        let claims: Claims<serde_json::Value> =
            serde_json::from_slice(&json).map_err(|_| PrincipalRejection::Malformed)?;
        if claims.v != VERSION {
            return Err(PrincipalRejection::UnsupportedVersion);
        }
        let lifetime = claims
            .exp
            .checked_sub(claims.iat)
            .ok_or(PrincipalRejection::BadLifetime)?;
        if lifetime > MAX_LIFETIME.as_secs() {
            return Err(PrincipalRejection::BadLifetime);
        }
        let now = unix_seconds(now).ok_or(PrincipalRejection::Expired)?;
        if claims.iat > now.saturating_add(CLOCK_SKEW.as_secs()) {
            return Err(PrincipalRejection::NotYetValid);
        }
        if claims.exp <= now {
            return Err(PrincipalRejection::Expired);
        }
        if !constant_time_eq(claims.tok.as_bytes(), token_hash(token).as_bytes()) {
            return Err(PrincipalRejection::TokenMismatch);
        }
        serde_json::from_value(claims.user).map_err(|_| PrincipalRejection::Malformed)
    }
}

/// The one value of header `name`, if any; two values are malformed.
fn single<'a>(headers: &'a HeaderMap, name: &str) -> Result<Option<&'a str>, PrincipalRejection> {
    let mut values = headers.get_all(name).iter();
    let Some(first) = values.next() else {
        return Ok(None);
    };
    if values.next().is_some() {
        return Err(PrincipalRejection::Malformed);
    }
    first
        .to_str()
        .map(Some)
        .map_err(|_| PrincipalRejection::Malformed)
}

fn unix_seconds(now: SystemTime) -> Option<u64> {
    now.duration_since(UNIX_EPOCH)
        .ok()
        .map(|since| since.as_secs())
}

/// The core's decision whether to take a forwarded principal.
#[derive(Debug, Clone, Default)]
pub struct PrincipalGate {
    verifier: Option<PrincipalVerifier>,
}

impl PrincipalGate {
    pub fn new(key: Option<PrincipalKey>) -> Self {
        Self {
            verifier: key.map(PrincipalVerifier::new),
        }
    }

    /// Whether principals can be taken at all (a key is configured).
    pub fn enabled(&self) -> bool {
        self.verifier.is_some()
    }

    /// The forwarded user of a request bearing `token`, when it may be
    /// trusted; `Err` says why not. A request with no principal header is
    /// `Err(Missing)`; every other refusal is logged.
    pub fn check<U: DeserializeOwned>(
        &self,
        headers: &HeaderMap,
        extensions: &Extensions,
        token: &str,
    ) -> Result<U, PrincipalRejection> {
        if !headers.contains_key(PRINCIPAL_HEADER) {
            return Err(PrincipalRejection::Missing);
        }
        let result = if extensions.get::<AuthenticatedService>().is_none() {
            Err(PrincipalRejection::UntrustedCaller)
        } else {
            match &self.verifier {
                Some(verifier) => verifier.verify(headers, token, SystemTime::now()),
                None => Err(PrincipalRejection::NoKey),
            }
        };
        if let Err(rejection) = &result {
            tracing::warn!(
                event = "security.principal.rejected",
                reason = rejection.reason(),
                "forwarded principal not taken; resolving the user again"
            );
        }
        result
    }

    /// [`Self::check`] as an `Option`, for a caller that falls back.
    pub fn accept<U: DeserializeOwned>(
        &self,
        headers: &HeaderMap,
        extensions: &Extensions,
        token: &str,
    ) -> Option<U> {
        self.check(headers, extensions, token).ok()
    }
}

/// Removes every client-sent `x-athenos-principal*` header (the gateway's
/// outermost layer).
#[derive(Debug, Clone, Copy, Default)]
pub struct StripPrincipalLayer;

impl<S> Layer<S> for StripPrincipalLayer {
    type Service = StripPrincipalService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        StripPrincipalService { inner }
    }
}

#[derive(Debug, Clone)]
pub struct StripPrincipalService<S> {
    inner: S,
}

impl<S, B> Service<Request<B>> for StripPrincipalService<S>
where
    S: Service<Request<B>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: Request<B>) -> Self::Future {
        let removed = strip_reserved(request.headers_mut());
        if removed > 0 {
            tracing::warn!(
                event = "security.principal.stripped",
                count = removed,
                "client sent a reserved principal header; dropped"
            );
        }
        self.inner.call(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct User {
        id: String,
        role: String,
    }

    const TOKEN: &str = "abcdefghijklmnopqrstuvwxyz0123456789";

    fn user() -> User {
        User {
            id: "u-1".to_string(),
            role: "admin".to_string(),
        }
    }

    fn key(secret: &str) -> PrincipalKey {
        PrincipalKey::derive(&Secret::new(secret))
    }

    fn signed(key: &PrincipalKey, token: &str, at: SystemTime) -> HeaderMap {
        let mut headers = HeaderMap::new();
        PrincipalSigner::new(key.clone())
            .sign(&user(), token, at)
            .unwrap()
            .apply(&mut headers);
        headers
    }

    #[test]
    fn a_signed_principal_verifies_for_its_own_token() {
        let key = key("shared-service-token-0123456789abcdef");
        let now = SystemTime::now();
        let headers = signed(&key, TOKEN, now);
        let verifier = PrincipalVerifier::new(key);
        assert_eq!(verifier.verify::<User>(&headers, TOKEN, now), Ok(user()));
        // Still valid 29 s later, no longer at 30 s.
        let later = now + Duration::from_secs(29);
        assert!(verifier.verify::<User>(&headers, TOKEN, later).is_ok());
        let expired = now + Duration::from_secs(30);
        assert_eq!(
            verifier.verify::<User>(&headers, TOKEN, expired),
            Err(PrincipalRejection::Expired)
        );
    }

    #[test]
    fn forged_mismatched_and_malformed_principals_are_refused() {
        let key = key("shared-service-token-0123456789abcdef");
        let now = SystemTime::now();
        let verifier = PrincipalVerifier::new(key.clone());

        // Another key.
        let forged = signed(
            &self::key("another-secret-0123456789abcdef0123"),
            TOKEN,
            now,
        );
        assert_eq!(
            verifier.verify::<User>(&forged, TOKEN, now),
            Err(PrincipalRejection::BadSignature)
        );

        // Another token.
        let headers = signed(&key, TOKEN, now);
        assert_eq!(
            verifier.verify::<User>(&headers, "zyxwvutsrqponmlkjihgfedcba9876543210", now),
            Err(PrincipalRejection::TokenMismatch)
        );

        // A tampered payload under the old signature.
        let mut tampered = headers.clone();
        let value = tampered[PRINCIPAL_HEADER].to_str().unwrap().to_string();
        let mut json: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(&value).unwrap()).unwrap();
        json["user"]["role"] = "guest".into();
        let forged_value = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&json).unwrap());
        tampered.insert(
            PRINCIPAL_HEADER,
            HeaderValue::from_str(&forged_value).unwrap(),
        );
        assert_eq!(
            verifier.verify::<User>(&tampered, TOKEN, now),
            Err(PrincipalRejection::BadSignature)
        );

        // No signature, a garbled one, two principals.
        let mut unsigned = headers.clone();
        unsigned.remove(PRINCIPAL_SIGNATURE_HEADER);
        assert_eq!(
            verifier.verify::<User>(&unsigned, TOKEN, now),
            Err(PrincipalRejection::Malformed)
        );
        let mut garbled = headers.clone();
        garbled.insert(
            PRINCIPAL_SIGNATURE_HEADER,
            HeaderValue::from_static("v2=abc"),
        );
        assert_eq!(
            verifier.verify::<User>(&garbled, TOKEN, now),
            Err(PrincipalRejection::Malformed)
        );
        let mut doubled = headers.clone();
        doubled.append(PRINCIPAL_HEADER, HeaderValue::from_static("eyJ2IjoxfQ"));
        assert_eq!(
            verifier.verify::<User>(&doubled, TOKEN, now),
            Err(PrincipalRejection::Malformed)
        );
        assert_eq!(
            verifier.verify::<User>(&HeaderMap::new(), TOKEN, now),
            Err(PrincipalRejection::Missing)
        );
    }

    /// A correctly signed principal whose claims break the rules: the
    /// signer itself never produces these, a holder of the key might.
    fn sign_raw(key: &PrincipalKey, claims: serde_json::Value) -> HeaderMap {
        let value = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap());
        let mut mac = key.mac();
        mac.update(value.as_bytes());
        let signature = format!("v1={}", URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()));
        let mut headers = HeaderMap::new();
        headers.insert(PRINCIPAL_HEADER, HeaderValue::from_str(&value).unwrap());
        headers.insert(
            PRINCIPAL_SIGNATURE_HEADER,
            HeaderValue::from_str(&signature).unwrap(),
        );
        headers
    }

    #[test]
    fn lifetimes_versions_and_clocks_are_checked() {
        let key = key("shared-service-token-0123456789abcdef");
        let verifier = PrincipalVerifier::new(key.clone());
        let now = SystemTime::now();
        let at = unix_seconds(now).unwrap();
        let claims = |v: u8, iat: u64, exp: u64| serde_json::json!({ "v": v, "iat": iat, "exp": exp, "tok": token_hash(TOKEN), "user": user() });
        let check = |claims| verifier.verify::<User>(&sign_raw(&key, claims), TOKEN, now);
        assert_eq!(check(claims(1, at, at + 30)), Ok(user()));
        assert_eq!(
            check(claims(1, at, at + 31)),
            Err(PrincipalRejection::BadLifetime)
        );
        assert_eq!(
            check(claims(1, at, at - 1)),
            Err(PrincipalRejection::BadLifetime)
        );
        assert_eq!(
            check(claims(2, at, at + 10)),
            Err(PrincipalRejection::UnsupportedVersion)
        );
        assert_eq!(
            check(claims(1, at + 60, at + 70)),
            Err(PrincipalRejection::NotYetValid)
        );
        assert_eq!(
            check(claims(1, at - 40, at - 10)),
            Err(PrincipalRejection::Expired)
        );
    }

    #[test]
    fn the_key_comes_from_the_principal_key_or_the_service_token() {
        let token = Secret::new("shared-service-token-0123456789abcdef");
        let explicit = Secret::new("explicit-principal-key-0123456789ab");
        assert!(PrincipalKey::resolve(None, None).is_none());
        let from_token = PrincipalKey::resolve(None, Some(&token)).unwrap();
        let from_explicit = PrincipalKey::resolve(Some(&explicit), Some(&token)).unwrap();
        assert_ne!(from_token.0, from_explicit.0);
        // The derived key is not the token itself.
        assert_ne!(&from_token.0[..], &token.expose().as_bytes()[..32]);
        assert!(!format!("{from_token:?}").contains("shared"));
    }

    #[test]
    fn the_gate_trusts_only_verified_service_calls() {
        let key = key("shared-service-token-0123456789abcdef");
        let headers = signed(&key, TOKEN, SystemTime::now());
        let gate = PrincipalGate::new(Some(key));
        let mut verified = Extensions::new();
        verified.insert(AuthenticatedService);
        assert_eq!(gate.check::<User>(&headers, &verified, TOKEN), Ok(user()));
        assert_eq!(
            gate.check::<User>(&headers, &Extensions::new(), TOKEN),
            Err(PrincipalRejection::UntrustedCaller)
        );
        assert_eq!(
            PrincipalGate::new(None).check::<User>(&headers, &verified, TOKEN),
            Err(PrincipalRejection::NoKey)
        );
        assert_eq!(
            gate.check::<User>(&HeaderMap::new(), &verified, TOKEN),
            Err(PrincipalRejection::Missing)
        );
    }

    #[test]
    fn reserves_the_principal_prefix_only() {
        for name in [
            "x-athenos-principal",
            "x-athenos-principal-sig",
            "X-Athenos-Principal-Anything",
        ] {
            assert!(is_reserved(name), "{name}");
        }
        for name in [
            "x-athenos-user-token",
            "authorization",
            "x-athenos",
            "x-principal",
        ] {
            assert!(!is_reserved(name), "{name}");
        }
    }

    #[tokio::test]
    async fn the_strip_layer_drops_client_principals() {
        let service =
            StripPrincipalLayer.layer(tower::service_fn(|request: Request<()>| async move {
                Ok::<_, std::convert::Infallible>(
                    request
                        .headers()
                        .keys()
                        .map(|name| name.as_str().to_string())
                        .collect::<Vec<_>>(),
                )
            }));
        let mut request = Request::new(());
        request
            .headers_mut()
            .insert(PRINCIPAL_HEADER, HeaderValue::from_static("forged"));
        request.headers_mut().insert(
            "x-athenos-principal-sig",
            HeaderValue::from_static("v1=forged"),
        );
        request
            .headers_mut()
            .insert("authorization", HeaderValue::from_static("Bearer x"));
        let seen = service.oneshot(request).await.unwrap();
        assert_eq!(seen, vec!["authorization".to_string()]);
    }
}
