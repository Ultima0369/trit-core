use crate::core::frame::Frame;
use crate::core::word::TritWord;

/// Frame presence bitmask for O(1) frame lookups during arbitration.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FrameMask(u8);

impl FrameMask {
    const SCIENCE: u8 = 1 << 0;
    const INDIVIDUAL: u8 = 1 << 1;
    const CONSENSUS: u8 = 1 << 2;
    const ABSOLUTE: u8 = 1 << 3;
    const META: u8 = 1 << 4;
    const FIRST_PERSON: u8 = 1 << 5;
    const EMBODIED: u8 = 1 << 6;
    const RELATIONAL: u8 = 1 << 7;

    /// Bitmask with all frame bits set. Update this when adding new Frame variants.
    const ALL_FRAMES: u8 = Self::SCIENCE
        | Self::INDIVIDUAL
        | Self::CONSENSUS
        | Self::ABSOLUTE
        | Self::META
        | Self::FIRST_PERSON
        | Self::EMBODIED
        | Self::RELATIONAL;

    pub(crate) fn from_inputs(inputs: &[TritWord]) -> Self {
        let mut mask = 0u8;
        for t in inputs {
            mask |= match t.frame() {
                Frame::Science => Self::SCIENCE,
                Frame::Individual => Self::INDIVIDUAL,
                Frame::Consensus => Self::CONSENSUS,
                Frame::Absolute => Self::ABSOLUTE,
                Frame::Meta => Self::META,
                Frame::FirstPerson => Self::FIRST_PERSON,
                Frame::Embodied => Self::EMBODIED,
                Frame::Relational => Self::RELATIONAL,
            };
            if mask == Self::ALL_FRAMES {
                break; // all frames seen, early exit
            }
        }
        FrameMask(mask)
    }

    pub(crate) fn has(&self, frame: &Frame) -> bool {
        let bit = match frame {
            Frame::Science => Self::SCIENCE,
            Frame::Individual => Self::INDIVIDUAL,
            Frame::Consensus => Self::CONSENSUS,
            Frame::Absolute => Self::ABSOLUTE,
            Frame::Meta => Self::META,
            Frame::FirstPerson => Self::FIRST_PERSON,
            Frame::Embodied => Self::EMBODIED,
            Frame::Relational => Self::RELATIONAL,
        };
        (self.0 & bit) != 0
    }

    pub(crate) fn count(&self) -> u32 {
        self.0.count_ones()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_mask_has_nothing() {
        let mask = FrameMask::from_inputs(&[]);
        assert!(!mask.has(&Frame::Science));
        assert_eq!(mask.count(), 0);
    }

    #[test]
    fn single_frame_detected() {
        let inputs = [TritWord::tru(Frame::Science)];
        let mask = FrameMask::from_inputs(&inputs);
        assert!(mask.has(&Frame::Science));
        assert!(!mask.has(&Frame::Individual));
        assert_eq!(mask.count(), 1);
    }

    #[test]
    fn multiple_frames_counted() {
        let inputs = [
            TritWord::tru(Frame::Science),
            TritWord::fals(Frame::Individual),
            TritWord::hold(Frame::Consensus),
        ];
        let mask = FrameMask::from_inputs(&inputs);
        assert!(mask.has(&Frame::Science));
        assert!(mask.has(&Frame::Individual));
        assert!(mask.has(&Frame::Consensus));
        assert!(!mask.has(&Frame::Absolute));
        assert_eq!(mask.count(), 3);
    }

    #[test]
    fn duplicates_do_not_increase_count() {
        let inputs = [
            TritWord::tru(Frame::Science),
            TritWord::fals(Frame::Science),
            TritWord::hold(Frame::Science),
        ];
        let mask = FrameMask::from_inputs(&inputs);
        assert_eq!(mask.count(), 1);
    }

    #[test]
    fn all_frames_detected() {
        let inputs = [
            TritWord::tru(Frame::Science),
            TritWord::tru(Frame::Individual),
            TritWord::tru(Frame::Consensus),
            TritWord::absolute(),
            TritWord::hold(Frame::Meta),
            TritWord::tru(Frame::FirstPerson),
            TritWord::tru(Frame::Embodied),
            TritWord::tru(Frame::Relational),
        ];
        let mask = FrameMask::from_inputs(&inputs);
        assert!(mask.has(&Frame::Science));
        assert!(mask.has(&Frame::Individual));
        assert!(mask.has(&Frame::Consensus));
        assert!(mask.has(&Frame::Absolute));
        assert!(mask.has(&Frame::Meta));
        assert!(mask.has(&Frame::FirstPerson));
        assert!(mask.has(&Frame::Embodied));
        assert!(mask.has(&Frame::Relational));
        assert_eq!(mask.count(), 8);
    }
}
