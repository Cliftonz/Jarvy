//! Shell completion generation for jarvy CLI
//!
//! Generates shell completions for bash, zsh, fish, and PowerShell.
//! Uses clap_complete to generate completions from the CLI definition.

use clap::Command;
use clap_complete::{Shell, generate};
use std::io;

/// Supported shells for completion generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
}

impl std::fmt::Display for CompletionShell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompletionShell::Bash => write!(f, "bash"),
            CompletionShell::Zsh => write!(f, "zsh"),
            CompletionShell::Fish => write!(f, "fish"),
            CompletionShell::PowerShell => write!(f, "powershell"),
            CompletionShell::Elvish => write!(f, "elvish"),
        }
    }
}

impl std::str::FromStr for CompletionShell {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bash" => Ok(CompletionShell::Bash),
            "zsh" => Ok(CompletionShell::Zsh),
            "fish" => Ok(CompletionShell::Fish),
            "powershell" | "pwsh" | "ps1" => Ok(CompletionShell::PowerShell),
            "elvish" => Ok(CompletionShell::Elvish),
            _ => Err(format!(
                "Unknown shell '{}'. Supported: bash, zsh, fish, powershell, elvish",
                s
            )),
        }
    }
}

impl From<CompletionShell> for Shell {
    fn from(shell: CompletionShell) -> Self {
        match shell {
            CompletionShell::Bash => Shell::Bash,
            CompletionShell::Zsh => Shell::Zsh,
            CompletionShell::Fish => Shell::Fish,
            CompletionShell::PowerShell => Shell::PowerShell,
            CompletionShell::Elvish => Shell::Elvish,
        }
    }
}

/// Generate shell completions and write to the provided writer
pub fn generate_completions<W: io::Write>(cmd: &mut Command, shell: CompletionShell, buf: &mut W) {
    let shell: Shell = shell.into();
    generate(shell, cmd, "jarvy", buf);
}

/// Generate shell completions as a string
pub fn generate_completions_string(cmd: &mut Command, shell: CompletionShell) -> String {
    let mut buf = Vec::new();
    generate_completions(cmd, shell, &mut buf);
    String::from_utf8(buf).unwrap_or_else(|_| "# Error generating completions".to_string())
}

/// Get installation instructions for shell completions
pub fn get_install_instructions() -> String {
    r#"Shell Completion Installation
=============================

Bash:
  # Option 1: System-wide (requires root)
  jarvy completions bash | sudo tee /usr/local/etc/bash_completion.d/jarvy > /dev/null

  # Option 2: User-local
  mkdir -p ~/.local/share/bash-completion/completions
  jarvy completions bash > ~/.local/share/bash-completion/completions/jarvy

  # Reload shell or run:
  source ~/.bashrc

Zsh:
  # Create completions directory if needed
  mkdir -p ~/.zsh/completions

  # Generate completions
  jarvy completions zsh > ~/.zsh/completions/_jarvy

  # Add to .zshrc if not present:
  # fpath=(~/.zsh/completions $fpath)
  # autoload -Uz compinit && compinit

  # Reload shell or run:
  source ~/.zshrc

Fish:
  # Generate completions
  jarvy completions fish > ~/.config/fish/completions/jarvy.fish

  # Completions will be available in new shell sessions

PowerShell:
  # Add to your PowerShell profile
  jarvy completions powershell >> $PROFILE

  # Or create a separate file and dot-source it
  jarvy completions powershell > ~/.config/powershell/jarvy.ps1
  # Add to $PROFILE: . ~/.config/powershell/jarvy.ps1

Elvish:
  # Generate completions
  jarvy completions elvish > ~/.elvish/lib/jarvy.elv

  # Add to ~/.elvish/rc.elv:
  # use jarvy
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_shell_display() {
        assert_eq!(CompletionShell::Bash.to_string(), "bash");
        assert_eq!(CompletionShell::Zsh.to_string(), "zsh");
        assert_eq!(CompletionShell::Fish.to_string(), "fish");
        assert_eq!(CompletionShell::PowerShell.to_string(), "powershell");
    }

    #[test]
    fn test_completion_shell_from_str() {
        assert_eq!(
            "bash".parse::<CompletionShell>().unwrap(),
            CompletionShell::Bash
        );
        assert_eq!(
            "zsh".parse::<CompletionShell>().unwrap(),
            CompletionShell::Zsh
        );
        assert_eq!(
            "fish".parse::<CompletionShell>().unwrap(),
            CompletionShell::Fish
        );
        assert_eq!(
            "powershell".parse::<CompletionShell>().unwrap(),
            CompletionShell::PowerShell
        );
        assert_eq!(
            "pwsh".parse::<CompletionShell>().unwrap(),
            CompletionShell::PowerShell
        );
    }

    #[test]
    fn test_completion_shell_from_str_case_insensitive() {
        assert_eq!(
            "BASH".parse::<CompletionShell>().unwrap(),
            CompletionShell::Bash
        );
        assert_eq!(
            "ZSH".parse::<CompletionShell>().unwrap(),
            CompletionShell::Zsh
        );
    }

    #[test]
    fn test_completion_shell_from_str_invalid() {
        assert!("invalid".parse::<CompletionShell>().is_err());
    }

    #[test]
    fn test_get_install_instructions() {
        let instructions = get_install_instructions();
        assert!(instructions.contains("Bash:"));
        assert!(instructions.contains("Zsh:"));
        assert!(instructions.contains("Fish:"));
        assert!(instructions.contains("PowerShell:"));
    }
}
