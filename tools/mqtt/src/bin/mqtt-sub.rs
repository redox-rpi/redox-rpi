//! mqtt-sub: print messages from an MQTT broker, like mosquitto_sub.
//!
//! Usage: `mqtt-sub [options] -t filter [-t filter]... [-v] [-C count]`

#[path = "../cli.rs"]
mod cli;

use std::io::{self, Write};

use cli::{die, fail, Common};

const NAME: &str = "mqtt-sub";

fn usage() -> String {
    format!(
        "usage: mqtt-sub [options] -t filter [-t filter]... [-v] [-C count]
  -t filter   topic filter to subscribe to, + and # allowed (repeatable)
  -v          print the topic before each message
  -C count    exit after this many messages
{}",
        cli::COMMON_USAGE
    )
}

fn main() {
    let usage = usage();
    let mut common = Common::new(NAME);
    let mut verbose = false;
    let mut count: Option<u64> = None;
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        if common.parse(&flag, &mut args, &usage) {
            continue;
        }
        match flag.as_str() {
            "-v" => verbose = true,
            "-C" => {
                let value = args.next().unwrap_or_else(|| fail("-C needs a value", &usage));
                count = Some(value.parse().unwrap_or_else(|_| fail("bad count", &usage)));
            }
            "--help" => {
                println!("{usage}");
                return;
            }
            _ => fail(&format!("unknown option {flag}"), &usage),
        }
    }
    if common.topics.is_empty() {
        fail("give at least one topic filter with -t", &usage);
    }

    let mut client = common.connect(NAME);
    let filters: Vec<(&str, mqtt::QoS)> = common.topics.iter().map(|t| (t.as_str(), common.qos)).collect();
    client.subscribe(&filters).unwrap_or_else(|e| die(NAME, &format!("subscribe: {e}")));

    let mut stdout = io::stdout().lock();
    let mut received = 0u64;
    while count.map_or(true, |count| received < count) {
        let message = client.recv().unwrap_or_else(|e| die(NAME, &format!("connection lost: {e}")));
        if verbose {
            let _ = write!(stdout, "{} ", message.topic);
        }
        let _ = stdout.write_all(&message.payload);
        let _ = stdout.write_all(b"\n");
        let _ = stdout.flush();
        received += 1;
    }
    let _ = client.disconnect();
}
