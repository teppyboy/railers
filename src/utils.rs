use rdev::{EventType, Key};
use std::thread;
use std::time::Duration;
use tracing::error;

pub fn press_key(key: Key) {
    match rdev::simulate(&EventType::KeyPress(key)) {
        Ok(()) => (),
        Err(_) => {
            error!("We could not send down {:?}", Key::Space);
        }
    }
    thread::sleep(Duration::from_millis(10));
    match rdev::simulate(&EventType::KeyRelease(key)) {
        Ok(()) => (),
        Err(_) => {
            error!("We could not send release {:?}", Key::Space);
        }
    }
}
