pub struct PeerConnection {
    impl_: web_datachannel::PeerConnection,
}

impl From<web_datachannel::Error> for crate::Error {
    fn from(value: web_datachannel::Error) -> Self {
        Self(value.into())
    }
}

impl From<web_datachannel::IceCandidate> for crate::IceCandidate {
    fn from(value: web_datachannel::IceCandidate) -> Self {
        Self {
            candidate: value.candidate,
            sdp_mid: value.sdp_mid,
            sdp_m_line_index: value.sdp_m_line_index,
        }
    }
}

impl From<crate::IceCandidate> for web_datachannel::IceCandidate {
    fn from(value: crate::IceCandidate) -> Self {
        Self {
            candidate: value.candidate,
            sdp_mid: value.sdp_mid,
            sdp_m_line_index: value.sdp_m_line_index,
        }
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
            impl_: web_datachannel::PeerConnection::new(web_datachannel::Configuration {
                ice_transport_policy: match config.ice_transport_policy {
                    crate::IceTransportPolicy::All => web_datachannel::IceTransportPolicy::All,
                    crate::IceTransportPolicy::Relay => web_datachannel::IceTransportPolicy::Relay,
                },
                ..Default::default()
            })?,
        })
    }

    pub fn close(&self) -> Result<(), crate::Error> {
        self.impl_.close();
        Ok(())
    }

    pub async fn set_local_description(&self, type_: crate::SdpType) -> Result<(), crate::Error> {
        let description = match type_ {
            crate::SdpType::Offer => self.impl_.create_offer().await?,
            crate::SdpType::Answer => self.impl_.create_answer().await?,
            crate::SdpType::Pranswer => web_datachannel::Description {
                type_: web_datachannel::SdpType::Pranswer,
                sdp: "".to_string(),
            },
            crate::SdpType::Rollback => web_datachannel::Description {
                type_: web_datachannel::SdpType::Rollback,
                sdp: "".to_string(),
            },
        };
        self.impl_
            .set_local_description(&description.into())
            .await?;
        Ok(())
    }

    pub async fn set_remote_description(
        &self,
        description: &crate::Description,
    ) -> Result<(), crate::Error> {
        self.impl_
            .set_remote_description(&description.clone().into())
            .await?;
        Ok(())
    }

    pub fn local_description(&self) -> Result<Option<crate::Description>, crate::Error> {
        Ok(self.impl_.local_description().map(|v| v.into()))
    }

    pub fn remote_description(&self) -> Result<Option<crate::Description>, crate::Error> {
        Ok(self.impl_.remote_description().map(|v| v.into()))
    }

    pub async fn add_ice_candidate(
        &self,
        cand: Option<&crate::IceCandidate>,
    ) -> Result<(), crate::Error> {
        self.impl_
            .add_ice_candidate(cand.map(|cand| cand.clone().into()).as_ref())
            .await?;
        Ok(())
    }

    pub fn set_on_ice_candidate(
        &mut self,
        cb: Option<impl Fn(Option<crate::IceCandidate>) + 'static>,
    ) {
        self.impl_.set_on_ice_candidate(cb.map(|cb| {
            move |cand: Option<web_datachannel::IceCandidate>| cb(cand.map(|cand| cand.into()))
        }));
    }

    pub fn set_on_ice_gathering_state_change(
        &mut self,
        cb: Option<impl Fn(crate::IceGatheringState) + 'static>,
    ) {
        self.impl_.set_on_ice_gathering_state_change(
            cb.map(|cb| move |state: web_datachannel::IceGatheringState| cb(state.into())),
        );
    }

    pub fn set_on_connection_state_change(
        &mut self,
        cb: Option<impl Fn(crate::PeerConnectionState) + 'static>,
    ) {
        self.impl_.set_on_connection_state_change(
            cb.map(|cb| move |state: web_datachannel::PeerConnectionState| cb(state.into())),
        );
    }

    pub fn set_on_data_channel(&mut self, cb: Option<impl Fn(crate::DataChannel) + 'static>) {
        self.impl_.set_on_data_channel(cb.map(|cb| {
            move |dc: web_datachannel::DataChannel| cb(crate::DataChannel { impl_: dc })
        }));
    }

    pub fn create_data_channel(
        &self,
        label: &str,
        options: crate::DataChannelOptions,
    ) -> Result<DataChannel, crate::Error> {
        Ok(DataChannel {
            impl_: self.impl_.create_data_channel(
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
    impl_: web_datachannel::DataChannel,
}

impl DataChannel {
    pub fn set_on_open(&mut self, cb: Option<impl Fn() + 'static>) {
        self.impl_.set_on_open(cb);
    }

    pub fn set_on_close(&mut self, cb: Option<impl Fn() + 'static>) {
        self.impl_.set_on_close(cb);
    }

    pub fn set_on_buffered_amount_low(&mut self, cb: Option<impl Fn() + 'static>) {
        self.impl_.set_on_buffered_amount_low(cb);
    }

    pub fn set_on_error(&mut self, cb: Option<impl Fn(&str) + 'static>) {
        self.impl_.set_on_error(
            cb.map(|cb| move |err: web_datachannel::Error| cb(&String::from(err.to_string()))),
        );
    }

    pub fn set_on_message(&mut self, cb: Option<impl Fn(&[u8]) + 'static>) {
        self.impl_.set_on_message(cb);
    }

    pub fn set_buffered_amount_low_threshold(&self, value: u32) -> Result<(), crate::Error> {
        self.impl_.set_buffered_amount_low_threshold(value);
        Ok(())
    }

    pub fn send(&self, buf: &[u8]) -> Result<(), crate::Error> {
        self.impl_.send(buf)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    pub async fn test_peer_connection_new() {
        crate::test::test_peer_connection_new().await;
    }

    #[wasm_bindgen_test]
    pub async fn test_peer_connection_communicate() {
        crate::test::test_peer_connection_communicate().await;
    }
}