use crate::frame::Frame;
use crate::net::bus::ResonanceBus;
use crate::net::message::Message;
use crate::net::node::Node;
use crate::trit::{TritValue, TritWord};

impl ResonanceBus {
    /// Run a negotiation among a set of participant nodes.
    ///
    /// Single-pass: collects phases, detects cross-frame, and computes consensus
    /// in one traversal instead of three.
    ///
    /// Returns the consensus TritWord and whether a conflict was detected.
    pub fn negotiate(&mut self, participant_ids: &[String]) -> (TritWord, bool) {
        let mut participants: Vec<&Node> = Vec::with_capacity(participant_ids.len());
        let mut phase_sum = 0.0;
        let mut first_frame: Option<&Frame> = None;
        let mut has_cross_frame = false;

        for id in participant_ids {
            if let Some(node) = self.nodes.get(id) {
                phase_sum += node.current_phase;
                if let Some(ff) = first_frame {
                    if &node.frame != ff {
                        has_cross_frame = true;
                    }
                } else {
                    first_frame = Some(&node.frame);
                }
                participants.push(node);
            }
        }

        if participants.is_empty() {
            return (TritWord::hold(Frame::Meta), false);
        }

        let consensus_phase = phase_sum / participants.len() as f64;
        let conflict_resolution = if has_cross_frame {
            "hold"
        } else {
            "commit_true"
        };

        // Build message
        let frames: Vec<String> = participants
            .iter()
            .map(|n| format!("{}", n.frame))
            .collect();
        let phases: Vec<f64> = participants.iter().map(|n| n.current_phase).collect();
        let msg = Message::negotiate(
            "resonance-bus",
            participant_ids.to_vec(),
            frames,
            phases,
            conflict_resolution,
        );
        self.push_log(msg);

        let result = if has_cross_frame {
            TritWord::hold(Frame::Meta)
        } else {
            TritWord::new(TritValue::True, consensus_phase, Frame::Meta)
        };

        (result, has_cross_frame)
    }
}

#[cfg(test)]
mod tests {
    use crate::frame::Frame;
    use crate::net::bus::ResonanceBus;
    use crate::net::node::Node;
    use crate::trit::TritValue;

    fn make_node(id: &str, frame: Frame, phase: f64) -> Node {
        Node::new(id.to_string(), frame, phase)
    }

    #[test]
    fn three_node_negotiation_cross_frame_hold() {
        let mut bus = ResonanceBus::new();
        bus.register(make_node("a", Frame::Science, 0.75));
        bus.register(make_node("b", Frame::Individual, 0.35));
        bus.register(make_node("c", Frame::Consensus, 0.6));

        let (result, has_conflict) =
            bus.negotiate(&["a".to_string(), "b".to_string(), "c".to_string()]);

        assert!(has_conflict);
        assert_eq!(result.value, TritValue::Hold);
    }

    #[test]
    fn three_node_negotiation_same_frame_commits() {
        let mut bus = ResonanceBus::new();
        bus.register(make_node("a", Frame::Science, 0.7));
        bus.register(make_node("b", Frame::Science, 0.8));
        bus.register(make_node("c", Frame::Science, 0.6));

        let (result, has_conflict) =
            bus.negotiate(&["a".to_string(), "b".to_string(), "c".to_string()]);

        assert!(!has_conflict);
        assert_eq!(result.value, TritValue::True);
        assert!((result.phase.inner() - 0.7).abs() < 0.01);
    }
}
