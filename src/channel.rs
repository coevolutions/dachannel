/// The receiver half of a channel.
pub struct Receiver {
    rx: async_channel::Receiver<Vec<u8>>,
}

impl Receiver {
    /// Receive a datagram from the channel, or [`None`] if the channel is closed.
    pub async fn recv(&self) -> Option<Vec<u8>> {
        self.rx.recv().await.ok()
    }

    /// Rejoin the Receiver with its Sender.
    pub fn unsplit(self, sender: Sender) -> Channel {
        sender.unsplit(self)
    }
}

/// The sender half of a channel.
pub struct Sender {
    is_open_rx: async_lock::Mutex<Option<oneshot::Receiver<()>>>,
    dc: datachannel_facade::DataChannel,
}

impl Sender {
    /// Send a datagram to the channel.
    pub async fn send(&self, buf: &[u8]) -> Result<(), crate::Error> {
        if let Some(is_open_rx) = self.is_open_rx.lock().await.take() {
            let _ = is_open_rx.await;
        }
        self.dc.send(buf)?;
        Ok(())
    }

    /// Rejoin the Sender with its Receiver.
    pub fn unsplit(self, receiver: Receiver) -> Channel {
        Channel {
            receiver,
            sender: self,
        }
    }
}

/// A Channel is a WebRTC DataChannel that datagrams can be sent and received on.
pub struct Channel {
    receiver: Receiver,
    sender: Sender,
}

impl Channel {
    pub(crate) fn wrap(mut dc: datachannel_facade::DataChannel) -> Channel {
        let (is_open_tx, is_open_rx) = oneshot::channel();
        let (tx, rx) = async_channel::unbounded();

        dc.set_on_open(Some({
            let is_open_tx = std::cell::RefCell::new(Some(is_open_tx));
            move || {
                if let Some(is_open_tx) = is_open_tx.take() {
                    let _ = is_open_tx.send(());
                }
            }
        }));
        dc.set_on_message(Some({
            let tx = tx.clone();
            move |buf: &[u8]| {
                let _ = tx.try_send(buf.to_vec());
            }
        }));
        dc.set_on_error(Some({
            let tx = tx.clone();
            move |_: datachannel_facade::Error| {
                // Do something useful with the error.
                tx.close();
            }
        }));
        dc.set_on_close(Some({
            let tx = tx.clone();
            move || {
                tx.close();
            }
        }));

        Channel {
            receiver: Receiver { rx },
            sender: Sender {
                dc,
                is_open_rx: async_lock::Mutex::new(Some(is_open_rx)),
            },
        }
    }

    /// Receive a datagram from the channel, or [`None`] if the channel is closed.
    pub async fn recv(&self) -> Option<Vec<u8>> {
        self.receiver.recv().await
    }

    /// Send a datagram to the channel.
    pub async fn send(&self, buf: &[u8]) -> Result<(), crate::Error> {
        self.sender.send(buf).await
    }

    /// Split the channel into [`Sender`] and [`Receiver`] halves.
    pub fn split(self) -> (Sender, Receiver) {
        (self.sender, self.receiver)
    }
}