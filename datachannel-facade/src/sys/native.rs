pub struct PeerConnection {
    inner: libdatachannel::PeerConnection,
}

impl From<libdatachannel::Error> for crate::Error {
    fn from(value: libdatachannel::Error) -> Self {
        Self(value.into())
    }
}

impl From<crate::SdpType> for libdatachannel::SdpType {
    fn from(value: crate::SdpType) -> Self {
        match value {
            crate::SdpType::Offer => Self::Offer,
            crate::SdpType::Answer => Self::Answer,
            crate::SdpType::Pranswer => Self::Pranswer,
            crate::SdpType::Rollback => Self::Rollback,
        }
    }
}

impl From<libdatachannel::SdpType> for crate::SdpType {
    fn from(value: libdatachannel::SdpType) -> Self {
        match value {
            libdatachannel::SdpType::Offer => Self::Offer,
            libdatachannel::SdpType::Answer => Self::Answer,
            libdatachannel::SdpType::Pranswer => Self::Pranswer,
            libdatachannel::SdpType::Rollback => Self::Rollback,
        }
    }
}

impl From<crate::Description> for libdatachannel::Description {
    fn from(value: crate::Description) -> Self {
        Self {
            type_: value.type_.into(),
            sdp: value.sdp,
        }
    }
}

impl From<libdatachannel::Description> for crate::Description {
    fn from(value: libdatachannel::Description) -> Self {
        Self {
            type_: value.type_.into(),
            sdp: value.sdp,
        }
    }
}

impl From<libdatachannel::GatheringState> for crate::IceGatheringState {
    fn from(value: libdatachannel::GatheringState) -> Self {
        match value {
            libdatachannel::GatheringState::New => Self::New,
            libdatachannel::GatheringState::InProgress => Self::Gathering,
            libdatachannel::GatheringState::Complete => Self::Complete,
        }
    }
}

impl From<libdatachannel::State> for crate::PeerConnectionState {
    fn from(value: libdatachannel::State) -> Self {
        match value {
            libdatachannel::State::Connecting => Self::Connecting,
            libdatachannel::State::Connected => Self::Connected,
            libdatachannel::State::Disconnected => Self::Disconnected,
            libdatachannel::State::Failed => Self::Failed,
            libdatachannel::State::Closed => Self::Closed,
        }
    }
}

impl From<libdatachannel::IceCandidate> for crate::IceCandidate {
    fn from(value: libdatachannel::IceCandidate) -> Self {
        Self {
            candidate: value.candidate,
            sdp_mid: Some(value.sdp_mid),
            sdp_m_line_index: None,
        }
    }
}

impl From<crate::IceCandidate> for libdatachannel::IceCandidate {
    fn from(value: crate::IceCandidate) -> Self {
        Self {
            candidate: value.candidate,
            sdp_mid: value.sdp_mid.unwrap_or_else(|| "".to_string()),
        }
    }
}

impl PeerConnection {
    pub fn new(config: crate::Configuration) -> Result<Self, crate::Error> {
        Ok(Self {
            inner: libdatachannel::PeerConnection::new(libdatachannel::Configuration {
                ice_transport_policy: match config.ice_transport_policy {
                    crate::IceTransportPolicy::All => libdatachannel::TransportPolicy::All,
                    crate::IceTransportPolicy::Relay => libdatachannel::TransportPolicy::Relay,
                },
                ..Default::default()
            })?,
        })
    }

    pub fn close(&self) -> Result<(), crate::Error> {
        self.inner.close()?;
        Ok(())
    }

    pub async fn set_local_description(&self, type_: crate::SdpType) -> Result<(), crate::Error> {
        if type_ == crate::SdpType::Answer
            && self
                .local_description()?
                .map(|d| d.type_ == crate::SdpType::Answer)
                .unwrap_or(false)
        {
            return Ok(());
        }
        self.inner.set_local_description(Some(type_.into()))?;
        Ok(())
    }

    pub async fn set_remote_description(
        &self,
        description: &crate::Description,
    ) -> Result<(), crate::Error> {
        self.inner
            .set_remote_description(&description.clone().into())?;
        Ok(())
    }

    pub fn local_description(&self) -> Result<Option<crate::Description>, crate::Error> {
        match self.inner.local_description() {
            Ok(v) => Ok(Some(v.into())),
            Err(libdatachannel::Error::NotAvail) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn remote_description(&self) -> Result<Option<crate::Description>, crate::Error> {
        match self.inner.remote_description() {
            Ok(v) => Ok(Some(v.into())),
            Err(libdatachannel::Error::NotAvail) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn add_ice_candidate(
        &self,
        cand: Option<&crate::IceCandidate>,
    ) -> Result<(), crate::Error> {
        self.inner
            .add_remote_candidate(&cand.map(|cand| cand.clone().into()).unwrap_or_else(|| {
                libdatachannel::IceCandidate {
                    candidate: "".to_string(),
                    sdp_mid: "".to_string(),
                }
            }))?;
        Ok(())
    }

    pub fn set_on_ice_candidate(
        &mut self,
        cb: Option<impl Fn(Option<crate::IceCandidate>) + 'static>,
    ) {
        self.inner.set_on_local_candidate(cb.map(|cb| {
            move |cand: libdatachannel::IceCandidate| {
                cb(if !cand.candidate.is_empty() {
                    Some(cand.into())
                } else {
                    None
                })
            }
        }))
    }

    pub fn set_on_ice_gathering_state_change(
        &mut self,
        cb: Option<impl Fn(crate::IceGatheringState) + 'static>,
    ) {
        self.inner.set_on_gathering_state_change(
            cb.map(|cb| move |state: libdatachannel::GatheringState| cb(state.into())),
        )
    }

    pub fn set_on_connection_state_change(
        &mut self,
        cb: Option<impl Fn(crate::PeerConnectionState) + 'static>,
    ) {
        self.inner
            .set_on_state_change(cb.map(|cb| move |state: libdatachannel::State| cb(state.into())))
    }

    pub fn set_on_data_channel(&mut self, cb: Option<impl Fn(DataChannel) + 'static>) {
        self.inner
            .set_on_data_channel(cb.map(|cb| move |dc| cb(DataChannel { inner: dc })))
    }

    pub fn create_data_channel(
        &self,
        label: &str,
        options: crate::DataChannelOptions,
    ) -> Result<DataChannel, crate::Error> {
        Ok(DataChannel {
            inner: self.inner.create_data_channel(
                label,
                libdatachannel::DataChannelOptions {
                    reliability: libdatachannel::Reliability {
                        unordered: !options.ordered,
                        unreliable: options.max_packet_life_time.is_some()
                            || options.max_retransmits.is_some(),
                        max_packet_life_time: options.max_packet_life_time.unwrap_or(0) as u32,
                        max_retransmits: options.max_retransmits.unwrap_or(0) as u32,
                    },
                    protocol: options.protocol,
                    negotiated: options.negotiated,
                    stream: options.id,
                },
            )?,
        })
    }
}

pub struct DataChannel {
    inner: libdatachannel::DataChannel,
}

impl DataChannel {
    pub fn set_on_open(&mut self, cb: Option<impl Fn() + 'static>) {
        self.inner.set_on_open(cb);
    }

    pub fn set_on_close(&mut self, cb: Option<impl Fn() + 'static>) {
        self.inner.set_on_closed(cb);
    }

    pub fn set_on_buffered_amount_low(&mut self, cb: Option<impl Fn() + 'static>) {
        self.inner.set_on_buffered_amount_low(cb);
    }

    pub fn set_on_error(&mut self, cb: Option<impl Fn(&str) + 'static>) {
        self.inner.set_on_error(cb);
    }

    pub fn set_on_message(&mut self, cb: Option<impl Fn(&[u8]) + 'static>) {
        self.inner.set_on_message(cb);
    }

    pub fn set_buffered_amount_low_threshold(&self, value: u32) -> Result<(), crate::Error> {
        self.inner
            .set_buffered_amount_low_threshold(value as usize)?;
        Ok(())
    }

    pub fn send(&self, buf: &[u8]) -> Result<(), crate::Error> {
        self.inner.send(buf)?;
        Ok(())
    }
}
