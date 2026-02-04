#![no_main]

use libfuzzer_sys::fuzz_target;
use mochi_parser::{Action, Parser};

fuzz_target!(|data: &[u8]| {
    let mut parser = Parser::new();
    let mut actions = Vec::new();

    // Feed the data to the parser - it should never panic
    parser.parse(data, |action| {
        actions.push(action);
    });

    // Verify invariants:
    // 1. Parser should always be in a valid state
    // 2. Actions should be well-formed
    for action in &actions {
        match action {
            Action::Print(c) => {
                // Print actions should have valid characters
                assert!(c.is_ascii() || c.len_utf8() <= 4);
            }
            Action::CsiDispatch { params, .. } => {
                // CSI params should be bounded
                assert!(params.len() <= 32);
            }
            Action::OscDispatch { params } => {
                // OSC params should be bounded
                assert!(params.len() <= 1024 * 1024); // 1MB max
            }
            _ => {}
        }
    }
});
