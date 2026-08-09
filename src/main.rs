mod api;
mod auth;
mod cache;
mod commands;
mod config;
mod envfile;
mod loopback;
mod ui;

use clap::{Parser, Subcommand};
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "casier",
    version,
    about = "Terminal client for Casier secrets management"
)]
struct Cli {
    #[arg(long, global = true, help = "Disable colored output")]
    no_color: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Authenticate with the Casier server")]
    Login {
        #[arg(long, help = "Casier server URL (saved for future commands)")]
        server: Option<String>,
        #[arg(long, help = "Print the sign-in URL instead of opening a browser")]
        no_browser: bool,
    },
    #[command(about = "Clear stored credentials")]
    Logout,
    #[command(about = "Initialize a casier.yml for the current project")]
    Init,
    #[command(about = "List projects you belong to")]
    Projects,
    #[command(about = "Manage secrets")]
    Secrets {
        #[command(subcommand)]
        action: SecretsAction,
    },
    #[command(about = "Compare secrets between two environments")]
    Diff {
        #[arg(short, long)]
        project: String,
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
        #[arg(short, long, help = "Project slug (defaults to casier.yml)")]
        project: Option<String>,
        #[arg(short, long, help = "Environment (defaults to casier.yml, then dev)")]
        env: Option<String>,
        #[arg(long, help = "Use cached secrets without contacting the server")]
        offline: bool,
        #[arg(last = true)]
        command: Vec<String>,
    },
    #[command(
        about = "Check a .env file against remote secrets (exit 1 if keys are missing remotely)"
    )]
    Check {
        #[arg(default_value = ".env")]
        file: String,
        #[arg(short, long, help = "Project slug (defaults to casier.yml)")]
        project: Option<String>,
        #[arg(short, long, help = "Environment (defaults to casier.yml, then dev)")]
        env: Option<String>,
    },
    #[command(about = "Push secrets to an external target")]
    Push {
        #[command(subcommand)]
        target: PushTarget,
    },
}

#[derive(Subcommand)]
enum PushTarget {
    #[command(
        about = "Push secrets to a Dokploy compose service (needs DOKPLOY_URL and DOKPLOY_API_KEY)"
    )]
    Dokploy {
        compose_id: String,
        #[arg(short, long, help = "Project slug (defaults to casier.yml)")]
        project: Option<String>,
        #[arg(short, long, help = "Environment (defaults to casier.yml, then dev)")]
        env: Option<String>,
    },
}

#[derive(Subcommand)]
enum SyncAction {
    #[command(about = "Push a .env file to Casier")]
    Push {
        #[arg(short, long)]
        project: String,
        #[arg(short, long, default_value = "dev")]
        env: String,
        #[arg(short, long, default_value = ".env")]
        file: String,
    },
    #[command(about = "Pull secrets from Casier to a .env file")]
    Pull {
        #[arg(short, long)]
        project: String,
        #[arg(short, long, default_value = "dev")]
        env: String,
        #[arg(short, long, default_value = ".env")]
        file: String,
    },
}

#[derive(Subcommand)]
enum SecretsAction {
    #[command(about = "List secrets for a project and environment")]
    List {
        #[arg(short, long)]
        project: String,
        #[arg(short, long, default_value = "dev")]
        env: String,
        #[arg(long)]
        show: bool,
    },
    #[command(about = "Set a secret")]
    Set {
        #[arg(short, long)]
        project: String,
        #[arg(short, long, default_value = "dev")]
        env: String,
        key: String,
        value: String,
    },
    #[command(about = "Get a single secret value")]
    Get {
        #[arg(short, long)]
        project: String,
        #[arg(short, long, default_value = "dev")]
        env: String,
        key: String,
    },
    #[command(about = "Delete a secret")]
    Delete {
        #[arg(short, long)]
        project: String,
        #[arg(short, long, default_value = "dev")]
        env: String,
        key: String,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    if cli.no_color {
        ui::disable_color();
    }

    let result = match cli.command {
        Commands::Login { server, no_browser } => commands::login::run(server, no_browser)
            .await
            .map(|_| ExitCode::SUCCESS),
        Commands::Logout => commands::logout::run().map(|_| ExitCode::SUCCESS),
        Commands::Init => commands::init::run().await.map(|_| ExitCode::SUCCESS),
        Commands::Diff { project, from, to } => commands::diff::run(&project, &from, &to)
            .await
            .map(|_| ExitCode::SUCCESS),
        Commands::Sync { action } => match action {
            SyncAction::Push { project, env, file } => commands::sync::push(&project, &env, &file)
                .await
                .map(|_| ExitCode::SUCCESS),
            SyncAction::Pull { project, env, file } => commands::sync::pull(&project, &env, &file)
                .await
                .map(|_| ExitCode::SUCCESS),
        },
        Commands::Projects => commands::projects::run().await.map(|_| ExitCode::SUCCESS),
        Commands::Secrets { action } => match action {
            SecretsAction::List { project, env, show } => {
                commands::secrets::list(&project, &env, show)
                    .await
                    .map(|_| ExitCode::SUCCESS)
            }
            SecretsAction::Set {
                project,
                env,
                key,
                value,
            } => commands::secrets::set(&project, &env, &key, &value)
                .await
                .map(|_| ExitCode::SUCCESS),
            SecretsAction::Get { project, env, key } => {
                commands::secrets::get(&project, &env, &key)
                    .await
                    .map(|_| ExitCode::SUCCESS)
            }
            SecretsAction::Delete { project, env, key } => {
                commands::secrets::delete(&project, &env, &key)
                    .await
                    .map(|_| ExitCode::SUCCESS)
            }
        },
        Commands::Run {
            project,
            env,
            offline,
            command,
        } => commands::run::run(project, env, offline, &command).await,
        Commands::Check { file, project, env } => commands::check::run(&file, project, env).await,
        Commands::Push { target } => match target {
            PushTarget::Dokploy {
                compose_id,
                project,
                env,
            } => commands::push::dokploy(&compose_id, project, env)
                .await
                .map(|_| ExitCode::SUCCESS),
        },
    };

    match result {
        Ok(code) => code,
        Err(e) => {
            ui::error(&format!("{e:#}"));
            ExitCode::FAILURE
        }
    }
}
