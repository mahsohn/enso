//! Tool that combines profiles captured by different Enso processes into one multi-process profile.
//!
//! # Usage
//!
//! The tool reads a
//! [JSON-formatted event log](https://github.com/enso-org/design/blob/main/epics/profiling/implementation.md#file-format)
//! and a backend message log, and combines them into a multi-process profile.
//!
//! For example:
//!
//! ```console
//! ~/git/enso/data $ cargo run --bin merge -- profile.json backend-messages.csv > multiprofile.json
//! ```

// === Features ===
#![feature(test)]
// === Standard Linter Configuration ===
#![deny(non_ascii_idents)]
#![warn(unsafe_code)]
// === Non-Standard Linter Configuration ===
#![deny(unconditional_recursion)]
#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_import_braces)]

use enso_profiler as profiler;
use enso_profiler::build::ProfileBuilder;
use enso_profiler::internal::Timestamp;
use enso_profiler_data as data;

use std::str::FromStr;



// ==================================
// === Backend message log format ===
// ==================================

mod backend {
    use super::*;

    #[derive(Clone, Debug, serde::Serialize)]
    pub struct Uuid(String);

    #[derive(Clone, Debug)]
    pub struct Message {
        pub timestamp:  chrono::DateTime<chrono::offset::Utc>,
        pub direction:  Direction,
        pub request_id: Option<Uuid>,
        pub endpoint:   String,
    }

    impl FromStr for Message {
        type Err = String;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let fields: Vec<_> = s.split(',').collect();
            if let &[timestamp, direction, request_id, endpoint] = &fields[..] {
                let timestamp = chrono::DateTime::parse_from_rfc3339(timestamp).unwrap().into();
                let direction = direction.parse().unwrap();
                let request_id = match request_id {
                    "" => None,
                    id => Some(Uuid(id.into())),
                };
                let endpoint = endpoint.into();
                Ok(Self { timestamp, direction, request_id, endpoint })
            } else {
                Err("Wrong number of fields?".into())
            }
        }
    }

    pub fn parse_log(log: &str) -> impl Iterator<Item=Message> + '_ {
        log
            .split('\n')
            .filter(|x| !x.is_empty())
            .map(|line| line.parse().unwrap())
    }
}



// ======================
// === BackendMessage ===
// ======================

/// `profiler` metadata type for messages between the Language Server and the Compiler.
#[derive(Clone, Debug, serde::Serialize)]
pub struct BackendMessage {
    direction: Direction,
    request_id: Option<backend::Uuid>,
    endpoint: String,
}


// === Direction ===

/// Identifies whether the primary process is the sender or receiver of the message.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub enum Direction {
    /// Process1 to Process2
    Request,
    /// Process2 to Process1
    Response,
}

impl FromStr for Direction {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Request" => Ok(Direction::Request),
            "Response" => Ok(Direction::Response),
            _ => Err(()),
        }
    }
}



// ============
// === main ===
// ============

fn main() {
    use std::fs;
    use std::io::Write;

    let mut args = std::env::args();
    let _ = args.next().unwrap();
    let profile_path = args.next().unwrap();
    //let profile: data::Profile<()> = log.parse().unwrap();
    let messages_path = args.next().unwrap();
    //let profile = fs::read_to_string("profile.json").unwrap();
    let backend_log = fs::read_to_string(messages_path).unwrap();
    let backend_messages = backend::parse_log(&backend_log);
    let mut backend_profile = ProfileBuilder::<String>::new();
    backend_profile.time_offset(profiler::TimeOffset(0.0));
    backend_profile.metadata(Timestamp::default(), "enso.Process1", "LanguageServer");
    backend_profile.metadata(Timestamp::default(), "enso.Process2", "Engine");
    for message in backend_messages {
        let backend::Message { timestamp, direction, request_id, endpoint } = message;
        let data = BackendMessage { direction, request_id, endpoint };
        let timestamp = timestamp.timestamp_millis();
        let timestamp = profiler::internal::Timestamp::from_ms(timestamp as f64);
        backend_profile.metadata(timestamp, "enso.BackendMessage", data);
    }
    let backend_profile = backend_profile.to_string();
    //std::io::stdout().write_all(ide_profile.as_bytes()).unwrap();
    std::io::stdout().write_all(backend_profile.as_bytes()).unwrap();
}
