pub mod searchable;
pub mod ssh;
pub mod ssh_config;
pub mod ui;

#[cfg(test)]
mod error_handling_tests;

#[cfg(test)]
mod ssh_security_tests;

#[cfg(test)]
mod terminal_state_tests;

use anyhow::Result;
use clap::Parser;
use ui::{App, AppConfig};

// Constants for default configuration
const DEFAULT_SYSTEM_SSH_CONFIG: &str = "/etc/ssh/ssh_config";
const DEFAULT_USER_SSH_CONFIG: &str = "~/.ssh/config";
const DEFAULT_SSH_TEMPLATE: &str = "ssh \"{{{name}}}\"";

// Default values for CLI flags
const DEFAULT_SORT_BY_NAME: bool = true;
const DEFAULT_SHOW_PROXY_COMMAND: bool = false;
const DEFAULT_EXIT_AFTER_SESSION: bool = false;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the SSH configuration file
    #[arg(
        short,
        long,
        num_args = 1..,
        default_values_t = [
            DEFAULT_SYSTEM_SSH_CONFIG.to_string(),
            DEFAULT_USER_SSH_CONFIG.to_string(),
        ],
    )]
    config: Vec<String>,

    /// Shows `ProxyCommand`
    #[arg(long)]
    show_proxy_command: bool,

    /// Host search filter
    #[arg(short, long)]
    search: Option<String>,

    /// Sort hosts by hostname
    #[arg(long, default_value_t = DEFAULT_SORT_BY_NAME)]
    sort: bool,

    /// Handlebars template of the command to execute
    #[arg(short, long, default_value = DEFAULT_SSH_TEMPLATE)]
    template: String,

    /// Handlebars template of the command to execute when an SSH session starts
    #[arg(long, value_name = "TEMPLATE")]
    on_session_start_template: Option<String>,

    /// Handlebars template of the command to execute when an SSH session ends
    #[arg(long, value_name = "TEMPLATE")]
    on_session_end_template: Option<String>,

    /// Exit after ending the SSH session
    #[arg(short, long, default_value_t = DEFAULT_EXIT_AFTER_SESSION)]
    exit: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut app = App::new(&AppConfig {
        config_paths: args.config,
        search_filter: args.search,
        sort_by_name: args.sort,
        show_proxy_command: args.show_proxy_command,
        command_template: args.template,
        command_template_on_session_start: args.on_session_start_template,
        command_template_on_session_end: args.on_session_end_template,
        exit_after_ssh_session_ends: args.exit,
    })?;
    app.start()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_default_proxy_command_flag() {
        // Test that show_proxy_command defaults to false
        let args = Args::try_parse_from(vec!["sshs"]).unwrap();
        assert_eq!(args.show_proxy_command, false);
    }

    #[test]
    fn test_proxy_command_flag_can_be_enabled() {
        // Test that we can enable show_proxy_command with --show-proxy-command
        let args = Args::try_parse_from(vec!["sshs", "--show-proxy-command"]).unwrap();
        assert_eq!(args.show_proxy_command, true);
    }

    #[test]
    fn test_proxy_command_flag_remains_false_when_not_specified() {
        // Test with other flags but without --show-proxy-command
        // Since sort has a default value, we should test with a different combination
        let args = Args::try_parse_from(vec!["sshs", "--search", "test"]).unwrap();
        assert_eq!(args.show_proxy_command, false);
        assert_eq!(args.search, Some("test".to_string()));
    }

    #[test]
    fn test_help_shows_proxy_command_option() {
        // This test will verify that the help text includes the proxy command option
        let help_output = Args::try_parse_from(vec!["sshs", "--help"]);
        
        // --help causes an early exit, so we expect an error
        assert!(help_output.is_err());
        
        // The help message should mention show-proxy-command
        let error_msg = format!("{}", help_output.unwrap_err());
        assert!(error_msg.contains("show-proxy-command"));
    }

    #[test]
    fn test_default_config_paths() {
        // Test that default SSH config paths are correctly set
        let args = Args::try_parse_from(vec!["sshs"]).unwrap();
        assert_eq!(args.config.len(), 2);
        assert_eq!(args.config[0], DEFAULT_SYSTEM_SSH_CONFIG);
        assert_eq!(args.config[1], DEFAULT_USER_SSH_CONFIG);
    }

    #[test]
    fn test_default_template() {
        // Test that the default SSH template is correctly set
        let args = Args::try_parse_from(vec!["sshs"]).unwrap();
        assert_eq!(args.template, DEFAULT_SSH_TEMPLATE);
    }

    #[test]
    fn test_constants_accessibility() {
        // Verify our constants are accessible and have the expected values
        assert_eq!(DEFAULT_SYSTEM_SSH_CONFIG, "/etc/ssh/ssh_config");
        assert_eq!(DEFAULT_USER_SSH_CONFIG, "~/.ssh/config");
        assert_eq!(DEFAULT_SSH_TEMPLATE, "ssh \"{{{name}}}\"");
        assert_eq!(DEFAULT_SORT_BY_NAME, true);
        assert_eq!(DEFAULT_SHOW_PROXY_COMMAND, false);
        assert_eq!(DEFAULT_EXIT_AFTER_SESSION, false);
    }

    #[test]
    fn test_config_path_override() {
        // Test that config paths can be overridden
        let args = Args::try_parse_from(vec!["sshs", "-c", "/custom/config"]).unwrap();
        assert_eq!(args.config.len(), 1);
        assert_eq!(args.config[0], "/custom/config");
    }

    #[test]
    fn test_template_override() {
        // Test that the template can be overridden
        let args = Args::try_parse_from(vec!["sshs", "-t", "custom_command \"{{name}}\""]).unwrap();
        assert_eq!(args.template, "custom_command \"{{name}}\"");
    }
}
