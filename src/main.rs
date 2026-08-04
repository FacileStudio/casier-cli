mod api;
mod auth;
mod commands;
mod config;

use clap::{Parser, Subcommand};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "casier", about = "Secrets manager CLI for Casier")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Authenticate with the Casier server")]
    Login,
    #[command(about = "Clear stored credentials")]
    Logout,
    #[command(about = "Initialize a .casier.toml for the current project")]
    Init,
    #[command(about = "List spaces you belong to")]
    Spaces,
    #[command(about = "Manage secrets")]
    Secrets {
        #[command(subcommand)]
        action: SecretsAction,
    },
    #[command(about = "Compare secrets between two environments")]
    Diff {
        #[arg(short, long)]
        space: String,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
    },
    #[command(about = "Sync secrets with a .env file")]
    Sync {
        #[command(subcommand)]
        action: SyncAction,
    },
    #[command(about = "Inject secrets as env vars and run a command")]
    Run {
        #[arg(short, long)]
        space: String,
        #[arg(short, long, default_value = "dev")]
        env: String,
        #[arg(last = true)]
        command: Vec<String>,
    },
}

#[derive(Subcommand)]
enum SyncAction {
    #[command(about = "Push a .env file to Casier")]
    Push {
        #[arg(short, long)]
        space: String,
        #[arg(short, long, default_value = "dev")]
        env: String,
        #[arg(short, long, default_value = ".env")]
        file: String,
    },
    #[command(about = "Pull secrets from Casier to a .env file")]
    Pull {
        #[arg(short, long)]
        space: String,
        #[arg(short, long, default_value = "dev")]
        env: String,
        #[arg(short, long, default_value = ".env")]
        file: String,
    },
}

#[derive(Subcommand)]
enum SecretsAction {
    #[command(about = "List secrets for a space and environment")]
    List {
        #[arg(short, long)]
        space: String,
        #[arg(short, long, default_value = "dev")]
        env: String,
        #[arg(long)]
        show: bool,
    },
    #[command(about = "Set a secret")]
    Set {
        #[arg(short, long)]
        space: String,
        #[arg(short, long, default_value = "dev")]
        env: String,
        key: String,
        value: String,
    },
    #[command(about = "Get a single secret value")]
    Get {
        #[arg(short, long)]
        space: String,
        #[arg(short, long, default_value = "dev")]
        env: String,
        key: String,
    },
    #[command(about = "Delete a secret")]
    Delete {
        #[arg(short, long)]
        space: String,
        #[arg(short, long, default_value = "dev")]
        env: String,
        key: String,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Login => commands::login::run().await.map(|_| ExitCode::SUCCESS),
        Commands::Logout => commands::logout::run().map(|_| ExitCode::SUCCESS),
        Commands::Init => commands::init::run().await.map(|_| ExitCode::SUCCESS),
        Commands::Diff { space, from, to } => commands::diff::run(&space, &from, &to)
            .await
            .map(|_| ExitCode::SUCCESS),
        Commands::Sync { action } => match action {
            SyncAction::Push { space, env, file } => commands::sync::push(&space, &env, &file)
                .await
                .map(|_| ExitCode::SUCCESS),
            SyncAction::Pull { space, env, file } => commands::sync::pull(&space, &env, &file)
                .await
                .map(|_| ExitCode::SUCCESS),
        },
        Commands::Spaces => commands::spaces::run().await.map(|_| ExitCode::SUCCESS),
        Commands::Secrets { action } => match action {
            SecretsAction::List { space, env, show } => commands::secrets::list(&space, &env, show)
                .await
                .map(|_| ExitCode::SUCCESS),
            SecretsAction::Set {
                space,
                env,
                key,
                value,
            } => commands::secrets::set(&space, &env, &key, &value)
                .await
                .map(|_| ExitCode::SUCCESS),
            SecretsAction::Get { space, env, key } => commands::secrets::get(&space, &env, &key)
                .await
                .map(|_| ExitCode::SUCCESS),
            SecretsAction::Delete { space, env, key } => {
                commands::secrets::delete(&space, &env, &key)
                    .await
                    .map(|_| ExitCode::SUCCESS)
            }
        },
        Commands::Run {
            space,
            env,
            command,
        } => commands::run::run(&space, &env, &command).await,
    };

    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {:#}", e);
            ExitCode::FAILURE
        }
    }
}
