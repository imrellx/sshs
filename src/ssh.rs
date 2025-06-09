use anyhow::anyhow;
use handlebars::Handlebars;
use itertools::Itertools;
use serde::Serialize;
use std::collections::VecDeque;
use std::process::Command;

use crate::ssh_config::{self, parser_error::ParseError, HostVecExt};

#[derive(Debug, Serialize, Clone)]
pub struct Host {
    pub name: String,
    pub aliases: String,
    pub user: Option<String>,
    pub destination: String,
    pub port: Option<String>,
    pub proxy_command: Option<String>,
}

impl Host {
    /// Validates that a string only contains safe characters for command execution.
    /// Uses an allowlist approach to ensure only known-safe characters are permitted.
    ///
    /// # Errors
    ///
    /// Will return `Err` if the value contains characters not in the allowlist.
    fn validate_safe_for_command(value: &str) -> anyhow::Result<()> {
        // Define an allowlist of characters that are considered safe
        // This is a more secure approach than a denylist
        let allowed_chars: &[char] = &[
            // Lowercase letters
            'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm',
            'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
            // Uppercase letters
            'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M',
            'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
            // Numbers
            '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
            // Safe punctuation and symbols for hostnames, usernames, and paths
            '.', '-', '_', ':', '/', '@', '%', '+', '=', ' ',
            // Additional safe characters for SSH config values
            ',',
        ];

        // Check if any character is not in our allowlist
        if let Some(unsafe_char) = value.chars().find(|c| !allowed_chars.contains(c)) {
            return Err(anyhow::anyhow!(
                "Value contains potentially dangerous characters: '{}' in: {}",
                unsafe_char,
                value
            ));
        }

        Ok(())
    }

    /// Uses the provided Handlebars template to run a command.
    ///
    /// # Errors
    ///
    /// Will return `Err` if the command cannot be executed or contains unsafe characters.
    pub fn run_command_template(&self, pattern: &str) -> anyhow::Result<()> {
        // Validate all fields that could be used in the template
        Self::validate_safe_for_command(&self.name)?;
        if let Some(ref user) = self.user {
            Self::validate_safe_for_command(user)?;
        }
        Self::validate_safe_for_command(&self.destination)?;
        if let Some(ref port) = self.port {
            Self::validate_safe_for_command(port)?;
        }
        if let Some(ref proxy) = self.proxy_command {
            Self::validate_safe_for_command(proxy)?;
        }
        Self::validate_safe_for_command(&self.aliases)?;

        let handlebars = Handlebars::new();
        let rendered_command = handlebars.render_template(pattern, &self)?;

        println!("Running command: {rendered_command}");

        let mut args = shlex::split(&rendered_command)
            .ok_or(anyhow!("Failed to parse command: {rendered_command}"))?
            .into_iter()
            .collect::<VecDeque<String>>();
        let command = args.pop_front().ok_or(anyhow!("Failed to get command"))?;

        let status = Command::new(command).args(args).spawn()?.wait()?;
        if !status.success() {
            // Only exit the process when not running in test mode
            std::process::exit(status.code().unwrap_or(1));
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum ParseConfigError {
    Io(std::io::Error),
    SshConfig(ParseError),
}

impl std::fmt::Display for ParseConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseConfigError::Io(e) => write!(f, "IO error: {e}"),
            ParseConfigError::SshConfig(e) => write!(f, "SSH config parsing error: {e:?}"),
        }
    }
}

impl std::error::Error for ParseConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParseConfigError::Io(e) => Some(e),
            ParseConfigError::SshConfig(e) => Some(e), // Now this works since ParseError implements Error
        }
    }
}

impl From<std::io::Error> for ParseConfigError {
    fn from(e: std::io::Error) -> Self {
        ParseConfigError::Io(e)
    }
}

impl From<ParseError> for ParseConfigError {
    fn from(e: ParseError) -> Self {
        ParseConfigError::SshConfig(e)
    }
}

/// # Errors
///
/// Will return `Err` if the SSH configuration file cannot be parsed.
pub fn parse_config(raw_path: &String) -> Result<Vec<Host>, ParseConfigError> {
    let normalized_path = shellexpand::tilde(&raw_path).to_string();
    let path = std::fs::canonicalize(normalized_path)?;

    let hosts = ssh_config::Parser::new()
        .parse_file(path)?
        .apply_patterns()
        .apply_name_to_empty_hostname()
        .merge_same_hosts()
        .iter()
        .map(|host| Host {
            name: host
                .get_patterns()
                .first()
                .unwrap_or(&String::new())
                .clone(),
            aliases: host.get_patterns().iter().skip(1).join(", "),
            user: host.get(&ssh_config::EntryType::User),
            destination: host
                .get(&ssh_config::EntryType::Hostname)
                .unwrap_or_default(),
            port: host.get(&ssh_config::EntryType::Port),
            proxy_command: host.get(&ssh_config::EntryType::ProxyCommand),
        })
        .collect();

    Ok(hosts)
}
