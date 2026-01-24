use clap::Parser;
use num_bigint::{BigInt, Sign};
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::fs;
use std::io::{self, Read, Write};

#[derive(Parser, Debug)]
#[command(about = "Convert SIM Dune activity JSON to Metering Chain Consume txs")]
struct Args {
    #[arg(short, long)]
    input: Option<String>,

    #[arg(short, long)]
    output: Option<String>,

    #[arg(long)]
    token_address: String,

    #[arg(long)]
    signer: String,

    #[arg(long)]
    owner: Option<String>,

    #[arg(long, default_value = "dimo-rewards")]
    service_id: String,

    #[arg(long, default_value_t = 1)]
    start_nonce: u64,

    #[arg(long, default_value_t = 1)]
    unit_price: u64,

    /// Override token decimals (otherwise read from token_metadata.decimals)
    #[arg(long)]
    decimals: Option<u32>,

    /// Target decimals to keep in units (<= token decimals)
    #[arg(long)]
    target_decimals: Option<u32>,

    /// Rounding mode when scaling (floor|reject)
    #[arg(long, default_value = "floor")]
    rounding: String,

    /// Allow zero units (otherwise skip)
    #[arg(long)]
    allow_zero: bool,

    /// Do not sort by block_time
    #[arg(long)]
    no_sort: bool,
}

#[derive(Clone, Copy, Debug)]
enum Rounding {
    Floor,
    Reject,
}

impl Rounding {
    fn parse(s: &str) -> Result<Self, String> {
        match s {
            "floor" => Ok(Self::Floor),
            "reject" => Ok(Self::Reject),
            _ => Err(format!("invalid rounding mode: {s}")),
        }
    }
}

#[derive(Debug)]
struct Item {
    idx: usize,
    block_time: Option<String>,
    units: BigInt,
}

fn parse_bigint(v: &Value) -> Option<BigInt> {
    match v {
        Value::String(s) => s.parse().ok(),
        Value::Number(n) => n.to_string().parse().ok(),
        _ => None,
    }
}

fn parse_u32(v: &Value) -> Option<u32> {
    match v {
        Value::Number(n) => n.as_u64().and_then(|x| u32::try_from(x).ok()),
        Value::String(s) => s.parse::<u32>().ok(),
        _ => None,
    }
}

fn pow10(exp: u32) -> BigInt {
    let mut result = BigInt::from(1);
    let ten = BigInt::from(10);
    for _ in 0..exp {
        result *= &ten;
    }
    result
}

fn bigint_to_u64(v: &BigInt) -> Option<u64> {
    let (sign, mut digits) = v.to_u64_digits();
    if sign == Sign::Minus {
        return None;
    }
    if digits.len() > 1 {
        return None;
    }
    Some(digits.pop().unwrap_or(0))
}

fn main() -> Result<(), String> {
    let args = Args::parse();
    let Args {
        input,
        output,
        token_address,
        signer,
        owner,
        service_id,
        start_nonce,
        unit_price,
        decimals,
        target_decimals: target_decimals_opt,
        rounding,
        allow_zero,
        no_sort,
    } = args;

    let input_str = match input {
        Some(path) => fs::read_to_string(path).map_err(|e| e.to_string())?,
        None => {
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| e.to_string())?;
            buf
        }
    };

    let root: Value = serde_json::from_str(&input_str).map_err(|e| e.to_string())?;
    let activity = root
        .get("activity")
        .and_then(|v| v.as_array())
        .ok_or("missing activity array")?;

    let token_lc = token_address.to_lowercase();
    let rounding = Rounding::parse(&rounding)?;

    let mut items: Vec<Item> = Vec::new();
    let mut truncated = 0usize;

    for (idx, entry) in activity.iter().enumerate() {
        let ty = entry.get("type").and_then(|v| v.as_str());
        let asset_type = entry.get("asset_type").and_then(|v| v.as_str());
        if ty != Some("send") || asset_type != Some("erc20") {
            continue;
        }

        let token = entry
            .get("token_address")
            .and_then(|v| v.as_str())
            .map(|s| s.to_lowercase());
        if token.as_deref() != Some(&token_lc) {
            continue;
        }

        let raw_value = entry
            .get("value")
            .and_then(parse_bigint)
            .ok_or("missing or invalid value")?;
        if raw_value.sign() == Sign::Minus {
            continue;
        }

        let token_decimals = match decimals {
            Some(d) => d,
            None => entry
                .get("token_metadata")
                .and_then(|m| m.get("decimals"))
                .and_then(parse_u32)
                .ok_or("missing token decimals; provide --decimals")?,
        };

        let target_decimals = target_decimals_opt.unwrap_or(token_decimals);
        if target_decimals > token_decimals {
            return Err("target_decimals cannot exceed token decimals".to_string());
        }

        let divisor = pow10(token_decimals - target_decimals);
        let remainder = &raw_value % &divisor;
        let units = &raw_value / &divisor;

        if remainder != BigInt::from(0) {
            match rounding {
                Rounding::Floor => {
                    truncated += 1;
                }
                Rounding::Reject => {
                    return Err("non-zero remainder when scaling; use --rounding floor".to_string());
                }
            }
        }

        if units.sign() == Sign::Minus {
            continue;
        }

        if units == BigInt::from(0) && !allow_zero {
            continue;
        }

        let block_time = entry
            .get("block_time")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        items.push(Item {
            idx,
            block_time,
            units,
        });
    }

    if !no_sort {
        items.sort_by(|a, b| match (&a.block_time, &b.block_time) {
            (Some(ta), Some(tb)) => ta.cmp(tb).then_with(|| a.idx.cmp(&b.idx)),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => a.idx.cmp(&b.idx),
        });
    }

    if truncated > 0 {
        eprintln!("warning: truncated {truncated} values when scaling");
    }

    let owner = owner.unwrap_or_else(|| signer.clone());
    let mut out = String::new();

    for (i, item) in items.iter().enumerate() {
        let nonce = start_nonce
            .checked_add(i as u64)
            .ok_or("nonce overflow")?;

        let units_u64 = bigint_to_u64(&item.units).ok_or("units overflow u64")?;

        let tx = json!({
            "signer": &signer,
            "nonce": nonce,
            "kind": {
                "Consume": {
                    "owner": &owner,
                    "service_id": &service_id,
                    "units": units_u64,
                    "pricing": { "UnitPrice": unit_price }
                }
            }
        });

        out.push_str(&serde_json::to_string(&tx).map_err(|e| e.to_string())?);
        out.push('\n');
    }

    match output {
        Some(path) => fs::write(path, out).map_err(|e| e.to_string())?,
        None => {
            let mut stdout = io::stdout();
            stdout.write_all(out.as_bytes()).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}
