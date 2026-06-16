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
