#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("closed")]
    Closed,

    #[error("facade: {0}")]
    Facade(#[from] datachannel_facade::Error),
}

pub struct Receiver {
    rx: async_channel::Receiver<Vec<u8>>,
}

impl Receiver {
    pub async fn recv(&self) -> Result<Vec<u8>, Error> {
        Ok(self.rx.recv().await.map_err(|_| Error::Closed)?)
    }

    pub fn unsplit(self, sender: Sender) -> Channel {
        sender.unsplit(self)
    }
}

pub struct Sender {
    is_open_rx: async_lock::Mutex<Option<oneshot::Receiver<()>>>,
    dc: datachannel_facade::DataChannel,
}

impl Sender {
    pub async fn send(&self, buf: &[u8]) -> Result<(), Error> {
        if let Some(is_open_rx) = self.is_open_rx.lock().await.take() {
            is_open_rx.await.map_err(|_| Error::Closed)?;
        }
        self.dc.send(buf)?;
        Ok(())
    }

    pub fn unsplit(self, receiver: Receiver) -> Channel {
        Channel {
            receiver,
            sender: self,
        }
    }
}

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

    pub async fn recv(&self) -> Result<Vec<u8>, Error> {
        Ok(self.receiver.recv().await?)
    }

    pub async fn send(&self, buf: &[u8]) -> Result<(), Error> {
        self.sender.send(buf).await?;
        Ok(())
    }

    pub fn split(self) -> (Sender, Receiver) {
        (self.sender, self.receiver)
    }
}
