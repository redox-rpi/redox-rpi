//! mqtt-pub: publish one message to an MQTT broker, like mosquitto_pub.
//!
//! Usage: `mqtt-pub [options] -t topic (-m message | -f file | -s | -l | -n)`

#[path = "../cli.rs"]
mod cli;

use std::fs;
use std::io::{self, BufRead, Read};

use cli::{die, fail, Common};

const NAME: &str = "mqtt-pub";

fn usage() -> String {
    format!(
        "usage: mqtt-pub [options] -t topic (-m message | -f file | -s | -l | -n)
  -t topic    topic to publish to
  -m message  the message
  -f file     send the content of a file (trailing newline removed)
  -s          send all of standard input
  -l          send each line of standard input as its own message
  -n          send an empty message
  -r          retain the message on the broker
{}",
        cli::COMMON_USAGE
    )
}

enum Source {
    Message(Vec<u8>),
    Lines,
}

fn main() {
    let usage = usage();
    let mut common = Common::new(NAME);
    let mut source = None;
    let mut retain = false;
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        if common.parse(&flag, &mut args, &usage) {
            continue;
        }
        let mut value = || args.next().unwrap_or_else(|| fail(&format!("{flag} needs a value"), &usage));
        let next = match flag.as_str() {
            "-m" => Source::Message(value().into_bytes()),
            "-f" => {
                let path = value();
                let mut data = fs::read(&path).unwrap_or_else(|e| die(NAME, &format!("{path}: {e}")));
                if data.last() == Some(&b'\n') {
                    data.pop();
                }
                Source::Message(data)
            }
            "-s" => {
                let mut data = Vec::new();
                io::stdin().read_to_end(&mut data).unwrap_or_else(|e| die(NAME, &format!("stdin: {e}")));
                Source::Message(data)
            }
            "-l" => Source::Lines,
            "-n" => Source::Message(Vec::new()),
            "-r" => {
                retain = true;
                continue;
            }
            "--help" => {
                println!("{usage}");
                return;
            }
            _ => fail(&format!("unknown option {flag}"), &usage),
        };
        if source.replace(next).is_some() {
            fail("give only one of -m, -f, -s, -l, -n", &usage);
        }
    }
    let topic = match common.topics.as_slice() {
        [topic] => topic.clone(),
        _ => fail("give one topic with -t", &usage),
    };
    let source = source.unwrap_or_else(|| fail("give a message with -m, -f, -s, -l or -n", &usage));

    let mut client = common.connect(NAME);
    let mut publish = |payload: &[u8]| {
        client
            .publish(&topic, payload, common.qos, retain)
            .unwrap_or_else(|e| die(NAME, &format!("publish to {topic}: {e}")))
    };
    match source {
        Source::Message(payload) => publish(&payload),
        Source::Lines => {
            for line in io::stdin().lock().lines() {
                publish(line.unwrap_or_else(|e| die(NAME, &format!("stdin: {e}"))).as_bytes());
            }
        }
    }
    client.disconnect().unwrap_or_else(|e| die(NAME, &format!("disconnect: {e}")));
}
