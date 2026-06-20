use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Decision domain / reference frame for ternary computation.
/// Each frame defines a context of validity. Cross-frame operations
/// trigger MetaInterrupt instead of forced collapse.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Frame {
    /// Empirical / evidence-based.
    Science,
    /// User-specific context / personal fact.
    Individual,
    /// Statistical / group preference.
    Consensus,
    /// Unknowable / unobservable (always Hold).
    Absolute,
    /// Conflict resolution / policy output.
    ///
    /// Meta is a system-internal frame: it is the output frame when
    /// cross-frame operations (TAND/TOR) detect a conflict. External
    /// signal inputs should not use Meta.
    Meta,
    /// Direct first-person experience.
    ///
    /// Prioritized over `Science` in first-person-aware arbitration:
    /// the subject's lived experience is treated as a distinct reference
    /// frame that should not be overridden by population statistics.
    FirstPerson,
    /// Body-state-driven judgment (e.g. heart rate, galvanic skin response).
    Embodied,
    /// Relationship-state-driven judgment (e.g. trust, reciprocity, role).
    Relational,
    /// Geographic and ecological context (e.g. location, climate, biome).
    GeoEco,
    /// Developmental trajectory (e.g. life stage, skill level, growth path).
    Developmental,
    /// Social or professional role context.
    Role,
    /// Environmental state (e.g. noise, light, weather, social density).
    Environmental,
}

impl std::fmt::Display for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Frame::Science => write!(f, "Science"),
            Frame::Individual => write!(f, "Individual"),
            Frame::Consensus => write!(f, "Consensus"),
            Frame::Absolute => write!(f, "Absolute"),
            Frame::Meta => write!(f, "Meta"),
            Frame::FirstPerson => write!(f, "FirstPerson"),
            Frame::Embodied => write!(f, "Embodied"),
            Frame::Relational => write!(f, "Relational"),
            Frame::GeoEco => write!(f, "GeoEco"),
            Frame::Developmental => write!(f, "Developmental"),
            Frame::Role => write!(f, "Role"),
            Frame::Environmental => write!(f, "Environmental"),
        }
    }
}

impl FromStr for Frame {
    type Err = FrameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Science" => Ok(Frame::Science),
            "Individual" => Ok(Frame::Individual),
            "Consensus" => Ok(Frame::Consensus),
            "Absolute" => Ok(Frame::Absolute),
            "Meta" => Ok(Frame::Meta),
            "FirstPerson" => Ok(Frame::FirstPerson),
            "Embodied" => Ok(Frame::Embodied),
            "Relational" => Ok(Frame::Relational),
            "GeoEco" => Ok(Frame::GeoEco),
            "Developmental" => Ok(Frame::Developmental),
            "Role" => Ok(Frame::Role),
            "Environmental" => Ok(Frame::Environmental),
            unknown => Err(FrameError::Unknown(unknown.to_string())),
        }
    }
}

/// Error returned when a string cannot be parsed as a [`Frame`].
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum FrameError {
    #[error("Unknown frame: '{0}'")]
    Unknown(String),
}

/// Registry of active frames in a computation session.
#[derive(Debug, Clone, Default)]
pub struct FrameRegistry {
    frames: Vec<Frame>,
}

impl FrameRegistry {
    pub fn new() -> Self {
        Self { frames: vec![] }
    }

    pub fn register(&mut self, frame: Frame) {
        if !self.frames.contains(&frame) {
            self.frames.push(frame);
        }
    }

    pub fn is_registered(&self, frame: &Frame) -> bool {
        self.frames.contains(frame)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Frame> {
        self.frames.iter()
    }

    /// Register all frames found in a slice of TritWords.
    pub fn register_from_words(&mut self, words: &[crate::core::word::TritWord]) {
        for word in words {
            self.register(word.frame());
        }
    }

    /// Validate that all frames in the given words are registered.
    /// Returns the first unregistered frame found, or Ok(()) if all are registered.
    pub fn validate_all(&self, words: &[crate::core::word::TritWord]) -> Result<(), Frame> {
        for word in words {
            if !self.is_registered(&word.frame()) {
                return Err(word.frame());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_science_from_str() {
        assert_eq!(Frame::from_str("Science").unwrap(), Frame::Science);
    }

    #[test]
    fn should_parse_all_known_frames() {
        for (input, expected) in [
            ("Science", Frame::Science),
            ("Individual", Frame::Individual),
            ("Consensus", Frame::Consensus),
            ("Absolute", Frame::Absolute),
            ("Meta", Frame::Meta),
            ("FirstPerson", Frame::FirstPerson),
            ("Embodied", Frame::Embodied),
            ("Relational", Frame::Relational),
        ] {
            assert_eq!(Frame::from_str(input).unwrap(), expected);
        }
    }

    #[test]
    fn should_reject_unknown_frame() {
        assert!(Frame::from_str("Unknown").is_err());
        assert!(Frame::from_str("").is_err());
    }

    #[test]
    fn should_display_frame_as_string() {
        assert_eq!(format!("{}", Frame::Science), "Science");
        assert_eq!(format!("{}", Frame::Individual), "Individual");
    }

    #[test]
    fn registry_should_track_unique_frames() {
        let mut reg = FrameRegistry::new();
        reg.register(Frame::Science);
        reg.register(Frame::Science); // duplicate
        reg.register(Frame::Individual);
        assert!(reg.is_registered(&Frame::Science));
        assert!(reg.is_registered(&Frame::Individual));
        assert!(!reg.is_registered(&Frame::Consensus));
    }

    #[test]
    fn registry_should_start_empty() {
        let reg = FrameRegistry::new();
        assert!(!reg.is_registered(&Frame::Science));
    }

    #[test]
    fn registry_iter_yields_registered_frames() {
        let mut reg = FrameRegistry::new();
        reg.register(Frame::Science);
        reg.register(Frame::Individual);
        let collected: Vec<_> = reg.iter().cloned().collect();
        assert_eq!(collected.len(), 2);
        assert!(collected.contains(&Frame::Science));
        assert!(collected.contains(&Frame::Individual));
    }

    #[test]
    fn default_registry_is_empty() {
        let reg: FrameRegistry = Default::default();
        assert!(!reg.is_registered(&Frame::Meta));
    }

    #[test]
    fn parsing_is_case_sensitive() {
        assert!(Frame::from_str("science").is_err());
        assert!(Frame::from_str("SCIENCE").is_err());
    }

    #[test]
    fn frame_error_display_is_informative() {
        let err = Frame::from_str("Bogus").unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("Bogus"));
        assert!(msg.contains("Unknown frame"));
    }

    #[test]
    fn display_covers_all_variants() {
        assert_eq!(format!("{}", Frame::Science), "Science");
        assert_eq!(format!("{}", Frame::Individual), "Individual");
        assert_eq!(format!("{}", Frame::Consensus), "Consensus");
        assert_eq!(format!("{}", Frame::Absolute), "Absolute");
        assert_eq!(format!("{}", Frame::Meta), "Meta");
        assert_eq!(format!("{}", Frame::FirstPerson), "FirstPerson");
        assert_eq!(format!("{}", Frame::Embodied), "Embodied");
        assert_eq!(format!("{}", Frame::Relational), "Relational");
    }
}
