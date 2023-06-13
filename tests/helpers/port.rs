use std::sync::atomic::{AtomicU16, Ordering};

static PORT_NUMBER: AtomicU16 = AtomicU16::new(check_mate_common::DEFAULT_PORT);

pub fn get_port_number() -> u16 {
    // This is needed, because every integration test is run in a separate thread simultaneously, so we have to ensure
    // each of them uses a different port number. Otherwise we'll have socket bind errors.
    PORT_NUMBER.fetch_add(1, Ordering::Relaxed)
}
