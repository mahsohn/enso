//! Supports constructing a profile file.

use derivative::Derivative;



// ======================
// === ProfileBuilder ===
// ======================

#[derive(Derivative)]
#[derivative(Default(bound=""))]
pub struct ProfileBuilder<LabelStorage> {
    events: Vec<crate::Event<AnyMetadata, LabelStorage>>,
    headers: Vec<crate::Header>,
}

impl<LabelStorage> ProfileBuilder<LabelStorage> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn time_offset(&mut self, offset: crate::TimeOffset) {
        self.headers.push(crate::Header::TimeOffset(offset));
    }

    pub fn metadata<M>(&mut self, timestamp: crate::Timestamp, name: &'static str, data: M)
    where M: serde::Serialize {
        let data = serde_json::value::to_raw_value(&data).unwrap();
        let data = Variant { name, data };
        let data = crate::Timestamped { timestamp, data };
        let data = crate::Event::Metadata(data);
        self.events.push(data);
    }

    /// TODO[kw]: Eliminate this. Provide a function for each event type, in order to create a
    ///  boundary decoupling the profile format definition from the internal log structures.
    pub fn raw_nonmetadata_event<M>(&mut self, event: crate::Event<M, LabelStorage>) {
        let event = event.map_metadata(|_| panic!());
        self.events.push(event);
    }
}

impl<LabelStorage: serde::Serialize> ProfileBuilder<LabelStorage> {
    pub fn to_string(self) -> String {
        let mut profile: Vec<_> = self.headers.into_iter().map(crate::internal::Event::Header).collect();
        profile.extend(self.events);
        serde_json::to_string(&profile).unwrap()
    }
}




// ===================
// === AnyMetadata ===
// ===================

type AnyMetadata = Variant<Box<serde_json::value::RawValue>>;


// === Variant ===

/// Wrapper for serializing an object as if it were a particular variant of some unspecified enum.
///
/// This allows serializing instances of one variant of an enum without knowledge of the other
/// variants.
struct Variant<T> {
    name: &'static str,
    data: T,
}

impl<T: serde::Serialize> serde::Serialize for Variant<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: serde::Serializer {
        serializer.serialize_newtype_variant("", 0, self.name, &self.data)
    }
}
