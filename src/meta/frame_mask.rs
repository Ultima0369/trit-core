use crate::frame::Frame;

/// Frame presence bitmask for O(1) frame lookups during arbitration.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FrameMask(u8);

impl FrameMask {
    const SCIENCE: u8 = 1 << 0;
    const INDIVIDUAL: u8 = 1 << 1;
    const CONSENSUS: u8 = 1 << 2;
    const ABSOLUTE: u8 = 1 << 3;
    const META: u8 = 1 << 4;

    pub(crate) fn from_inputs(inputs: &[crate::trit::TritWord]) -> Self {
        let mut mask = 0u8;
        for t in inputs {
            mask |= match t.frame {
                Frame::Science => Self::SCIENCE,
                Frame::Individual => Self::INDIVIDUAL,
                Frame::Consensus => Self::CONSENSUS,
                Frame::Absolute => Self::ABSOLUTE,
                Frame::Meta => Self::META,
            };
            if mask == 0b11111 {
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
        };
        (self.0 & bit) != 0
    }

    pub(crate) fn count(&self) -> u32 {
        self.0.count_ones()
    }
}
