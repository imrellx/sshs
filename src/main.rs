pub mod searchable;
pub mod ssh;
pub mod ssh_config;
pub mod ui;

use anyhow::Result;
use clap::Parser;
use ui::{App, AppConfig};

// Constants for default configuration
const DEFAULT_SYSTEM_SSH_CONFIG: &str = "/etc/ssh/ssh_config";
const DEFAULT_USER_SSH_CONFIG: &str = "~/.ssh/config";
const DEFAULT_SSH_TEMPLATE: &str = "ssh \"{{{name}}}\"";

// Default values for CLI flags
const DEFAULT_SORT_BY_NAME: bool = true;
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
