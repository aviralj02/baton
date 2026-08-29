//! Problems the user needs to hear about.
//!
//! Baton reports failures it can survive by printing to stderr, which a packaged
//! app does not have. A user whose hotkey never registered because another app
//! owns the combination sees a launcher that does nothing, with no way to find
//! out why and nothing to tell us.
//!
//! So a notice goes three places: stderr for a developer, an event for a window
//! that is already open, and a queue for the windows that are not. Startup runs
//! before any webview exists, which is exactly when the interesting failures
//! happen, so the event alone would drop them.

use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager};

pub const EVENT: &str = "baton://notice";

#[derive(Default)]
pub struct Queue(pub Mutex<Vec<String>>);

/// Record something the user should know. Never fails: a notice that cannot be
/// delivered must not take down the operation that produced it.
pub fn report(app: &AppHandle, message: impl Into<String>) {
    let message = message.into();
    eprintln!("[baton] {message}");

    if let Some(queue) = app.try_state::<Queue>() {
        if let Ok(mut pending) = queue.0.lock() {
            // Bounded: an unattended app that fails on a timer must not grow a
            // queue nobody will ever read.
            if pending.len() < 20 {
                pending.push(message.clone());
            }
        }
    }
    let _ = app.emit(EVENT, message);
}

/// Drain the queue. The caller has shown them, so nothing else should.
pub fn take(app: &AppHandle) -> Vec<String> {
    app.try_state::<Queue>()
        .and_then(|queue| queue.0.lock().ok().map(|mut q| std::mem::take(&mut *q)))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_queue_is_bounded() {
        let queue = Queue::default();
        {
            let mut pending = queue.0.lock().unwrap();
            for i in 0..50 {
                if pending.len() < 20 {
                    pending.push(format!("{i}"));
                }
            }
        }
        assert_eq!(queue.0.lock().unwrap().len(), 20);
    }

    #[test]
    fn taking_leaves_the_queue_empty() {
        let queue = Queue::default();
        queue.0.lock().unwrap().push("one".into());
        let drained = std::mem::take(&mut *queue.0.lock().unwrap());
        assert_eq!(drained, vec!["one".to_string()]);
        assert!(queue.0.lock().unwrap().is_empty());
    }
}
