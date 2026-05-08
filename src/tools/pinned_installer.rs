//! Hardened `curl|bash` pipeline for installer scripts that don't have a
//! Homebrew formula or distro package.
//!
//! Mirrors the pattern in `src/tools/brew/definition.rs`: pull a script at a
//! pinned commit, verify its sha256 matches the constant we ship, and only
//! then pipe it into a shell. A compromise of the upstream branch tip cannot
//! silently land RCE because the hash mismatch aborts before any code from
//! the third-party repo runs.
//!
//! To update a pinned installer, pick a commit, download the script at that
//! commit, compute its sha256, and update both constants together.

/// A single pinned installer script that we can fetch-and-verify.
pub struct PinnedInstaller<'a> {
    /// Display name used in log messages and refusal errors.
    pub name: &'a str,
    /// Full URL to the raw installer script at a specific commit. MUST NOT
    /// reference a moving ref like `main`/`master`/`HEAD`.
    pub url: &'a str,
    /// Lowercase 64-char hex sha256 of the script body.
    pub sha256: &'a str,
}

impl PinnedInstaller<'_> {
    /// Build the bash one-liner that fetches the pinned installer, verifies
    /// its sha256 matches the constant we ship, and only then pipes it into
    /// `/bin/bash`. The mismatch path aborts before any third-party code
    /// runs, so a compromise of the upstream tip cannot silently land RCE.
    pub fn shell_command(&self) -> String {
        format!(
            r#"set -euo pipefail
SCRIPT="$(curl -fsSL '{url}')"
ACTUAL=$(printf '%s' "$SCRIPT" | shasum -a 256 | cut -d' ' -f1)
EXPECTED='{expected}'
if [ "$ACTUAL" != "$EXPECTED" ]; then
  printf 'jarvy: refusing to run %s installer; sha256 mismatch (got %s, want %s)\n' \
      '{name}' "$ACTUAL" "$EXPECTED" >&2
  exit 1
fi
printf '%s' "$SCRIPT" | /bin/bash -s --
"#,
            url = self.url,
            expected = self.sha256,
            name = self.name,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> PinnedInstaller<'static> {
        PinnedInstaller {
            name: "demo",
            url: "https://raw.githubusercontent.com/example/repo/abc123/install.sh",
            sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        }
    }

    #[test]
    fn shell_command_embeds_url_and_hash() {
        let cmd = fixture().shell_command();
        assert!(cmd.contains("abc123/install.sh"));
        assert!(cmd.contains(fixture().sha256));
        assert!(cmd.contains("demo"));
    }

    #[test]
    fn shell_command_aborts_on_mismatch_before_exec() {
        let cmd = fixture().shell_command();
        // The exit-1 branch must come before the pipe-to-bash line so a
        // mismatched script never reaches `/bin/bash`.
        let exit_pos = cmd.find("exit 1").expect("exit 1 must appear");
        let pipe_pos = cmd.find("/bin/bash").expect("bash pipe must appear");
        assert!(
            exit_pos < pipe_pos,
            "sha256-mismatch refusal must short-circuit before bash exec"
        );
    }

    #[test]
    fn shell_command_does_not_use_moving_refs() {
        let cmd = fixture().shell_command();
        assert!(!cmd.contains("/HEAD/"));
        assert!(!cmd.contains("/master/"));
        assert!(!cmd.contains("/main/"));
    }
}
