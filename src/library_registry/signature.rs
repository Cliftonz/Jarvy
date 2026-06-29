//! Cosign signature verification for library manifests (PRD-054 phase 5).
//!
//! When a `LibrarySource` declares `require_signature = true` (the
//! default), every manifest fetch tries to also fetch its companion
//! `.sig` + `.pem` files and run `cosign verify-blob` against the
//! `identity_regexp` + `oidc_issuer` declared in the source config.
//!
//! Behaviour matrix:
//!
//! | `require_signature` | cosign on PATH | sigs available | verify result | outcome |
//! |---|---|---|---|---|
//! | true | no | n/a | n/a | refuse (CosignMissing) |
//! | true | yes | no | n/a | refuse (SignatureCompanionsMissing) |
//! | true | yes | yes | reject | refuse (SignatureRejected) |
//! | true | yes | yes | verify | proceed |
//! | false | * | * | * | proceed, emit `library.signature_disabled` warning (handled in mod.rs) |
//!
//! The actual cosign subprocess lives in `crate::update::signature`
//! (PRD-012). We feed it three temp files (`manifest.json`, `.sig`,
//! `.pem`) so the existing implementation works without modification.

use super::{LibraryError, fetch};
use std::io::Write;
use std::path::Path;

/// Default identity-regexp / oidc-issuer used when the library source
/// doesn't override them. Empty regexp means "no enforcement of the
/// signing identity" — we still verify the signature itself but accept
/// any identity. Publishers SHOULD set `identity_regexp` to pin their
/// signing repo (e.g. `^https://github\.com/myorg/jarvy-library/.+$`).
const DEFAULT_OIDC_ISSUER: &str = "https://token.actions.githubusercontent.com";

/// Verify a fetched manifest's cosign signature. Returns Ok(()) when
/// verification succeeds OR when `require_signature` is false (the
/// caller decides whether to warn). Returns a structured `LibraryError`
/// when verification is required but fails.
///
/// `manifest_body` is the raw fetched bytes — they're written to a
/// temp file so the cosign subprocess can verify by path.
pub fn verify_manifest_signature(
    manifest_url: &str,
    manifest_body: &[u8],
    require_signature: bool,
    identity_regexp: Option<&str>,
    oidc_issuer: Option<&str>,
) -> Result<(), LibraryError> {
    if !require_signature {
        return Ok(());
    }

    // Materialize the three files cosign needs in a single tempdir.
    let tmpdir = tempfile::tempdir().map_err(LibraryError::Io)?;
    let manifest_path = tmpdir.path().join("manifest.json");
    std::fs::write(&manifest_path, manifest_body).map_err(LibraryError::Io)?;

    // Companion fetches. Both are required when `require_signature` is
    // on; missing-companion is a refusal.
    let sig_url = format!("{}.sig", manifest_url);
    let cert_url = format!("{}.pem", manifest_url);
    let sig_bytes = match fetch::fetch_bounded(&sig_url, fetch::MAX_ITEM_BYTES) {
        Ok(b) => b,
        Err(e) => {
            return Err(LibraryError::SignatureCompanionsMissing {
                url: crate::network::redact_credentials(manifest_url).into_owned(),
                reason: format!("could not fetch {sig_url}: {e}"),
            });
        }
    };
    let cert_bytes = match fetch::fetch_bounded(&cert_url, fetch::MAX_ITEM_BYTES) {
        Ok(b) => b,
        Err(e) => {
            return Err(LibraryError::SignatureCompanionsMissing {
                url: crate::network::redact_credentials(manifest_url).into_owned(),
                reason: format!("could not fetch {cert_url}: {e}"),
            });
        }
    };

    // `verify_sigstore_signature_with_identity` derives sig/cert paths
    // from the file's extension (`manifest.json` → `manifest.json.sig`
    // + `manifest.json.pem`). Write the companions next to the manifest.
    let sig_path = manifest_path.with_extension("json.sig");
    let cert_path = manifest_path.with_extension("json.pem");
    write_atomic(&sig_path, &sig_bytes)?;
    write_atomic(&cert_path, &cert_bytes)?;

    // Default identity-regexp accepts anything when the publisher
    // didn't pin one. Documented as "trust the issuer + signature
    // only" — strong publishers MUST pin identity_regexp.
    let identity = identity_regexp.unwrap_or(".*");
    let issuer = oidc_issuer.unwrap_or(DEFAULT_OIDC_ISSUER);

    match crate::update::signature::verify_sigstore_signature_with_identity(
        &manifest_path,
        identity,
        issuer,
    ) {
        Ok(crate::update::signature::SignatureOutcome::Verified) => Ok(()),
        Ok(crate::update::signature::SignatureOutcome::CosignMissing) => {
            Err(LibraryError::CosignMissing {
                url: crate::network::redact_credentials(manifest_url).into_owned(),
            })
        }
        Ok(crate::update::signature::SignatureOutcome::SignatureFilesMissing) => {
            Err(LibraryError::SignatureCompanionsMissing {
                url: crate::network::redact_credentials(manifest_url).into_owned(),
                reason: "cosign reports sig/cert files missing after temp write".to_string(),
            })
        }
        Ok(crate::update::signature::SignatureOutcome::Rejected(stderr)) => {
            Err(LibraryError::SignatureRejected {
                url: crate::network::redact_credentials(manifest_url).into_owned(),
                reason: stderr,
            })
        }
        Err(e) => Err(LibraryError::SignatureRejected {
            url: crate::network::redact_credentials(manifest_url).into_owned(),
            reason: e.to_string(),
        }),
    }
}

fn write_atomic(path: &Path, content: &[u8]) -> Result<(), LibraryError> {
    let mut f = std::fs::File::create(path).map_err(LibraryError::Io)?;
    f.write_all(content).map_err(LibraryError::Io)?;
    f.flush().map_err(LibraryError::Io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `require_signature = false` is the documented escape hatch —
    /// must return Ok regardless of cosign / sig availability.
    #[test]
    fn require_signature_false_short_circuits() {
        let res = verify_manifest_signature(
            "https://cdn.example/lib/manifest.json",
            b"{}",
            false,
            None,
            None,
        );
        assert!(res.is_ok());
    }
}
