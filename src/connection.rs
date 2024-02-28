pub use datachannel_facade::Configuration;
pub use datachannel_facade::DataChannelOptions;
pub use datachannel_facade::Description;
pub use datachannel_facade::Error;
pub use datachannel_facade::IceGatheringState;
pub use datachannel_facade::PeerConnectionState;
pub use datachannel_facade::SdpType;

pub struct Connection {
    pc: datachannel_facade::PeerConnection,
    ice_candidates_rx: async_channel::Receiver<Option<String>>,
    ice_gathering_states_rx: async_channel::Receiver<IceGatheringState>,
    peer_connection_states_rx: async_channel::Receiver<PeerConnectionState>,
    data_channels_rx: async_channel::Receiver<datachannel_facade::DataChannel>,
}

impl Connection {
    fn wrap(mut pc: datachannel_facade::PeerConnection) -> Self {
        let (ice_candidates_tx, ice_candidates_rx) = async_channel::unbounded();
        let (ice_gathering_states_tx, ice_gathering_states_rx) = async_channel::unbounded();
        let (peer_connection_states_tx, peer_connection_states_rx) = async_channel::unbounded();
        let (data_channels_tx, data_channels_rx) = async_channel::unbounded();

        pc.set_on_ice_candidate(Some(move |cand: Option<&str>| {
            let _ = ice_candidates_tx.try_send(cand.map(|v| v.to_string()));
        }));
        pc.set_on_ice_gathering_state_change(Some(move |state: IceGatheringState| {
            let _ = ice_gathering_states_tx.try_send(state);
        }));
        pc.set_on_connection_state_change(Some(move |state: PeerConnectionState| {
            let _ = peer_connection_states_tx.try_send(state);
        }));
        pc.set_on_data_channel(Some(move |dc: datachannel_facade::DataChannel| {
            let _ = data_channels_tx.try_send(dc);
        }));

        Self {
            pc,
            ice_candidates_rx,
            ice_gathering_states_rx,
            peer_connection_states_rx,
            data_channels_rx,
        }
    }

    pub fn new(config: Configuration) -> Result<Self, Error> {
        Ok(Self::wrap(datachannel_facade::PeerConnection::new(config)?))
    }

    pub async fn next_ice_candidate(&self) -> Option<Option<String>> {
        self.ice_candidates_rx.recv().await.ok()
    }

    pub async fn next_ice_gathering_state(&self) -> Option<IceGatheringState> {
        self.ice_gathering_states_rx.recv().await.ok()
    }

    pub async fn next_connection_state(&self) -> Option<PeerConnectionState> {
        self.peer_connection_states_rx.recv().await.ok()
    }

    pub async fn next_channel(&self) -> Option<crate::Channel> {
        Some(super::Channel::wrap(
            self.data_channels_rx.recv().await.ok()?,
        ))
    }

    pub fn close(&self) -> Result<(), Error> {
        self.pc.close()
    }

    pub fn create_data_channel(
        &self,
        label: &str,
        options: DataChannelOptions,
    ) -> Result<crate::Channel, Error> {
        Ok(crate::Channel::wrap(
            self.pc.create_data_channel(label, options)?,
        ))
    }

    pub async fn set_local_description(&self, type_: SdpType) -> Result<(), Error> {
        self.pc.set_local_description(type_).await
    }

    pub async fn set_remote_description(&self, description: &Description) -> Result<(), Error> {
        self.pc.set_remote_description(description).await
    }

    pub fn local_description(&self) -> Result<Option<Description>, Error> {
        self.pc.local_description()
    }

    pub fn remote_description(&self) -> Result<Option<Description>, Error> {
        self.pc.remote_description()
    }

    pub async fn add_ice_candidate(&self, cand: Option<&str>) -> Result<(), Error> {
        self.pc.add_ice_candidate(cand).await
    }
}
