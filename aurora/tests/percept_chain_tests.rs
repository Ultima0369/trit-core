use aurora::percept::{ExternalPercept, PerceptBatch, PerceptChain, PerceptError};

/// Mock provider that always succeeds.
struct MockOkProvider {
    name: &'static str,
    prio: u8,
}

impl ExternalPercept for MockOkProvider {
    fn perceive(&self, _raw: &str) -> Result<PerceptBatch, PerceptError> {
        Ok(PerceptBatch::empty(self.name))
    }
    fn provider_name(&self) -> &str {
        self.name
    }
    fn priority(&self) -> u8 {
        self.prio
    }
    fn available(&self) -> bool {
        true
    }
}

/// Mock provider that always fails.
struct MockFailProvider {
    name: &'static str,
    prio: u8,
}

impl ExternalPercept for MockFailProvider {
    fn perceive(&self, _raw: &str) -> Result<PerceptBatch, PerceptError> {
        Err(PerceptError::ParseError("mock failure".into()))
    }
    fn provider_name(&self) -> &str {
        self.name
    }
    fn priority(&self) -> u8 {
        self.prio
    }
    fn available(&self) -> bool {
        true
    }
}

#[test]
fn chain_uses_first_provider_when_it_succeeds() {
    let chain = PerceptChain::new()
        .with(Box::new(MockOkProvider {
            name: "first",
            prio: 0,
        }))
        .with(Box::new(MockOkProvider {
            name: "second",
            prio: 1,
        }));

    let result = chain.perceive_or_degrade("test").unwrap();
    assert_eq!(result.source, "first");
}

#[test]
fn chain_degrades_to_second_when_first_fails() {
    let chain = PerceptChain::new()
        .with(Box::new(MockFailProvider {
            name: "bad",
            prio: 0,
        }))
        .with(Box::new(MockOkProvider {
            name: "good",
            prio: 1,
        }));

    let result = chain.perceive_or_degrade("test").unwrap();
    assert_eq!(result.source, "good");
}

#[test]
fn chain_returns_error_when_all_fail() {
    let chain = PerceptChain::new()
        .with(Box::new(MockFailProvider {
            name: "bad1",
            prio: 0,
        }))
        .with(Box::new(MockFailProvider {
            name: "bad2",
            prio: 1,
        }));

    let err = chain.perceive_or_degrade("test").unwrap_err();
    assert!(matches!(err, PerceptError::AllUnavailable));
}

#[test]
fn chain_sorts_providers_by_priority() {
    let chain = PerceptChain::new()
        .with(Box::new(MockOkProvider {
            name: "low",
            prio: 2,
        }))
        .with(Box::new(MockOkProvider {
            name: "high",
            prio: 0,
        }))
        .with(Box::new(MockOkProvider {
            name: "mid",
            prio: 1,
        }));

    let result = chain.perceive_or_degrade("test").unwrap();
    assert_eq!(result.source, "high");
}

#[test]
fn chain_skips_unavailable_providers() {
    struct UnavailableProvider;
    impl ExternalPercept for UnavailableProvider {
        fn perceive(&self, _raw: &str) -> Result<PerceptBatch, PerceptError> {
            panic!("should not be called");
        }
        fn provider_name(&self) -> &str {
            "offline"
        }
        fn priority(&self) -> u8 {
            0
        }
        fn available(&self) -> bool {
            false
        }
    }

    let chain = PerceptChain::new()
        .with(Box::new(UnavailableProvider))
        .with(Box::new(MockOkProvider {
            name: "fallback",
            prio: 1,
        }));

    let result = chain.perceive_or_degrade("test").unwrap();
    assert_eq!(result.source, "fallback");
}

#[test]
fn empty_chain_returns_all_unavailable() {
    let chain = PerceptChain::new();
    let err = chain.perceive_or_degrade("test").unwrap_err();
    assert!(matches!(err, PerceptError::AllUnavailable));
}
