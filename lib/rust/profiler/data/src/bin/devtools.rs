//! Tool that generates Chrome DevTools-compatible files from profiling interval data.
//!
//! # Usage
//!
//! The tool reads a JSON-formatted event log from stdin, and writes a report to stdout.
//!
//! For example:
//!
//! ```console
//! ~/git/enso/data $ cargo run --bin intervals < profile.json > devtools.json
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



/// Support for the Chrome DevTools profile format.
mod devtools {
    // =============
    // === Event ===
    // =============

    /// DevTools-profile interval.
    #[derive(serde::Serialize)]
    pub struct Event {
        pub name:         String,
        #[serde(rename = "cat")]
        pub category:     String,
        #[serde(rename = "ph")]
        pub event_type:   EventType,
        #[serde(rename = "ts")]
        pub timestamp_us: u64,
        #[serde(rename = "dur")]
        pub duration_us:  u64,
        #[serde(rename = "pid")]
        pub process_id:   u32,
        #[serde(rename = "tid")]
        pub thread_id:    u32,
        // Actually a type of map, but we don't need to write anything there.
        pub args:         Option<()>,
    }

    /// Information about type of event in DevTools profiling interval.
    #[derive(Clone, Copy, Eq, PartialEq, serde::Serialize)]
    pub enum EventType {
        #[serde(rename = "X")]
        Complete,
    }
}



// ============
// === main ===
// ============

fn main() {
    use std::io::Read;
    let mut log = String::new();
    std::io::stdin().read_to_string(&mut log).unwrap();
    let profile: data::Profile<()> = log.parse().unwrap();
    let events = IntervalTranslator::run(&profile);
    serde_json::to_writer(std::io::stdout(), &events).unwrap();
}



// ==========================
// === IntervalTranslator ===
// ==========================

/// Translates `profiler` data to the Chrome DevTools format.
struct IntervalTranslator<'p, Metadata> {
    profile: &'p data::Profile<Metadata>,
    events:  Vec<devtools::Event>,
}

impl<'p, Metadata> IntervalTranslator<'p, Metadata> {
    /// Translate `profiler` data to the Chrome DevTools format.
    fn run(profile: &'p data::Profile<Metadata>) -> Vec<devtools::Event> {
        let events = Default::default();
        let mut builder = Self { profile, events };
        // We skip the root node APP_LIFETIME, which is not a real measurement.
        for child in &profile.root_interval().children {
            builder.visit_interval(*child, 0);
        }
        let Self { events, .. } = builder;
        events
    }
}

impl<'p, Metadata> IntervalTranslator<'p, Metadata> {
    /// Translate an interval, and its children.
    fn visit_interval(&mut self, active: data::IntervalId, row: u32) {
        let active = &self.profile[active];
        let measurement = &self.profile[active.measurement];
        let start = active.interval.start.into_ms();
        // TODO[kw]: The format supports incomplete events, but isn't documented.
        const DEFAULT_END: f64 = 30_000.0;
        let end = active.interval.end.map(|x| x.into_ms()).unwrap_or(DEFAULT_END);
        let event = devtools::Event {
            name:         measurement.label.to_string(),
            category:     "interval".to_owned(),
            event_type:   devtools::EventType::Complete,
            timestamp_us: (start * 1000.0) as u64,
            duration_us:  ((end - start) * 1000.0) as u64,
            process_id:   1,
            thread_id:    1,
            args:         None,
        };
        self.events.push(event);
        for child in &active.children {
            self.visit_interval(*child, row + 1);
        }
    }
}
