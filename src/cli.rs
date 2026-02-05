use clap::{Parser, Subcommand};
use metering_chain::config::Config;
use metering_chain::error::{Error, Result};
use metering_chain::state::{apply, State};
use metering_chain::storage::{FileStorage, Storage};
use metering_chain::tx::validation::ValidationContext;
use metering_chain::tx::SignedTx;
use metering_chain::wallet;
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

        /// Allow unsigned transactions (legacy/Phase 1). Signed tx still verified.
        #[arg(long)]
        allow_unsigned: bool,
    },

    /// Wallet operations (Phase 2: create, list, sign)
    Wallet {
        #[command(subcommand)]
        sub: WalletSub,
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

#[derive(Subcommand)]
pub enum WalletSub {
    /// Create a new wallet (keypair, address = hex of pubkey)
    Create,

    /// List all wallet addresses
    List,

    /// Sign a transaction (needs current nonce from state)
    Sign {
        /// Wallet address (signer)
        #[arg(short, long)]
        address: String,

        /// JSON file with transaction kind only, e.g. {"Consume":{"owner":"0x...","service_id":"s","units":1,"pricing":{"UnitPrice":2}}}
        #[arg(short, long)]
        file: String,

        /// Delegated sign: consume on behalf of this owner (nonce_account). Requires --nonce and proof/valid_at.
        #[arg(long)]
        for_owner: Option<String>,

        /// Nonce to use (required for delegated sign; otherwise read from state for signer/owner)
        #[arg(long)]
        nonce: Option<u64>,

        /// Reference time (valid_at) for delegated consume. Default: current Unix time.
        #[arg(long)]
        valid_at: Option<u64>,

        /// Path to file containing delegation proof bytes (owner-signed; use create-delegation-proof). Required for --for-owner.
        #[arg(long)]
        proof_file: Option<String>,
    },

    /// Create owner-signed delegation proof (write to file). Issuer = owner address, audience = delegate.
    CreateDelegationProof {
        /// Owner wallet address (issuer; must exist in wallets)
        #[arg(short, long)]
        address: String,

        /// Delegate address (audience)
        #[arg(long)]
        audience: String,

        /// Scope: service_id (must match Consume tx service_id)
        #[arg(long)]
        service_id: String,

        /// Scope: ability (e.g. "consume"; optional; if set must match tx type)
        #[arg(long)]
        ability: Option<String>,

        /// Issued-at time (Unix seconds)
        #[arg(long)]
        iat: u64,

        /// Expiry time (Unix seconds)
        #[arg(long)]
        exp: u64,

        /// Optional caveat: max units for this capability
        #[arg(long)]
        max_units: Option<u64>,

        /// Optional caveat: max cost for this capability
        #[arg(long)]
        max_cost: Option<u64>,

        /// Output file path for proof bytes
        #[arg(short, long)]
        output: String,
    },

    /// Create owner-signed RevokeDelegation transaction (output JSON; apply separately).
    RevokeDelegation {
        /// Owner wallet address (signer; must exist in wallets)
        #[arg(short, long)]
        address: String,

        /// Capability ID to revoke (e.g. from capability_id(proof_bytes), lowercase hex)
        #[arg(short, long)]
        capability_id: String,

        /// Nonce to use; if omitted, read from state for owner
        #[arg(long)]
        nonce: Option<u64>,

        /// Write signed tx JSON to file instead of stdout
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Print capability_id (sha256 hex) for a delegation proof file. Use with revoke-delegation --capability-id.
    CapabilityId {
        /// Path to file containing delegation proof bytes
        #[arg(short, long)]
        proof_file: String,
    },
}

/// Load state from storage or return genesis state
pub fn load_or_create_state(storage: &FileStorage, _config: &Config) -> Result<(State, u64)> {
    match storage.load_state()? {
        Some((state, last_tx_id)) => {
            let txs = storage.load_txs_from(last_tx_id + 1)?;
            let mut current_state = state;
            let mut current_tx_id = last_tx_id;
            let replay_ctx = ValidationContext::replay();
            for tx in txs {
                if tx.signature.is_some() {
                    wallet::verify_signature(&tx)?;
                }
                current_state = apply(&current_state, &tx, &replay_ctx, None)?;
                current_tx_id += 1;
            }

            Ok((current_state, current_tx_id))
        }
        None => Ok((State::new(), 0)),
    }
}

/// Authorized minters: "authority" (legacy) + METERING_CHAIN_MINTERS env (comma-separated addresses).
/// Only explicitly listed addresses can mint; locally-created wallets are not minters by default.
fn get_authorized_minters(_config: &Config) -> HashSet<String> {
    let mut minters = HashSet::new();
    minters.insert("authority".to_string());
    if let Ok(list) = std::env::var("METERING_CHAIN_MINTERS") {
        for addr in list.split(',') {
            let a = addr.trim();
            if !a.is_empty() {
                minters.insert(a.to_string());
            }
        }
    }
    minters
}

/// Parse transaction from JSON string
fn parse_tx(json: &str) -> Result<SignedTx> {
    let tx: SignedTx = serde_json::from_str(json).map_err(|e| {
        Error::InvalidTransaction(format!("Failed to parse transaction JSON: {}", e))
    })?;
    Ok(tx)
}

/// Read transaction from file or stdin
fn read_tx(file: Option<&str>) -> Result<String> {
    match file {
        Some(path) => fs::read_to_string(path)
            .map_err(|e| Error::InvalidTransaction(format!("Failed to read file {}: {}", path, e))),
        None => {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer).map_err(|e| {
                Error::InvalidTransaction(format!("Failed to read from stdin: {}", e))
            })?;
            Ok(buffer)
        }
    }
}

/// Format output based on format type
fn format_output<T: serde::Serialize + std::fmt::Debug>(data: &T, format: &str) -> Result<String> {
    match format {
        "json" => serde_json::to_string_pretty(data)
            .map_err(|e| Error::StateError(format!("Failed to serialize JSON: {}", e))),
        _ => {
            // Human-readable format (simple debug output for now)
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
    let minters = get_authorized_minters(&config);

    match cli.command {
        Commands::Init => {
            // Create data directory
            fs::create_dir_all(config.get_data_dir()).map_err(|e| {
                Error::StateError(format!("Failed to create data directory: {}", e))
            })?;
            println!(
                "Initialized data directory at: {}",
                config.get_data_dir().display()
            );
            Ok(())
        }

        Commands::Apply {
            tx,
            file,
            dry_run,
            allow_unsigned,
        } => {
            // Load current state
            let (mut state, mut last_tx_id) = load_or_create_state(&storage, &config)?;

            // Read transaction
            let tx_json = match tx {
                Some(json) => json,
                None => read_tx(file.as_deref())?,
            };

            let signed_tx = parse_tx(&tx_json)?;

            // Phase 2: require valid signature for user-submitted tx unless explicitly allowed
            if signed_tx.signature.is_some() {
                wallet::verify_signature(&signed_tx)?;
            } else if !allow_unsigned {
                return Err(Error::SignatureVerification(
                    "Unsigned tx rejected (use --allow-unsigned for legacy apply)".to_string(),
                ));
            } else {
                eprintln!("Warning: applying unsigned transaction (legacy/unsafe)");
            }

            let now = metering_chain::current_timestamp().max(0) as u64;
            const DEFAULT_MAX_AGE: u64 = 300;
            let live_ctx = ValidationContext::live(now, DEFAULT_MAX_AGE);

            // Validate transaction
            let cost_opt = metering_chain::tx::validation::validate(
                &state,
                &signed_tx,
                &live_ctx,
                Some(&minters),
            )?;

            if dry_run {
                println!("Transaction is valid");
                if let Some(cost) = cost_opt {
                    println!("  Cost: {}", cost);
                }
                return Ok(());
            }

            // Apply transaction
            state = apply(&state, &signed_tx, &live_ctx, Some(&minters))?;
            last_tx_id += 1;

            // Persist transaction and state
            storage.append_tx(&signed_tx)?;
            storage.persist_state(&state, last_tx_id)?;

            println!("Transaction applied successfully");
            if let Some(cost) = cost_opt {
                println!("  Cost: {}", cost);
            }

            Ok(())
        }

        Commands::Wallet { sub } => match sub {
            WalletSub::Create => {
                fs::create_dir_all(config.get_data_dir()).map_err(|e| {
                    Error::StateError(format!("Failed to create data directory: {}", e))
                })?;
                let mut wallets =
                    metering_chain::wallet::Wallets::new(config.get_wallets_path().clone());
                let address = wallets.create_wallet()?;
                println!("Created wallet: {}", address);
                Ok(())
            }
            WalletSub::List => {
                let wallets =
                    metering_chain::wallet::Wallets::new(config.get_wallets_path().clone());
                let addrs = wallets.get_addresses();
                if addrs.is_empty() {
                    println!("No wallets. Run: metering-chain wallet create");
                    return Ok(());
                }
                for a in addrs {
                    println!("{}", a);
                }
                Ok(())
            }
            WalletSub::Sign {
                address,
                file,
                for_owner,
                nonce,
                valid_at,
                proof_file,
            } => {
                let (state, _) = load_or_create_state(&storage, &config)?;
                let kind_json = fs::read_to_string(&file).map_err(|e| {
                    Error::InvalidTransaction(format!("Failed to read {}: {}", file, e))
                })?;
                let kind: metering_chain::tx::Transaction = serde_json::from_str(&kind_json)
                    .map_err(|e| Error::InvalidTransaction(format!("Invalid kind JSON: {}", e)))?;
                let wallets =
                    metering_chain::wallet::Wallets::new(config.get_wallets_path().clone());

                let signed = if let Some(owner) = for_owner {
                    let nonce_val = nonce.ok_or_else(|| {
                        Error::InvalidTransaction("Delegated sign requires --nonce".to_string())
                    })?;
                    let proof_bytes = match &proof_file {
                        Some(path) => fs::read(path).map_err(|e| {
                            Error::InvalidTransaction(format!("Failed to read proof file: {}", e))
                        })?,
                        None => {
                            return Err(Error::InvalidTransaction(
                                "Delegated sign requires --proof-file".to_string(),
                            ));
                        }
                    };
                    let valid_at_sec = valid_at
                        .unwrap_or_else(|| metering_chain::current_timestamp().max(0) as u64);
                    wallets.sign_transaction_v2(
                        &address,
                        nonce_val,
                        owner,
                        valid_at_sec,
                        proof_bytes,
                        kind,
                    )?
                } else {
                    let nonce_val = nonce.unwrap_or_else(|| {
                        state.get_account(&address).map(|a| a.nonce()).unwrap_or(0)
                    });
                    wallets.sign_transaction(&address, nonce_val, kind)?
                };
                println!("{}", serde_json::to_string_pretty(&signed).unwrap());
                Ok(())
            }
            WalletSub::CreateDelegationProof {
                address,
                audience,
                service_id,
                ability,
                iat,
                exp,
                max_units,
                max_cost,
                output,
            } => {
                let wallets =
                    metering_chain::wallet::Wallets::new(config.get_wallets_path().clone());
                let wallet = wallets.get_wallet(&address).ok_or_else(|| {
                    Error::InvalidTransaction(format!("Wallet not found: {}", address))
                })?;
                let claims = metering_chain::tx::DelegationProofMinimal {
                    iat,
                    exp,
                    issuer: address.clone(),
                    audience: audience.clone(),
                    service_id: service_id.clone(),
                    ability: ability.clone(),
                    max_units: max_units.clone(),
                    max_cost: max_cost.clone(),
                };
                let proof_bytes = wallet.sign_delegation_proof(&claims);
                fs::write(output, &proof_bytes).map_err(|e| {
                    Error::StateError(format!("Failed to write proof file: {}", e))
                })?;
                println!("Created signed delegation proof: {} bytes", proof_bytes.len());
                Ok(())
            }
            WalletSub::RevokeDelegation {
                address,
                capability_id,
                nonce,
                output,
            } => {
                let (state, _) = load_or_create_state(&storage, &config)?;
                let nonce_val = nonce.unwrap_or_else(|| {
                    state.get_account(&address).map(|a| a.nonce()).unwrap_or(0)
                });
                let wallets =
                    metering_chain::wallet::Wallets::new(config.get_wallets_path().clone());
                let kind = metering_chain::tx::Transaction::RevokeDelegation {
                    owner: address.clone(),
                    capability_id: capability_id.clone(),
                };
                let signed = wallets.sign_transaction(&address, nonce_val, kind)?;
                let json = serde_json::to_string_pretty(&signed).unwrap();
                if let Some(path) = output {
                    fs::write(&path, &json).map_err(|e| {
                        Error::StateError(format!("Failed to write {}: {}", path, e))
                    })?;
                    println!("Wrote signed RevokeDelegation to {}", path);
                } else {
                    println!("{}", json);
                }
                Ok(())
            }
            WalletSub::CapabilityId { proof_file } => {
                let bytes = fs::read(&proof_file)
                    .map_err(|e| Error::StateError(format!("Failed to read {}: {}", proof_file, e)))?;
                let cap_id = metering_chain::tx::capability_id(&bytes);
                println!("{}", cap_id);
                Ok(())
            }
        },

        Commands::Account { address } => {
            let (state, _) = load_or_create_state(&storage, &config)?;

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
                None => Err(Error::StateError(format!("Account {} not found", address))),
            }
        }

        Commands::Meters { address } => {
            let (state, _) = load_or_create_state(&storage, &config)?;

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
            let (state, _) = load_or_create_state(&storage, &config)?;

            let reports: Vec<ReportOutput> = if let Some(addr) = address {
                // Single account report
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
                // All accounts report - iterate through all accounts and their meters
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
