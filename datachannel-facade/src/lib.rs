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
mod test {
    use super::*;

    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
        }
    }

    #[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    pub async fn test_peer_connection_new() {
        let pc = PeerConnection::new(Default::default()).unwrap();
        pc.create_data_channel("test", Default::default()).unwrap();
    }

    #[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    pub async fn test_peer_connection_communicate() {
        let mut pc1 = PeerConnection::new(Default::default()).unwrap();
        let pc1_gathered = std::sync::Arc::new(async_notify::Notify::new());
        pc1.set_on_ice_gathering_state_change(Some({
            let pc1_gathered = std::sync::Arc::clone(&pc1_gathered);
            move |ice_gathering_state| {
                if ice_gathering_state == IceGatheringState::Complete {
                    pc1_gathered.notify();
                }
            }
        }));

        let mut dc1 = pc1
            .create_data_channel(
                "test",
                DataChannelOptions {
                    negotiated: true,
                    id: Some(1),
                    ..Default::default()
                },
            )
            .unwrap();
        let dc1_open = std::sync::Arc::new(async_notify::Notify::new());
        dc1.set_on_open(Some({
            let dc1_open = std::sync::Arc::clone(&dc1_open);
            move || {
                dc1_open.notify();
            }
        }));
        pc1.set_local_description(SdpType::Offer).await.unwrap();
        pc1_gathered.notified().await;

        let mut pc2 = PeerConnection::new(Default::default()).unwrap();
        let pc2_gathered = std::sync::Arc::new(async_notify::Notify::new());
        pc2.set_on_ice_gathering_state_change(Some({
            let pc2_gathered = std::sync::Arc::clone(&pc2_gathered);
            move |ice_gathering_state| {
                if ice_gathering_state == IceGatheringState::Complete {
                    pc2_gathered.notify();
                }
            }
        }));

        let mut dc2 = pc2
            .create_data_channel(
                "test",
                DataChannelOptions {
                    negotiated: true,
                    id: Some(1),
                    ..Default::default()
                },
            )
            .unwrap();
        let dc2_open = std::sync::Arc::new(async_notify::Notify::new());
        dc2.set_on_open(Some({
            let dc2_open = std::sync::Arc::clone(&dc2_open);
            move || {
                dc2_open.notify();
            }
        }));

        let (tx1, rx1) = async_channel::bounded(1);
        dc1.set_on_message(Some(move |msg: &[u8]| {
            tx1.try_send(msg.to_vec()).unwrap();
        }));

        let (tx2, rx2) = async_channel::bounded(1);
        dc2.set_on_message(Some(move |msg: &[u8]| {
            tx2.try_send(msg.to_vec()).unwrap();
        }));

        pc2.set_remote_description(&pc1.local_description().unwrap().unwrap())
            .await
            .unwrap();

        pc2.set_local_description(SdpType::Answer).await.unwrap();
        pc2_gathered.notified().await;

        pc1.set_remote_description(&pc2.local_description().unwrap().unwrap())
            .await
            .unwrap();

        dc1_open.notified().await;
        dc2_open.notified().await;

        dc1.send(b"hello world!").unwrap();
        assert_eq!(rx2.recv().await.unwrap(), b"hello world!");

        dc2.send(b"goodbye world!").unwrap();
        assert_eq!(rx1.recv().await.unwrap(), b"goodbye world!");
    }
}
