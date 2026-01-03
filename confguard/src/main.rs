use clap::Parser;
use colored::Colorize;
use confguard::cli::{args::Cli, execute_command};
use tracing::{debug, info};
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::{
    filter::LevelFilter,
    fmt::{self, format::FmtSpan},
    prelude::*,
};

fn main() {
    let cli = Cli::parse();

    // Set up logging based on verbosity level
    setup_logging(cli.debug);

    if let Err(e) = execute_command(&cli) {
        eprintln!("{}", format!("Error: {}", e).red());
        std::process::exit(1);
    }
}

fn setup_logging(verbosity: u8) {
    let filter = match verbosity {
        0 => LevelFilter::WARN,
        1 => LevelFilter::INFO,
        2 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    };

    // Create a noisy module filter
    let noisy_modules = ["skim", "html5ever", "reqwest", "mio", "want"];
    let module_filter = filter_fn(move |metadata| {
        !noisy_modules
            .iter()
            .any(|name| metadata.target().starts_with(name))
    });

    // Create a subscriber with formatted output
    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_names(false)
        .with_span_events(FmtSpan::CLOSE)
        .with_filter(filter)
        .with_filter(module_filter);

    tracing_subscriber::registry().with(fmt_layer).init();

    // Log initial debug level
    match filter {
        LevelFilter::INFO => info!("Debug mode: info"),
        LevelFilter::DEBUG => debug!("Debug mode: debug"),
        LevelFilter::TRACE => debug!("Debug mode: trace"),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert()
    }
}
