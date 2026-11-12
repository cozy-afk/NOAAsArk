//! Strategy wrapper: currently pass-through on SignalEngine output.
//! Multi-model blending and timing filters land here in v0.2.

use noaas_ark_core::types::Signal;

pub fn filter_signal(signal: Option<Signal>) -> Option<Signal> {
    // v0.1: no extra filters beyond the engine's own gates.
    signal
}
