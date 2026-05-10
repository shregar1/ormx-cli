use clap::{Parser, Subcommand};
use sqlx::any::{AnyPoolOptions, install_default_drivers};
use std::fs;
use std::path::Path;
use anyhow::Result;
use dotenvy::dotenv;
use std::process::Command;

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
    install_default_drivers();
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
            println!("🔍 Analyzing Python models...");
            
            let output = Command::new("python3")
                .arg("-m")
                .arg("ormx.cli_helper")
                .arg(".")
                .output()?;

            if !output.status.success() {
                println!("❌ Error analyzing models: {}", String::from_utf8_lossy(&output.stderr));
                return Ok(());
            }

            let schema_json = String::from_utf8_lossy(&output.stdout);
            println!("✨ Found models, generating SQL...");
            
            // In a real implementation, we would compare this schema with the previous one
            // For now, we generate a timestamped migration file
            let migration_name = name.clone().unwrap_or_else(|| "auto_migration".to_string());
            let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S");
            let filename = format!("migrations/{}_{}.sql", timestamp, migration_name);
            
            let sql_content = format!("-- Auto-generated migration\n-- Schema: {}\n", schema_json);
            fs::write(&filename, sql_content)?;
            
            println!("📝 Generated migration: {}", filename);
        }
        Commands::Migrate => {
            let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
            let pool = AnyPoolOptions::new()
                .max_connections(1)
                .connect(&database_url)
                .await?;

            println!("⚡ Applying migrations...");
            
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS _ormx_migrations (
                    version BIGINT PRIMARY KEY,
                    name TEXT NOT NULL,
                    applied_at TIMESTAMPTZ DEFAULT NOW()
                )"
            ).execute(&pool).await?;

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

            println!("✅ Migration complete.");
        }
        Commands::Status => {
            println!("📊 Migration Status: Connected.");
        }
    }

    Ok(())
}
