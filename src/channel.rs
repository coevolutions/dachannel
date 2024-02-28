pub struct ReadHalf {
    rx: async_channel::Receiver<Vec<u8>>,
}

impl ReadHalf {
    pub async fn recv(&self) -> Result<Option<Vec<u8>>, async_channel::RecvError> {
        Ok(Some(self.rx.recv().await?))
    }
}

pub struct WriteHalf {
    dc: datachannel_facade::DataChannel,
}

impl WriteHalf {
    pub fn send(&self, buf: &[u8]) -> Result<(), datachannel_facade::Error> {
        self.dc.send(buf)?;
        Ok(())
    }
}
