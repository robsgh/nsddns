use std::path::PathBuf;

use clap::Parser;

use nsddns::{get_current_ip, get_namesilo_a_record, parse_config, update_namesilo_a_record};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Configuration file to read from
    #[arg(short, long, default_value = "/etc/nsddns/conf.json")]
    config: PathBuf,

    /// Do not update the resource record
    #[arg(long)]
    dry_run: bool,
}

fn run_nsddns(cfg: PathBuf, dry_run: bool) {
    let config = parse_config(cfg).expect("config file should be valid JSON with all keys");

    println!("Fetching DNS information...");
    let resource_record = match get_namesilo_a_record(&config) {
        Ok(dns) => dns,
        Err(e) => {
            println!("ERROR: Failed to fetch DNS A record from Namesilo: {:?}", e);
            return;
        }
    };

    println!("Fetching current IP address...");
    let current_ip = match get_current_ip() {
        Ok(ip) => ip,
        Err(e) => {
            println!("ERROR: failed to fetch current IP address: {:?}", e);
            return;
        }
    };

    println!(
        "DNS record value: {}.\nCurrent IP is {}.\n",
        resource_record.record_value, current_ip,
    );
    if resource_record.record_value == current_ip {
        println!("Nothing to do.");
        return;
    }

    println!("Updating record....");
    if dry_run {
        println!(
            "DRY RUN: would have updated DNS record of {:?} to {}.",
            resource_record, current_ip
        );
        return;
    }

    match update_namesilo_a_record(&config, &resource_record, &current_ip) {
        Ok(()) => println!("DNS record updated successfully"),
        Err(e) => {
            println!("ERROR: failed to update DNS record: {:?}", e);
        }
    }
}

fn main() {
    let args = Args::parse();

    let cfg = args.config;
    println!("Loading configuration from {}...", cfg.to_string_lossy());

    match cfg.try_exists() {
        Ok(true) => run_nsddns(cfg, args.dry_run),
        Ok(false) => {
            println!(
                "ERROR: Config file at {} does not exist",
                cfg.to_string_lossy()
            );
        }
        Err(e) => {
            println!(
                "ERROR: Failed to read config file {}: {:?}",
                cfg.to_string_lossy(),
                e
            );
        }
    }
}
