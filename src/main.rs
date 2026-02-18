use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "agent-shell")]
#[command(about = "Small Rust CLI skeleton", long_about = None)]
struct Cli {
    /// Increase log verbosity (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print a greeting
    Greet {
        /// Name to greet (defaults to "world")
        #[arg(long)]
        name: Option<String>,
    },

    /// Echo a message
    Echo {
        /// Message to print
        message: String,
    },

    /// Print version
    Version,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Default to info unless RUST_LOG is set.
    // Allow -v/-vv to override it in a predictable way.
    let default_level = match cli.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    tracing_subscriber::fmt().with_env_filter(filter).init();

    match cli.command {
        Command::Greet { name } => {
            let out = agent_shell::commands::greet(name.as_deref());
            println!("{out}");
        }
        Command::Echo { message } => {
            let out = agent_shell::commands::echo(&message);
            println!("{out}");
        }
        Command::Version => {
            println!("{}", agent_shell::commands::version());
        }
    }

    info!("done");
    Ok(())
}
