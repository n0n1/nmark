use nmark::app::App;
use nmark::cli::{Command, RunOptions};
use nmark::error::{AppError, AppResult};
use nmark::inputs::InputError;
use nmark::settings::{resolve_config, SettingsError};

#[tokio::main]
async fn main() {
    if let Err(error) = run_main().await {
        eprintln!("Error: {error}");

        if should_show_download_help(&error) {
            eprintln!();
            eprintln!("{}", Command::usage());
        }

        std::process::exit(1);
    }
}

async fn run_main() -> AppResult<()> {
    match Command::parse().map_err(AppError::from)? {
        Command::Help => {
            print!("{}", Command::usage());
            Ok(())
        }
        Command::Run(options) => run(options).await,
    }
}

async fn run(options: RunOptions) -> AppResult<()> {
    let config = resolve_config(options).map_err(AppError::from)?;
    let app = App::new(&config.http)?;
    app.run(config).await
}

fn should_show_download_help(error: &AppError) -> bool {
    matches!(
        error,
        AppError::Settings(SettingsError::Input(InputError::DownloadListNotFound(_)))
    )
}
