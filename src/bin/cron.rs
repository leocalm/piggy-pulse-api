use piggy_pulse::{Config, generate_periods};
use tracing_subscriber::EnvFilter;

fn print_usage(bin_name: &str) {
    eprintln!("Usage: {bin_name} generate-periods");
}

fn init_tracing(log_level: &str, json_format: bool) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));
    let subscriber = tracing_subscriber::fmt().with_env_filter(filter).with_target(true).with_line_number(true);

    if json_format {
        subscriber.json().init();
    } else {
        subscriber.init();
    }
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    let mut args = std::env::args();
    let bin_name = args.next().unwrap_or_else(|| "cron".to_string());
    let command = args.next();

    if command.as_deref() != Some("generate-periods") || args.next().is_some() {
        print_usage(&bin_name);
        std::process::exit(2);
    }

    let config = match Config::load() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("Failed to load configuration: {err}");
            std::process::exit(1);
        }
    };

    init_tracing(&config.logging.level, config.logging.json_format);

    match generate_periods(&config).await {
        Ok(result) => {
            println!(
                "Automatic period generation completed: users_processed={}, periods_created={}",
                result.users_processed, result.periods_created
            );
        }
        Err(err) => {
            eprintln!("Cron job failed: {err}");
            std::process::exit(1);
        }
    }
}
