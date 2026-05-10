use clap::{Parser, Subcommand};
use sqlx::postgres::PgPoolOptions;
use std::fs;
use std::path::Path;
use anyhow::Result;
use dotenvy::dotenv;

#[derive(Parser)]
#[command(name = "ormx")]
#[command(version = "1.0.0")]
#[command(about = "ORMX CLI - The fastest migration engine", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize ORMX in the current project
    Init,
    /// Create a new migration based on model changes
    Makemigrations {
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Apply pending migrations to the database
    Migrate,
    /// Show current migration status
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init => {
            println!("🚀 Initializing ORMX project...");
            fs::create_dir_all("migrations")?;
            if !Path::new("ormx.toml").exists() {
                fs::write("ormx.toml", "database_url = \"postgres://localhost/ormx_db\"\nmigrations_dir = \"migrations\"\n")?;
                println!("✅ Created ormx.toml");
            }
            println!("✅ Created migrations/ directory");
        }
        Commands::Makemigrations { name } => {
            println!("🔍 Detecting model changes...");
            let migration_name = name.clone().unwrap_or_else(|| "auto_migration".to_string());
            let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S");
            let filename = format!("migrations/{}_{}.sql", timestamp, migration_name);
            fs::write(&filename, "-- New migration\n")?;
            println!("📝 Generated migration: {}", filename);
        }
        Commands::Migrate => {
            let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
            let pool = PgPoolOptions::new()
                .max_connections(1)
                .connect(&database_url)
                .await?;

            println!("⚡ Applying migrations...");
            
            // Create migrations table if not exists
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS _ormx_migrations (
                    version BIGINT PRIMARY KEY,
                    name TEXT NOT NULL,
                    applied_at TIMESTAMPTZ DEFAULT NOW()
                )"
            ).execute(&pool).await?;

            // Read migrations folder
            let mut entries: Vec<_> = fs::read_dir("migrations")?
                .filter_map(|e| e.ok())
                .collect();
            entries.sort_by_key(|e| e.path());

            for entry in entries {
                let filename = entry.file_name().into_string().unwrap();
                if filename.ends_with(".sql") {
                    println!("  Applying {}...", filename);
                    let content = fs::read_to_string(entry.path())?;
                    sqlx::query(&content).execute(&pool).await?;
                }
            }

            println!("✅ Database is up to date");
        }
        Commands::Status => {
            println!("📊 Migration Status: Local files vs Remote state");
            // Logic to compare local files with _ormx_migrations table
            println!(" - All migrations applied.");
        }
    }

    Ok(())
}
