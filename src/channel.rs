/// The receiver half of a channel.
pub struct Receiver {
    rx: async_channel::Receiver<Result<Vec<u8>, datachannel_facade::Error>>,
}

impl Receiver {
    /// Receive a datagram from the channel, or [`None`] if the channel is closed.
    pub async fn recv(&self) -> Result<Vec<u8>, std::io::Error> {
        Ok(self
            .rx
            .recv()
            .await
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "receiver closed"))?
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?)
    }

    /// Rejoin the Receiver with its Sender.
    pub fn unsplit(self, sender: Sender) -> Channel {
        sender.unsplit(self)
    }
}

/// The sender half of a channel.
pub struct Sender {
    is_open_notify: std::sync::Arc<crate::sync_util::PermanentNotify>,
    dc: datachannel_facade::DataChannel,
}

impl Sender {
    /// Send a datagram to the channel.
    pub async fn send(&self, buf: &[u8]) -> Result<(), std::io::Error> {
        self.is_open_notify.notified().await;
        self.dc
            .send(buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
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
    pub(crate) fn wrap(mut dc: datachannel_facade::DataChannel, is_open: bool) -> Channel {
        let is_open_notify = std::sync::Arc::new(crate::sync_util::PermanentNotify::new());
        if is_open {
            is_open_notify.notify();
        }

        let (tx, rx) = async_channel::unbounded();

        dc.set_on_open(Some({
            let is_open_notify = std::sync::Arc::clone(&is_open_notify);
            move || {
                is_open_notify.notify();
            }
        }));
        dc.set_on_message(Some({
            let tx = tx.clone();
            move |buf: &[u8]| {
                let _ = tx.try_send(Ok(buf.to_vec()));
            }
        }));
        dc.set_on_error(Some({
            let tx = tx.clone();
            move |err: datachannel_facade::Error| {
                let _ = tx.try_send(Err(err));
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
            sender: Sender { dc, is_open_notify },
        }
    }

    /// Receive a datagram from the channel, or [`None`] if the channel is closed.
    pub async fn recv(&self) -> Result<Vec<u8>, std::io::Error> {
        self.receiver.recv().await
    }

    /// Send a datagram to the channel.
    pub async fn send(&self, buf: &[u8]) -> Result<(), std::io::Error> {
        self.sender.send(buf).await
    }

    /// Split the channel into [`Sender`] and [`Receiver`] halves.
    pub fn split(self) -> (Sender, Receiver) {
        (self.sender, self.receiver)
    }
}
