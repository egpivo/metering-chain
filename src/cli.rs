use clap::{Parser, Subcommand};
use metering_chain::config::Config;
use metering_chain::storage::{Storage, FileStorage};
use metering_chain::state::{State, apply};
use metering_chain::tx::SignedTx;
use metering_chain::error::{Error, Result};
use std::collections::HashSet;
use std::fs;
use std::io::{self, Read};

#[derive(Parser)]
#[command(name = "metering-chain")]
#[command(about = "Metering Chain CLI - Service usage and billing state machine")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output format: "human" or "json"
    #[arg(short, long, default_value = "human")]
    pub format: String,

    /// Data directory path
    #[arg(short, long)]
    pub data_dir: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize the data directory
    Init,

    /// Apply a transaction
    Apply {
        /// Transaction JSON (or read from stdin if not provided)
        #[arg(short, long)]
        tx: Option<String>,

        /// Transaction file path
        #[arg(short, long)]
        file: Option<String>,

        /// Dry-run: validate but don't apply
        #[arg(long)]
        dry_run: bool,
    },

    /// Show account information
    Account {
        /// Account address
        address: String,
    },

    /// List meters for an account
    Meters {
        /// Account address
        address: String,
    },

    /// Show usage report
    Report {
        /// Account address (optional, shows all if not provided)
        address: Option<String>,
    },
}

/// Load state from storage or return genesis state
pub fn load_or_create_state(storage: &FileStorage) -> Result<(State, u64)> {
    match storage.load_state()? {
        Some((state, last_tx_id)) => {
            let txs = storage.load_txs_from(last_tx_id + 1)?;
            let mut current_state = state;
            let mut current_tx_id = last_tx_id;
            let minters = get_authorized_minters();

            for tx in txs {
                current_state = apply(&current_state, &tx, &minters)?;
                current_tx_id += 1;
            }

            Ok((current_state, current_tx_id))
        }
        None => Ok((State::new(), 0)),
    }
}

/// Get authorized minters (hardcoded for MVP)
fn get_authorized_minters() -> HashSet<String> {
    let mut minters = HashSet::new();
    minters.insert("authority".to_string());
    minters
}

/// Parse transaction from JSON string
fn parse_tx(json: &str) -> Result<SignedTx> {
    let tx: SignedTx = serde_json::from_str(json)
        .map_err(|e| Error::InvalidTransaction(format!("Failed to parse transaction JSON: {}", e)))?;
    Ok(tx)
}

/// Read transaction from file or stdin
fn read_tx(file: Option<&str>) -> Result<String> {
    match file {
        Some(path) => {
            fs::read_to_string(path)
                .map_err(|e| Error::InvalidTransaction(format!("Failed to read file {}: {}", path, e)))
        }
        None => {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)
                .map_err(|e| Error::InvalidTransaction(format!("Failed to read from stdin: {}", e)))?;
            Ok(buffer)
        }
    }
}

/// Format output based on format type
fn format_output<T: serde::Serialize + std::fmt::Debug>(data: &T, format: &str) -> Result<String> {
    match format {
        "json" => {
            serde_json::to_string_pretty(data)
                .map_err(|e| Error::StateError(format!("Failed to serialize JSON: {}", e)))
        }
        _ => {
            Ok(format!("{:#?}", data))
        }
    }
}

pub fn run(cli: Cli) -> Result<()> {
    let mut config = Config::new();
    if let Some(dir) = cli.data_dir {
        config.set_data_dir(std::path::PathBuf::from(dir));
    }
    if cli.format == "json" {
        config.set_output_format("json".to_string());
    }

    let mut storage = FileStorage::new(&config);
    let minters = get_authorized_minters();

    match cli.command {
        Commands::Init => {
            fs::create_dir_all(config.get_data_dir())
                .map_err(|e| Error::StateError(format!("Failed to create data directory: {}", e)))?;
            println!("Initialized data directory at: {}", config.get_data_dir().display());
            Ok(())
        }

        Commands::Apply { tx, file, dry_run } => {
            let (mut state, mut last_tx_id) = load_or_create_state(&storage)?;

            let tx_json = match tx {
                Some(json) => json,
                None => read_tx(file.as_deref())?,
            };

            let signed_tx = parse_tx(&tx_json)?;
            let cost_opt = metering_chain::tx::validation::validate(&state, &signed_tx, &minters)?;

            if dry_run {
                println!("✓ Transaction is valid");
                if let Some(cost) = cost_opt {
                    println!("  Cost: {}", cost);
                }
                return Ok(());
            }

            state = apply(&state, &signed_tx, &minters)?;
            last_tx_id += 1;

            storage.append_tx(&signed_tx)?;
            storage.persist_state(&state, last_tx_id)?;

            println!("✓ Transaction applied successfully");
            if let Some(cost) = cost_opt {
                println!("  Cost: {}", cost);
            }

            Ok(())
        }

        Commands::Account { address } => {
            let (state, _) = load_or_create_state(&storage)?;

            match state.get_account(&address) {
                Some(account) => {
                    let output = AccountOutput {
                        address: address.clone(),
                        balance: account.balance(),
                        nonce: account.nonce(),
                    };
                    println!("{}", format_output(&output, &cli.format)?);
                    Ok(())
                }
                None => {
                    Err(Error::StateError(format!("Account {} not found", address)))
                }
            }
        }

        Commands::Meters { address } => {
            let (state, _) = load_or_create_state(&storage)?;

            let meters: Vec<MeterOutput> = state
                .get_owner_meters(&address)
                .iter()
                .map(|m| MeterOutput {
                    owner: m.owner.clone(),
                    service_id: m.service_id.clone(),
                    total_units: m.total_units(),
                    total_spent: m.total_spent(),
                    active: m.is_active(),
                    locked_deposit: m.locked_deposit(),
                })
                .collect();

            let output = MetersOutput {
                address: address.clone(),
                meters,
            };

            println!("{}", format_output(&output, &cli.format)?);
            Ok(())
        }

        Commands::Report { address } => {
            let (state, _) = load_or_create_state(&storage)?;

            let reports: Vec<ReportOutput> = if let Some(addr) = address {
                state
                    .get_owner_meters(&addr)
                    .iter()
                    .map(|m| ReportOutput {
                        account: m.owner.clone(),
                        service_id: m.service_id.clone(),
                        total_units: m.total_units(),
                        total_spent: m.total_spent(),
                        active: m.is_active(),
                        effective_unit_price: if m.total_units() > 0 {
                            Some(m.total_spent() as f64 / m.total_units() as f64)
                        } else {
                            None
                        },
                    })
                    .collect()
            } else {
                let mut all_reports = Vec::new();
                for account_addr in state.accounts.keys() {
                    for meter in state.get_owner_meters(account_addr) {
                        all_reports.push(ReportOutput {
                            account: meter.owner.clone(),
                            service_id: meter.service_id.clone(),
                            total_units: meter.total_units(),
                            total_spent: meter.total_spent(),
                            active: meter.is_active(),
                            effective_unit_price: if meter.total_units() > 0 {
                                Some(meter.total_spent() as f64 / meter.total_units() as f64)
                            } else {
                                None
                            },
                        });
                    }
                }
                all_reports
            };

            let output = ReportListOutput { reports };
            println!("{}", format_output(&output, &cli.format)?);
            Ok(())
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct AccountOutput {
    address: String,
    balance: u64,
    nonce: u64,
}

#[derive(Debug, serde::Serialize)]
struct MeterOutput {
    owner: String,
    service_id: String,
    total_units: u64,
    total_spent: u64,
    active: bool,
    locked_deposit: u64,
}

#[derive(Debug, serde::Serialize)]
struct MetersOutput {
    address: String,
    meters: Vec<MeterOutput>,
}

#[derive(Debug, serde::Serialize)]
struct ReportOutput {
    account: String,
    service_id: String,
    total_units: u64,
    total_spent: u64,
    active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    effective_unit_price: Option<f64>,
}

#[derive(Debug, serde::Serialize)]
struct ReportListOutput {
    reports: Vec<ReportOutput>,
}
