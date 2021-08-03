extern crate bincode;
extern crate chrono;
extern crate dotenv;
extern crate fd_lock;
extern crate serde;
extern crate serde_json;
extern crate systemstat;

use chrono::prelude::{DateTime, Datelike, Utc};
use dotenv::dotenv;
use fd_lock::RwLock;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{self, Read, Seek, Write};
use std::process::Command;
use std::thread;
use std::time;
use std::{env, error::Error};
use systemstat::{ByteSize, Platform, System};

// tc qdisc delete dev enp0s3 root
fn unlimit_interface(interface: &String) -> Result<(), Box<dyn Error>> {
    // TODO: needs some elevation
    let output = Command::new("tc")
        .args(&["qdisc", "delete", "dev", &interface, "root"])
        .output()?;

    println!("{}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();
    Ok(())
}

// tc qdisc add dev enp0s3 root tbf rate 10mbit burst 40kbit latency 70ms
// c.f. https://unix.stackexchange.com/questions/100785/bucket-size-in-tbf
fn limit_interface(
    interface: &String,
    rate: &String,
    burst: &String,
    latency: &String,
) -> Result<(), Box<dyn Error>> {
    // TODO: calculate burst from rate
    // TODO: needs some elevation
    let output = Command::new("tc")
        .args(&[
            "qdisc", "add", "dev", &interface, "root", "tbf", "rate", &rate, "burst", &burst,
            "latency", &latency,
        ])
        .output()?;

    println!("{}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    Ok(())
}

fn get_network_stats(interface: &str) -> Result<ByteSize, Box<dyn Error>> {
    let sys = System::new();

    match sys.networks() {
        Ok(netifs) => {
            for netif in netifs.values() {
                if &netif.name == interface {
                    let stats = sys.network_stats(&netif.name)?;
                    return Ok(stats.tx_bytes);
                }
            }
        }
        Err(x) => println!("\nNetworks: error: {}", x),
    }
    Ok(ByteSize::b(0))
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct UsedStats {
    pub used: u64,
    pub last_reset: DateTime<Utc>,
}

impl UsedStats {
    fn new() -> Self {
        UsedStats {
            used: 0_u64,
            last_reset: Utc::now(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct Limit {
    pub limit: u64,
    pub rate: String,
    pub burst: String,
    pub latency: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct LimitList {
    pub limits: Vec<Limit>,
}

fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    // Read config from environment or .env file
    let interface = env::var("INTERFACE").expect("missing envvar interface");
    let path = env::var("LOGPATH").expect("missing envvar LOGPATH");
    let timeout = env::var("TIMEOUT")
        .expect("missing envvar TIMEOUT")
        .parse::<u64>()?;
    let limit_json = env::var("LIMITS").expect("missing envvar LIMITS");

    let limit_json = std::fs::read_to_string(limit_json)?;
    let limits: LimitList = serde_json::from_str(&limit_json)?;
    let mut limits = limits.limits;
    limits.sort_by(|a, b| {
        a.limit
            .partial_cmp(&b.limit)
            .unwrap_or(std::cmp::Ordering::Less)
    });
    let limits = limits.clone();
    let mut next_limit = 0_usize;

    // TODO: this still allows to remove the file
    // and modify it with a different process
    let mut f = RwLock::new(
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?,
    );

    let mut f = f.try_write()?;

    // TODO: not safe
    let mut buf = [0; 2000];
    let k = f.read(&mut buf)?;

    let mut use_stats = if k > 0 {
        bincode::deserialize(&buf[..])?
    } else {
        UsedStats::new()
    };

    let start = get_network_stats(&interface)?;
    println!("start\t{}", start);
    println!("start duration\t {}", use_stats.last_reset.timestamp());
    println!("start used\t {}", ByteSize::b(use_stats.used));
    let timeout = time::Duration::from_millis(timeout);

    let mut orig = use_stats.clone();

    unlimit_interface(&interface)?;

    loop {
        let cur = get_network_stats(&interface)?;
        //println!("cur\t{}", cur);
        let now_used = ByteSize::b(cur.as_u64() - start.as_u64());
        use_stats.used = now_used.as_u64() + orig.used;

        //println!("now_used\t{}", now_used);
        //println!("used\t{}", ByteSize::b(use_stats.used));

        let now = Utc::now();
        if (now.month() > use_stats.last_reset.month()
            || (now.year() > use_stats.last_reset.year() && now.month() == 1))
            && now.day() == 1
        {
            println!("releasing limits");
            use_stats.last_reset = now;
            use_stats.used = 0;
            orig.used = 0;
            next_limit = 0;
            unlimit_interface(&interface)?;
        }

        if next_limit < limits.len() && use_stats.used > limits[next_limit].limit {
            println!("activating limit\t{}", limits[next_limit].limit);
            unlimit_interface(&interface)?;
            limit_interface(
                &interface,
                &limits[next_limit].rate,
                &limits[next_limit].burst,
                &limits[next_limit].latency,
            )?;

            next_limit += 1;
        }

        let bytes = bincode::serialize(&use_stats)?;
        f.seek(std::io::SeekFrom::Start(0))?;
        f.write(&bytes[..])?;

        thread::sleep(timeout);
    }
}
