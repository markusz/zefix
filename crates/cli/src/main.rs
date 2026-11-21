mod report;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use report::RunInfo;
use schafsim_core::scenario::Scenario;
use schafsim_core::sim;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Parser)]
#[command(version, about = "Schafkopf-Simulator")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Simulate a scenario
    Run(RunArgs),
    /// Validate a scenario without simulating
    Check { config: PathBuf },
}

#[derive(Args)]
struct RunArgs {
    config: PathBuf,
    /// Number of deals, e.g. 1e6 or 500_000
    #[arg(short = 'n', long, value_parser = parse_count)]
    iterations: Option<u64>,
    #[arg(long)]
    seed: Option<u64>,
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
    /// Print up to N matching deals per query
    #[arg(long, default_value_t = 0)]
    examples: usize,
    #[arg(long)]
    threads: Option<usize>,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Json,
}

fn parse_count(s: &str) -> Result<u64, String> {
    let s = s.replace('_', "");
    let n = if s.contains(['e', 'E']) {
        let f: f64 = s.parse().map_err(|_| format!("keine Zahl: {s}"))?;
        if f.fract() != 0.0 || f > u64::MAX as f64 {
            return Err(format!("keine ganze Zahl: {s}"));
        }
        f as u64
    } else {
        s.parse().map_err(|_| format!("keine Zahl: {s}"))?
    };
    if n == 0 {
        return Err("muss > 0 sein".into());
    }
    Ok(n)
}

fn load(path: &Path) -> Result<Scenario> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("kann {} nicht lesen", path.display()))?;
    Ok(Scenario::from_toml(&text)?)
}

fn check(config: &Path) -> Result<()> {
    let scn = load(config)?;
    println!("{}", scn.game.describe());
    println!("Hand: {}", scn.describe_hand());
    println!("Mögliche Hände: {}", scn.possible_hands());
    println!("Queries: {}", scn.queries.len());
    Ok(())
}

fn time_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64)
}

fn run(args: RunArgs) -> Result<()> {
    let scn = load(&args.config)?;
    if let Some(threads) = args.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .context("Thread-Pool")?;
    }
    let iterations = args.iterations.unwrap_or(scn.iterations);
    let seed = args.seed.or(scn.seed).unwrap_or_else(time_seed);
    let start = Instant::now();
    let outcome = sim::run(&scn, iterations, seed, args.examples)?;
    let info = RunInfo {
        seed,
        elapsed: start.elapsed(),
    };
    match args.format {
        Format::Text => print!("{}", report::text(&scn, &outcome, &info)),
        Format::Json => {
            let value = report::json(&scn, &outcome, &info);
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Check { config } => check(&config),
        Cmd::Run(args) => run(args),
    }
}
