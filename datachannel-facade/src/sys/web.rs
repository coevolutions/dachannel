pub type Configuration = web_datachannel::Configuration;

pub struct PeerConnection {
    inner: web_datachannel::PeerConnection,
}

impl From<web_datachannel::Error> for crate::Error {
    fn from(value: web_datachannel::Error) -> Self {
        Self(value.into())
    }
}

impl From<crate::SdpType> for web_datachannel::SdpType {
    fn from(value: crate::SdpType) -> Self {
        match value {
            crate::SdpType::Offer => Self::Offer,
            crate::SdpType::Answer => Self::Answer,
            crate::SdpType::Pranswer => Self::Pranswer,
            crate::SdpType::Rollback => Self::Rollback,
        }
    }
}

impl From<web_datachannel::SdpType> for crate::SdpType {
    fn from(value: web_datachannel::SdpType) -> Self {
        match value {
            web_datachannel::SdpType::Offer => Self::Offer,
            web_datachannel::SdpType::Answer => Self::Answer,
            web_datachannel::SdpType::Pranswer => Self::Pranswer,
            web_datachannel::SdpType::Rollback => Self::Rollback,
            _ => unreachable!(),
        }
    }
}

impl From<crate::Description> for web_datachannel::Description {
    fn from(value: crate::Description) -> Self {
        Self {
            type_: value.type_.into(),
            sdp: value.sdp,
        }
    }
}

impl From<web_datachannel::Description> for crate::Description {
    fn from(value: web_datachannel::Description) -> Self {
        Self {
            type_: value.type_.into(),
            sdp: value.sdp,
        }
    }
}

impl From<web_datachannel::IceGatheringState> for crate::IceGatheringState {
    fn from(value: web_datachannel::IceGatheringState) -> Self {
        match value {
            web_datachannel::IceGatheringState::New => Self::New,
            web_datachannel::IceGatheringState::Gathering => Self::Gathering,
            web_datachannel::IceGatheringState::Complete => Self::Complete,
            _ => unreachable!(),
        }
    }
}

impl From<web_datachannel::PeerConnectionState> for crate::PeerConnectionState {
    fn from(value: web_datachannel::PeerConnectionState) -> Self {
        match value {
            web_datachannel::PeerConnectionState::New => Self::New,
            web_datachannel::PeerConnectionState::Connecting => Self::Connecting,
            web_datachannel::PeerConnectionState::Connected => Self::Connected,
            web_datachannel::PeerConnectionState::Disconnected => Self::Disconnected,
            web_datachannel::PeerConnectionState::Failed => Self::Failed,
            web_datachannel::PeerConnectionState::Closed => Self::Closed,
            _ => unreachable!(),
        }
    }
}

impl PeerConnection {
    pub fn new(config: crate::Configuration) -> Result<Self, crate::Error> {
        Ok(Self {
            inner: web_datachannel::PeerConnection::new(web_datachannel::Configuration {
                ice_transport_policy: match config.ice_transport_policy {
                    crate::IceTransportPolicy::All => web_datachannel::IceTransportPolicy::All,
                    crate::IceTransportPolicy::Relay => web_datachannel::IceTransportPolicy::Relay,
                },
                ..config.sys
            })?,
        })
    }

    pub fn close(&self) -> Result<(), crate::Error> {
        self.inner.close();
        Ok(())
    }

    pub async fn set_local_description(&self, type_: crate::SdpType) -> Result<(), crate::Error> {
        let description = match type_ {
            crate::SdpType::Offer => self.inner.create_offer().await?,
            crate::SdpType::Answer => self.inner.create_answer().await?,
            crate::SdpType::Pranswer => web_datachannel::Description {
                type_: web_datachannel::SdpType::Pranswer,
                sdp: "".to_string(),
            },
            crate::SdpType::Rollback => web_datachannel::Description {
                type_: web_datachannel::SdpType::Rollback,
                sdp: "".to_string(),
            },
        };
        self.inner
            .set_local_description(&description.into())
            .await?;
        Ok(())
    }

    pub async fn set_remote_description(
        &self,
        description: &crate::Description,
    ) -> Result<(), crate::Error> {
        self.inner
            .set_remote_description(&description.clone().into())
            .await?;
        Ok(())
    }

    pub fn local_description(&self) -> Result<Option<crate::Description>, crate::Error> {
        Ok(self.inner.local_description().map(|v| v.into()))
    }

    pub fn remote_description(&self) -> Result<Option<crate::Description>, crate::Error> {
        Ok(self.inner.remote_description().map(|v| v.into()))
    }

    pub async fn add_ice_candidate(&self, cand: Option<&str>) -> Result<(), crate::Error> {
        self.inner
            .add_ice_candidate(
                cand.map(|cand| cand.to_string())
                    .as_ref()
                    .map(|v| v.as_str()),
            )
            .await?;
        Ok(())
    }

    pub fn set_on_ice_candidate(&mut self, cb: Option<impl Fn(Option<&str>) + 'static>) {
        self.inner.set_on_ice_candidate(cb);
    }

    pub fn set_on_ice_gathering_state_change(
        &mut self,
        cb: Option<impl Fn(crate::IceGatheringState) + 'static>,
    ) {
        self.inner.set_on_ice_gathering_state_change(
            cb.map(|cb| move |state: web_datachannel::IceGatheringState| cb(state.into())),
        );
    }

    pub fn set_on_connection_state_change(
        &mut self,
        cb: Option<impl Fn(crate::PeerConnectionState) + 'static>,
    ) {
        self.inner.set_on_connection_state_change(
            cb.map(|cb| move |state: web_datachannel::PeerConnectionState| cb(state.into())),
        );
    }

    pub fn set_on_data_channel(&mut self, cb: Option<impl Fn(DataChannel) + 'static>) {
        self.inner.set_on_data_channel(
            cb.map(|cb| move |dc: web_datachannel::DataChannel| cb(DataChannel { inner: dc })),
        );
    }

    pub fn create_data_channel(
        &self,
        label: &str,
        options: crate::DataChannelOptions,
    ) -> Result<DataChannel, crate::Error> {
        Ok(DataChannel {
            inner: self.inner.create_data_channel(
                label,
                web_datachannel::DataChannelOptions {
                    ordered: options.ordered,
                    max_packet_life_time: options.max_packet_life_time,
                    max_retransmits: options.max_retransmits,
                    protocol: options.protocol,
                    negotiated: options.negotiated,
                    id: options.id,
                },
            )?,
        })
    }
}

pub struct DataChannel {
    inner: web_datachannel::DataChannel,
}

impl DataChannel {
    pub fn set_on_open(&mut self, cb: Option<impl Fn() + 'static>) {
        self.inner.set_on_open(cb);
    }

    pub fn set_on_close(&mut self, cb: Option<impl Fn() + 'static>) {
        self.inner.set_on_close(cb);
    }

    pub fn set_on_buffered_amount_low(&mut self, cb: Option<impl Fn() + 'static>) {
        self.inner.set_on_buffered_amount_low(cb);
    }

    pub fn set_on_error(&mut self, cb: Option<impl Fn(crate::Error) + 'static>) {
        self.inner
            .set_on_error(cb.map(|cb| move |err: web_datachannel::Error| cb(err.into())));
    }

    pub fn set_on_message(&mut self, cb: Option<impl Fn(&[u8]) + 'static>) {
        self.inner.set_on_message(cb);
    }

    pub fn set_buffered_amount_low_threshold(&self, value: u32) -> Result<(), crate::Error> {
        self.inner.set_buffered_amount_low_threshold(value);
        Ok(())
    }

    pub fn buffered_amount(&self) -> Result<u32, crate::Error> {
        Ok(self.inner.buffered_amount())
    }

    pub fn close(&self) -> Result<(), crate::Error> {
        self.inner.close();
        Ok(())
    }

    pub fn send(&self, buf: &[u8]) -> Result<(), crate::Error> {
        self.inner.send(buf)?;
        Ok(())
    }
}
