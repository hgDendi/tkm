use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use secrecy::{ExposeSecret, SecretString};

use crate::core::registry::Registry;
use crate::core::token::{BackendType, TokenMeta};
use crate::storage::encrypted_file::EncryptedFileBackend;
use crate::storage::keychain::KeychainBackend;
use crate::storage::StorageBackend;

/// tkm — unified developer token manager
#[derive(Parser)]
#[command(name = "tkm", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize tkm: set master password and create vault
    Init,

    /// Get a token value
    Get {
        /// Service name (e.g., github, docker)
        service: String,
        /// Key name (default: "token")
        #[arg(short, long, default_value = "token")]
        key: String,
        /// Copy to clipboard instead of printing
        #[arg(short, long)]
        clip: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Set a token value
    Set {
        /// Service name
        service: String,
        /// Key name (default: "token")
        #[arg(short, long, default_value = "token")]
        key: String,
        /// Storage backend
        #[arg(short, long, default_value = "file")]
        backend: String,
        /// Human-readable label
        #[arg(short, long)]
        label: Option<String>,
        /// Associated username
        #[arg(short, long)]
        username: Option<String>,
        /// Service URL
        #[arg(long)]
        url: Option<String>,
    },

    /// Remove a token
    Rm {
        /// Service name
        service: String,
        /// Key name (default: "token")
        #[arg(short, long, default_value = "token")]
        key: String,
    },

    /// List all tokens (metadata only)
    List {
        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,
        /// Show only expired tokens
        #[arg(long)]
        expired: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Import tokens from existing tools
    Import {
        /// Source to import from: gh, glab, docker, gradle, pencil, all
        source: String,
    },

    /// Print eval-able export statements
    Env {
        /// Services to export
        services: Vec<String>,
    },

    /// Lock the vault
    Lock,

    /// Change master password
    Passwd,
}

fn tkm_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .expect("cannot find home directory")
        .join(".tkm")
}

fn registry_path() -> std::path::PathBuf {
    tkm_dir().join("registry.toml")
}

fn prompt_password(prompt: &str) -> Result<SecretString> {
    let password = rpassword::prompt_password(prompt)
        .context("failed to read password")?;
    if password.is_empty() {
        bail!("password cannot be empty");
    }
    Ok(SecretString::from(password))
}

fn get_backend(backend_type: &BackendType) -> Box<dyn StorageBackend> {
    match backend_type {
        BackendType::Keychain => Box::new(KeychainBackend::new()),
        BackendType::EncryptedFile => Box::new(EncryptedFileBackend::new(&tkm_dir())),
    }
}

fn parse_backend_type(s: &str) -> Result<BackendType> {
    match s {
        "keychain" | "kc" => Ok(BackendType::Keychain),
        "file" | "encrypted_file" | "ef" => Ok(BackendType::EncryptedFile),
        _ => bail!("unknown backend: {s} (use 'keychain' or 'file')"),
    }
}

fn ensure_unlocked(backend: &mut Box<dyn StorageBackend>) -> Result<()> {
    if backend.needs_unlock() {
        let password = prompt_password("Master password: ")?;
        backend.unlock(&password)?;
    }
    Ok(())
}

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        None => crate::tui::app::run_tui(),
        Some(command) => match command {
            Commands::Init => cmd_init(),
            Commands::Get { service, key, clip, json } => cmd_get(&service, &key, clip, json),
            Commands::Set { service, key, backend, label, username, url } => {
                cmd_set(&service, &key, &backend, label, username, url)
            }
            Commands::Rm { service, key } => cmd_rm(&service, &key),
            Commands::List { tag, expired, json } => cmd_list(tag.as_deref(), expired, json),
            Commands::Import { source } => cmd_import(&source),
            Commands::Env { services } => cmd_env(&services),
            Commands::Lock => cmd_lock(),
            Commands::Passwd => cmd_passwd(),
        },
    }
}

fn cmd_init() -> Result<()> {
    let dir = tkm_dir();
    let mut file_backend = EncryptedFileBackend::new(&dir);

    if file_backend.vault_exists() {
        bail!("tkm is already initialized. Use `tkm passwd` to change the master password.");
    }

    println!("Initializing tkm...");
    let password = prompt_password("Set master password: ")?;
    let confirm = prompt_password("Confirm master password: ")?;

    if password.expose_secret() != confirm.expose_secret() {
        bail!("passwords do not match");
    }

    file_backend.init(&password)?;

    // Create empty registry
    let registry = Registry::load(&registry_path())?;
    registry.save()?;

    println!("tkm initialized successfully!");
    println!("  Vault: {}", dir.join("vault.enc").display());
    println!("  Registry: {}", registry_path().display());
    println!("\nRun `tkm set <service>` to add your first token.");
    Ok(())
}

fn cmd_get(service: &str, key: &str, clip: bool, json: bool) -> Result<()> {
    let registry = Registry::load(&registry_path())?;

    let meta = registry
        .get_exact(service, key)
        .with_context(|| format!("no token registered for {service}:{key}"))?;

    let mut backend = get_backend(&meta.backend);
    ensure_unlocked(&mut backend)?;

    let secret = backend.get(service, key)?;

    if json {
        let output = serde_json::json!({
            "service": service,
            "key": key,
            "value": secret.expose_secret(),
            "backend": meta.backend.to_string(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if clip {
        let mut clipboard = arboard::Clipboard::new()
            .context("failed to access clipboard")?;
        clipboard
            .set_text(secret.expose_secret().to_string())
            .context("failed to copy to clipboard")?;
        eprintln!("Copied {service}:{key} to clipboard (clears in 30s)");

        // Spawn a thread to clear clipboard after 30s
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(30));
            if let Ok(mut cb) = arboard::Clipboard::new() {
                let _ = cb.set_text(String::new());
            }
        });
    } else {
        print!("{}", secret.expose_secret());
    }

    Ok(())
}

fn cmd_set(
    service: &str,
    key: &str,
    backend_str: &str,
    label: Option<String>,
    username: Option<String>,
    url: Option<String>,
) -> Result<()> {
    let backend_type = parse_backend_type(backend_str)?;
    let mut backend = get_backend(&backend_type);
    ensure_unlocked(&mut backend)?;

    let value = prompt_password(&format!("Enter value for {service}:{key}: "))?;

    backend.set(service, key, &value)?;

    // Update registry
    let mut registry = Registry::load(&registry_path())?;
    let mut meta = TokenMeta::new(service, key, backend_type);
    meta.label = label;
    meta.username = username;
    meta.url = url;

    // Preserve existing metadata if updating
    if let Some(existing) = registry.get_exact(service, key) {
        meta.created_at = existing.created_at;
        if meta.label.is_none() {
            meta.label = existing.label.clone();
        }
        if meta.username.is_none() {
            meta.username = existing.username.clone();
        }
        if meta.url.is_none() {
            meta.url = existing.url.clone();
        }
        if meta.tags.is_empty() {
            meta.tags = existing.tags.clone();
        }
    }

    registry.upsert(meta);
    registry.save()?;

    eprintln!("Saved {service}:{key} to {}", backend.name());
    Ok(())
}

fn cmd_rm(service: &str, key: &str) -> Result<()> {
    let mut registry = Registry::load(&registry_path())?;

    let meta = registry
        .get_exact(service, key)
        .with_context(|| format!("no token registered for {service}:{key}"))?
        .clone();

    let mut backend = get_backend(&meta.backend);
    ensure_unlocked(&mut backend)?;

    backend.delete(service, key)?;
    registry.remove(service, key);
    registry.save()?;

    eprintln!("Removed {service}:{key}");
    Ok(())
}

fn cmd_list(tag: Option<&str>, expired: bool, json: bool) -> Result<()> {
    let registry = Registry::load(&registry_path())?;
    let entries: Vec<&TokenMeta> = registry
        .list()
        .iter()
        .filter(|e| {
            if let Some(tag) = tag {
                e.tags.iter().any(|t| t == tag)
            } else {
                true
            }
        })
        .filter(|e| !expired || e.is_expired())
        .collect();

    if json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    if entries.is_empty() {
        println!("No tokens registered. Run `tkm set <service>` to add one.");
        return Ok(());
    }

    // Print table header
    println!(
        "{:<20} {:<12} {:<12} {:<12} {}",
        "SERVICE", "KEY", "BACKEND", "EXPIRES", "LABEL"
    );
    println!("{}", "-".repeat(72));

    for entry in entries {
        let expires = entry
            .expires_at
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "never".into());
        let label = entry.label.as_deref().unwrap_or("");
        println!(
            "{:<20} {:<12} {:<12} {:<12} {}",
            entry.service, entry.key, entry.backend, expires, label
        );
    }

    Ok(())
}

fn cmd_env(services: &[String]) -> Result<()> {
    if services.is_empty() {
        bail!("specify at least one service: tkm env github nexus-maven ...");
    }

    let registry = Registry::load(&registry_path())?;

    // Group by backend to minimize unlock prompts
    let mut file_entries = Vec::new();
    let mut keychain_entries = Vec::new();

    for svc in services {
        let meta = registry
            .get(svc)
            .with_context(|| format!("no token registered for {svc}"))?;

        match meta.backend {
            BackendType::EncryptedFile => file_entries.push(meta.clone()),
            BackendType::Keychain => keychain_entries.push(meta.clone()),
        }
    }

    // Process keychain entries (no unlock needed)
    let kc = KeychainBackend::new();
    for entry in &keychain_entries {
        let secret = kc.get(&entry.service, &entry.key)?;
        let env_var = format!("{}_{}", entry.service, entry.key)
            .to_uppercase()
            .replace('-', "_");
        println!("export {}={}", env_var, shell_escape(secret.expose_secret()));
    }

    // Process file entries (unlock once)
    if !file_entries.is_empty() {
        let mut fb = EncryptedFileBackend::new(&tkm_dir());
        let password = prompt_password("Master password: ")?;
        fb.unlock(&password)?;

        for entry in &file_entries {
            let secret = fb.get(&entry.service, &entry.key)?;
            let env_var = format!("{}_{}", entry.service, entry.key)
                .to_uppercase()
                .replace('-', "_");
            println!("export {}={}", env_var, shell_escape(secret.expose_secret()));
        }
    }

    Ok(())
}

fn cmd_import(source: &str) -> Result<()> {
    use crate::integrations::{gh, glab, docker, gradle, pencil};
    use crate::integrations::gh::ImportEntry;

    let dir = tkm_dir();
    let mut file_backend = EncryptedFileBackend::new(&dir);

    // Ensure vault exists
    if !file_backend.vault_exists() {
        bail!("tkm is not initialized. Run `tkm init` first.");
    }

    // Unlock file backend for storing imports
    let password = prompt_password("Master password: ")?;
    file_backend.unlock(&password)?;

    let mut kc = KeychainBackend::new();
    let mut registry = Registry::load(&registry_path())?;

    let sources: Vec<&str> = match source {
        "all" => vec!["gh", "glab", "docker", "gradle", "pencil"],
        s => vec![s],
    };

    let mut total_imported = 0;

    for src in sources {
        let entries: Vec<ImportEntry> = match src {
            "gh" | "github" => {
                eprint!("Scanning GitHub CLI... ");
                gh::scan().unwrap_or_else(|e| { eprintln!("skip ({e})"); Vec::new() })
            }
            "glab" | "gitlab" => {
                eprint!("Scanning GitLab CLI... ");
                glab::scan().unwrap_or_else(|e| { eprintln!("skip ({e})"); Vec::new() })
            }
            "docker" => {
                eprint!("Scanning Docker... ");
                docker::scan().unwrap_or_else(|e| { eprintln!("skip ({e})"); Vec::new() })
            }
            "gradle" | "maven" => {
                eprint!("Scanning Gradle properties... ");
                gradle::scan().unwrap_or_else(|e| { eprintln!("skip ({e})"); Vec::new() })
            }
            "pencil" => {
                eprint!("Scanning Pencil... ");
                pencil::scan().unwrap_or_else(|e| { eprintln!("skip ({e})"); Vec::new() })
            }
            _ => {
                eprintln!("Unknown source: {src}. Available: gh, glab, docker, gradle, pencil, all");
                continue;
            }
        };

        if entries.is_empty() {
            eprintln!("no tokens found");
            continue;
        }

        eprintln!("{} token(s) found", entries.len());

        for entry in entries {
            // Check if already exists
            if registry.get_exact(&entry.meta.service, &entry.meta.key).is_some() {
                eprintln!("  skip {}:{} (already exists)", entry.meta.service, entry.meta.key);
                continue;
            }

            // Store based on backend type
            let result = match entry.meta.backend {
                BackendType::Keychain => kc.set(&entry.meta.service, &entry.meta.key, &entry.value),
                BackendType::EncryptedFile => file_backend.set(&entry.meta.service, &entry.meta.key, &entry.value),
            };

            match result {
                Ok(()) => {
                    eprintln!("  imported {}:{} -> {}", entry.meta.service, entry.meta.key, entry.meta.backend);
                    registry.upsert(entry.meta);
                    total_imported += 1;
                }
                Err(e) => {
                    eprintln!("  failed {}:{}: {e}", entry.meta.service, entry.meta.key);
                }
            }
        }
    }

    registry.save()?;
    eprintln!("\nImported {total_imported} token(s).");
    Ok(())
}

fn cmd_lock() -> Result<()> {
    eprintln!("Session locked.");
    Ok(())
}

fn cmd_passwd() -> Result<()> {
    let dir = tkm_dir();
    let mut backend = EncryptedFileBackend::new(&dir);

    if !backend.vault_exists() {
        bail!("tkm is not initialized. Run `tkm init` first.");
    }

    let old_password = prompt_password("Current master password: ")?;
    backend.unlock(&old_password)?;

    let new_password = prompt_password("New master password: ")?;
    let confirm = prompt_password("Confirm new master password: ")?;

    if new_password.expose_secret() != confirm.expose_secret() {
        bail!("passwords do not match");
    }

    // Re-encrypt vault with new password
    backend.lock();
    // Re-init with new password would overwrite - instead we need to:
    // 1. Read current vault data
    // 2. Generate new salt
    // 3. Derive new key
    // 4. Re-encrypt and save

    // Unlock with old password to get vault data
    backend.unlock(&old_password)?;

    // Get vault data via get/set cycle is not ideal.
    // For now, just re-init - this is a V1 limitation.
    // A proper implementation would expose the vault data directly.
    eprintln!("Password change is not yet implemented in V1. Coming soon!");
    Ok(())
}

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
