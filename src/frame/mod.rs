/// Decision domain / reference frame for ternary computation.
/// Each frame defines a context of validity. Cross-frame operations
/// trigger MetaInterrupt instead of forced collapse.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Frame {
    Science,    // Empirical / evidence-based
    Individual, // User-specific context / personal fact
    Consensus,  // Statistical / group preference
    Absolute,   // Unknowable / unobservable (always Hold)
    Meta,       // Conflict resolution / policy output
}

impl std::fmt::Display for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Frame::Science => write!(f, "Science"),
            Frame::Individual => write!(f, "Individual"),
            Frame::Consensus => write!(f, "Consensus"),
            Frame::Absolute => write!(f, "Absolute"),
            Frame::Meta => write!(f, "Meta"),
        }
    }
}

impl std::str::FromStr for Frame {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Science" => Ok(Frame::Science),
            "Individual" => Ok(Frame::Individual),
            "Consensus" => Ok(Frame::Consensus),
            "Absolute" => Ok(Frame::Absolute),
            "Meta" => Ok(Frame::Meta),
            unknown => Err(format!("Unknown frame: '{}'", unknown)),
        }
    }
}

/// Registry of active frames in a computation session.
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
}

impl Default for FrameRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

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
}
