pub struct PermanentNotify {
    notified: std::sync::atomic::AtomicBool,
    event: event_listener::Event,
}

impl PermanentNotify {
    pub fn new() -> Self {
        PermanentNotify {
            notified: false.into(),
            event: event_listener::Event::new(),
        }
    }

    pub async fn notified(&self) {
        while !self.notified.load(std::sync::atomic::Ordering::SeqCst) {
            self.event.listen().await;
        }
    }

    pub fn notify(&self) {
        self.notified
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.event.notify(usize::MAX);
    }
}
