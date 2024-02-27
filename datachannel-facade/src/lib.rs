mod sys;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SdpType {
    Offer,
    Answer,
    Pranswer,
    Rollback,
}

#[derive(Clone, Debug)]
pub struct Description {
    pub type_: SdpType,
    pub sdp: String,
}

#[derive(Clone, Debug)]
pub struct IceCandidate {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_m_line_index: Option<u16>,
}

pub struct DataChannelOptions {
    pub ordered: bool,
    pub max_packet_life_time: Option<u16>,
    pub max_retransmits: Option<u16>,
    pub protocol: String,
    pub negotiated: bool,
    pub id: Option<u16>,
}

impl Default for DataChannelOptions {
    fn default() -> Self {
        Self {
            ordered: true,
            max_packet_life_time: None,
            max_retransmits: None,
            protocol: "".to_string(),
            negotiated: false,
            id: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeerConnectionState {
    New,
    Connecting,
    Connected,
    Disconnected,
    Failed,
    Closed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IceGatheringState {
    New,
    Gathering,
    Complete,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IceTransportPolicy {
    #[default]
    All,
    Relay,
}

#[derive(Debug)]
pub struct IceServer {
    pub urls: Vec<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
}

#[derive(Debug, Default)]
pub struct Configuration {
    pub ice_servers: Vec<IceServer>,
    pub ice_transport_policy: IceTransportPolicy,
}

#[derive(thiserror::Error, Debug)]
#[error("{0}")]
pub struct Error(Box<dyn std::error::Error>);

pub struct PeerConnection {
    inner: sys::PeerConnection,
}

impl PeerConnection {
    pub fn new(config: Configuration) -> Result<Self, Error> {
        Ok(Self {
            inner: sys::PeerConnection::new(config)?,
        })
    }

    pub fn close(&self) -> Result<(), Error> {
        self.inner.close()
    }

    pub async fn set_local_description(&self, type_: SdpType) -> Result<(), Error> {
        self.inner.set_local_description(type_).await
    }

    pub async fn set_remote_description(&self, description: &Description) -> Result<(), Error> {
        self.inner.set_remote_description(description).await
    }

    pub fn local_description(&self) -> Result<Option<Description>, Error> {
        self.inner.local_description()
    }

    pub fn remote_description(&self) -> Result<Option<Description>, Error> {
        self.inner.remote_description()
    }

    pub async fn add_ice_candidate(&self, cand: Option<&IceCandidate>) -> Result<(), Error> {
        self.inner.add_ice_candidate(cand).await
    }

    pub fn set_on_ice_candidate(&mut self, cb: Option<impl Fn(Option<IceCandidate>) + 'static>) {
        self.inner.set_on_ice_candidate(cb)
    }

    pub fn set_on_ice_gathering_state_change(
        &mut self,
        cb: Option<impl Fn(IceGatheringState) + 'static>,
    ) {
        self.inner.set_on_ice_gathering_state_change(cb)
    }

    pub fn set_on_connection_state_change(
        &mut self,
        cb: Option<impl Fn(PeerConnectionState) + 'static>,
    ) {
        self.inner.set_on_connection_state_change(cb)
    }

    pub fn set_on_data_channel(&mut self, cb: Option<impl Fn(DataChannel) + 'static>) {
        self.inner
            .set_on_data_channel(cb.map(|cb| move |dc| cb(DataChannel { inner: dc })))
    }

    pub fn create_data_channel(
        &self,
        label: &str,
        options: DataChannelOptions,
    ) -> Result<DataChannel, Error> {
        Ok(DataChannel {
            inner: self.inner.create_data_channel(label, options)?,
        })
    }
}

pub struct DataChannel {
    inner: sys::DataChannel,
}

impl DataChannel {
    pub fn set_on_open(&mut self, cb: Option<impl Fn() + 'static>) {
        self.inner.set_on_open(cb)
    }

    pub fn set_on_close(&mut self, cb: Option<impl Fn() + 'static>) {
        self.inner.set_on_close(cb)
    }

    pub fn set_on_buffered_amount_low(&mut self, cb: Option<impl Fn() + 'static>) {
        self.inner.set_on_buffered_amount_low(cb)
    }

    pub fn set_on_error(&mut self, cb: Option<impl Fn(&str) + 'static>) {
        self.inner.set_on_error(cb)
    }

    pub fn set_on_message(&mut self, cb: Option<impl Fn(&[u8]) + 'static>) {
        self.inner.set_on_message(cb)
    }

    pub fn set_buffered_amount_low_threshold(&self, value: u32) -> Result<(), crate::Error> {
        self.inner.set_buffered_amount_low_threshold(value)
    }

    pub fn send(&self, buf: &[u8]) -> Result<(), crate::Error> {
        self.inner.send(buf)
    }
}

#[cfg(test)]
mod test;
