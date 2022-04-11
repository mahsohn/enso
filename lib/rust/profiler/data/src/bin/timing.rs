//! (TODO)
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

use enso_profiler_data as data;
use enso_profiler_data::{Error, Profile};

// ==========================
// === Render to PlantUML ===
// ==========================

mod uml {
    use std::fmt::Formatter;
    use super::*;

    #[derive(Clone, Copy)]
    pub struct Participant(&'static str);

    pub struct Message {
        p0: Participant,
        p1: Participant,
        time: f64,
        label: String,
    }

    #[derive(Default)]
    pub struct Uml {
        participants: Vec<Participant>,
        messages: Vec<Message>,
    }

    impl Uml {
        pub fn participant(&mut self, name: &'static str) -> Participant {
            let participant = Participant(name);
            self.participants.push(participant);
            participant
        }

        pub fn message(&mut self, p0: Participant, p1: Participant, time: f64, label: String) {
            self.messages.push(Message { p0, p1, time, label });
        }
    }

    impl std::fmt::Display for Uml {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            writeln!(f, "@startuml")?;
            for p in &self.participants {
                writeln!(f, "concise \"{}\" as {}", p.0, p.0)?;
            }
            for m in &self.messages {
                writeln!(f, "@{}", m.time)?;
                writeln!(f, "{} -> {} : {}", m.p0.0, m.p1.0, m.label)?;
            }
            Ok(())
        }
    }
}



// ============
// === main ===
// ============

// TODO[kw]: share this with merge.rs
#[derive(Clone, Debug, serde::Deserialize)]
pub struct BackendMessage {
    direction: String,
    request_id: Option<String>,
    endpoint: String,
}

#[derive(Clone, serde::Deserialize)]
enum Metadata {
    #[serde(rename = "RpcEvent")]
    RpcMessage(String),
    #[serde(rename = "enso.BackendMessage")]
    BackendMessage(BackendMessage),
}

fn collect_metadata<M: Clone>(profile: &data::Profile<M>, interval: data::IntervalId, metadata:
&mut Vec<data::Metadata<M>>) {
    let interval = &profile[interval];
    metadata.extend(interval.metadata.iter().cloned());
    for &child in &interval.children {
        collect_metadata(profile, child, metadata);
    }
}

fn main() {
    use std::io::Read;

    let mut profile = String::new();
    std::io::stdin().read_to_string(&mut profile).unwrap();
    let profiles: Vec<Result<data::Profile<Metadata>, data::Error<_>>> =
        data::parse_multiprocess_profile(&profile).collect();
    let mut profiles_ = Vec::new();
    for profile in profiles {
        match profile {
            Ok(profile) => profiles_.push(profile),
            Err(data::Error::RecoverableFormatError { with_missing_data, ..  }) =>
                profiles_.push(with_missing_data.unwrap()),
            Err(e) => panic!("{}", e),
        }
    }
    let profiles = profiles_;
    assert_eq!(profiles.len(), 2);

    let mut metadata0 = vec![];
    let mut metadata1 = vec![];
    collect_metadata(&profiles[0], profiles[0].root_interval_id(), &mut metadata0);
    collect_metadata(&profiles[1], profiles[1].root_interval_id(), &mut metadata1);
    let mut uml = uml::Uml::default();
    let frontend = uml.participant("ide");
    let ls = uml.participant("ls");
    let engine = uml.participant("engine");
    let offset0 = profiles[0].time_offset.unwrap().0;
    let offset1 = profiles[1].time_offset.unwrap().0;
    for meta in metadata0.into_iter() {
        if let Metadata::RpcMessage(message) = meta.data {
            let time = meta.mark.into_ms() + offset0;
            uml.message(ls, frontend, time, message);
        }
    }
    for meta in metadata1.into_iter() {
        if let Metadata::BackendMessage(message) = meta.data {
            let time = meta.mark.into_ms() + offset1;
            let (p0, p1) = match message.direction.as_str() {
                "Request" => (ls, engine),
                "Response" => (engine, ls),
                _ => panic!(),
            };
            uml.message(p0, p1, time, message.endpoint);
        }
    }
    println!("{}", uml.to_string());
}
