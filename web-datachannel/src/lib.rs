use wasm_bindgen::JsCast as _;

pub struct PeerConnection {
    pc: web_sys::RtcPeerConnection,
}

#[derive(thiserror::Error, Debug)]
pub struct Error(js_sys::Error);

impl From<wasm_bindgen::JsValue> for Error {
    fn from(value: wasm_bindgen::JsValue) -> Self {
        Self(value.into())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: String = self.0.to_string().into();
        write!(f, "{s}")
    }
}

pub type SdpType = web_sys::RtcSdpType;
pub type IceGatheringState = web_sys::RtcIceGatheringState;
pub type PeerConnectionState = web_sys::RtcPeerConnectionState;
pub type IceTransportPolicy = web_sys::RtcIceTransportPolicy;

#[derive(Debug)]
pub struct Description {
    pub type_: SdpType,
    pub sdp: String,
}

#[derive(Debug, serde::Serialize)]
pub struct IceServer {
    pub urls: Vec<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
}

#[derive(Debug)]
pub struct Configuration {
    pub ice_servers: Vec<IceServer>,
    pub ice_transport_policy: IceTransportPolicy,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            ice_servers: Default::default(),
            ice_transport_policy: IceTransportPolicy::All,
        }
    }
}

impl PeerConnection {
    pub fn new(configuration: Configuration) -> Result<Self, Error> {
        let mut raw = web_sys::RtcConfiguration::new();
        raw.ice_servers(&serde_wasm_bindgen::to_value(&configuration.ice_servers).unwrap());
        raw.ice_transport_policy(configuration.ice_transport_policy);
        Ok(Self {
            pc: web_sys::RtcPeerConnection::new_with_configuration(&raw)?,
        })
    }

    pub fn close(&self) {
        self.pc.close()
    }

    pub async fn create_offer(&self) -> Result<Description, Error> {
        let raw = web_sys::RtcSessionDescription::from(
            wasm_bindgen_futures::JsFuture::from(self.pc.create_offer()).await?,
        );

        Ok(Description {
            type_: raw.type_(),
            sdp: raw.sdp(),
        })
    }

    pub async fn create_answer(&self) -> Result<Description, Error> {
        let raw = web_sys::RtcSessionDescription::from(
            wasm_bindgen_futures::JsFuture::from(self.pc.create_answer()).await?,
        );

        Ok(Description {
            type_: raw.type_(),
            sdp: raw.sdp(),
        })
    }

    pub async fn set_local_description(&self, description: &Description) -> Result<(), Error> {
        let mut raw = web_sys::RtcSessionDescriptionInit::new(description.type_);
        raw.sdp(&description.sdp);
        wasm_bindgen_futures::JsFuture::from(self.pc.set_local_description(&raw)).await?;
        Ok(())
    }

    pub async fn set_remote_description(&self, description: &Description) -> Result<(), Error> {
        let mut raw = web_sys::RtcSessionDescriptionInit::new(description.type_);
        raw.sdp(&description.sdp);
        wasm_bindgen_futures::JsFuture::from(self.pc.set_remote_description(&raw)).await?;
        Ok(())
    }

    pub fn create_data_channel(
        &self,
        label: &str,
        options: DataChannelOptions,
    ) -> Result<DataChannel, Error> {
        let mut raw = web_sys::RtcDataChannelInit::new();
        raw.ordered(options.ordered);
        if let Some(v) = options.max_packet_life_time {
            raw.max_packet_life_time(v);
        }
        if let Some(v) = options.max_retransmits {
            raw.max_retransmits(v);
        }
        raw.protocol(&options.protocol);
        raw.negotiated(options.negotiated);
        if let Some(v) = options.id {
            raw.id(v);
        }
        Ok(DataChannel {
            dc: self
                .pc
                .create_data_channel_with_data_channel_dict(label, &raw),
        })
    }

    pub fn set_on_ice_candidate(&self, cb: Option<impl Fn(Option<&str>) + 'static>) {
        let cb = cb.map(|cb| {
            wasm_bindgen::closure::Closure::<dyn FnMut(_)>::new(
                move |ev: web_sys::RtcPeerConnectionIceEvent| {
                    cb(ev
                        .candidate()
                        .map(|cand| cand.candidate())
                        .as_ref()
                        .map(|v| v.as_str()));
                },
            )
        });
        self.pc
            .set_onicecandidate(cb.as_ref().map(|cb| cb.as_ref().unchecked_ref()));
        if let Some(cb) = cb {
            cb.forget();
        }
    }

    pub fn set_on_ice_gathering_state_change(
        &self,
        cb: Option<impl Fn(IceGatheringState) + 'static>,
    ) {
        let pc = self.pc.clone();
        let cb = cb.map(|cb| {
            wasm_bindgen::closure::Closure::<dyn FnMut(_)>::new(move |_ev: web_sys::Event| {
                cb(pc.ice_gathering_state());
            })
        });
        self.pc
            .set_onicegatheringstatechange(cb.as_ref().map(|cb| cb.as_ref().unchecked_ref()));
        if let Some(cb) = cb {
            cb.forget();
        }
    }

    pub fn set_on_connection_state_change(
        &self,
        cb: Option<impl Fn(PeerConnectionState) + 'static>,
    ) {
        let pc = self.pc.clone();
        let cb = cb.map(|cb| {
            wasm_bindgen::closure::Closure::<dyn FnMut(_)>::new(move |_ev: web_sys::Event| {
                cb(pc.connection_state());
            })
        });
        self.pc
            .set_onconnectionstatechange(cb.as_ref().map(|cb| cb.as_ref().unchecked_ref()));
        if let Some(cb) = cb {
            cb.forget();
        }
    }

    pub fn set_on_data_channel(&self, cb: Option<impl Fn(DataChannel) + 'static>) {
        let cb = cb.map(|cb| {
            wasm_bindgen::closure::Closure::<dyn FnMut(_)>::new(
                move |ev: web_sys::RtcDataChannelEvent| {
                    cb(DataChannel { dc: ev.channel() });
                },
            )
        });
        self.pc
            .set_ondatachannel(cb.as_ref().map(|cb| cb.as_ref().unchecked_ref()));
        if let Some(cb) = cb {
            cb.forget();
        }
    }

    pub fn local_description(&self) -> Option<Description> {
        self.pc.local_description().map(|v| Description {
            type_: v.type_(),
            sdp: v.sdp(),
        })
    }

    pub fn remote_description(&self) -> Option<Description> {
        self.pc.remote_description().map(|v| Description {
            type_: v.type_(),
            sdp: v.sdp(),
        })
    }

    pub async fn add_ice_candidate(&self, cand: Option<&str>) -> Result<(), crate::Error> {
        wasm_bindgen_futures::JsFuture::from(
            self.pc.add_ice_candidate_with_opt_rtc_ice_candidate(
                cand.map(|cand| {
                    let raw = web_sys::RtcIceCandidateInit::new(cand);
                    web_sys::RtcIceCandidate::new(&raw).unwrap()
                })
                .as_ref(),
            ),
        )
        .await?;
        Ok(())
    }
}

impl Drop for PeerConnection {
    fn drop(&mut self) {
        self.pc.close();
    }
}

pub struct DataChannel {
    dc: web_sys::RtcDataChannel,
}

impl DataChannel {
    pub fn set_on_open(&self, cb: Option<impl Fn() + 'static>) {
        let cb = cb.map(|cb| {
            wasm_bindgen::closure::Closure::<dyn FnMut(_)>::new(
                move |_: web_sys::RtcDataChannelEvent| {
                    cb();
                },
            )
        });
        self.dc
            .set_onopen(cb.as_ref().map(|cb| cb.as_ref().unchecked_ref()));
        if let Some(cb) = cb {
            cb.forget();
        }
    }

    pub fn set_on_close(&self, cb: Option<impl Fn() + 'static>) {
        let cb = cb.map(|cb| {
            wasm_bindgen::closure::Closure::<dyn FnMut(_)>::new(
                move |_: web_sys::RtcDataChannelEvent| {
                    cb();
                },
            )
        });
        self.dc
            .set_onclose(cb.as_ref().map(|cb| cb.as_ref().unchecked_ref()));
        if let Some(cb) = cb {
            cb.forget();
        }
    }

    pub fn set_on_buffered_amount_low(&self, cb: Option<impl Fn() + 'static>) {
        let cb = cb.map(|cb| {
            wasm_bindgen::closure::Closure::<dyn FnMut(_)>::new(move |_: web_sys::Event| {
                cb();
            })
        });
        self.dc
            .set_onbufferedamountlow(cb.as_ref().map(|cb| cb.as_ref().unchecked_ref()));
        if let Some(cb) = cb {
            cb.forget();
        }
    }

    pub fn set_on_error(&self, cb: Option<impl Fn(Error) + 'static>) {
        let cb = cb.map(|cb| {
            wasm_bindgen::closure::Closure::<dyn FnMut(_)>::new(move |ev: web_sys::ErrorEvent| {
                cb(ev.error().into());
            })
        });
        self.dc
            .set_onerror(cb.as_ref().map(|cb| cb.as_ref().unchecked_ref()));
        if let Some(cb) = cb {
            cb.forget();
        }
    }

    pub fn set_on_message(&self, cb: Option<impl Fn(&[u8]) + 'static>) {
        let cb = cb.map(|cb| {
            wasm_bindgen::closure::Closure::<dyn FnMut(_)>::new(move |ev: web_sys::MessageEvent| {
                let arr = match ev.data().dyn_into::<js_sys::ArrayBuffer>() {
                    Ok(arr) => arr,
                    Err(e) => {
                        log::error!("unsupported message: {:?}", e);
                        return;
                    }
                };
                cb(js_sys::Uint8Array::new(&arr).to_vec().as_slice());
            })
        });
        self.dc
            .set_onmessage(cb.as_ref().map(|cb| cb.as_ref().unchecked_ref()));
        if let Some(cb) = cb {
            cb.forget();
        }
    }

    pub fn set_buffered_amount_low_threshold(&self, value: u32) {
        self.dc.set_buffered_amount_low_threshold(value);
    }

    pub fn buffered_amount(&self) -> u32 {
        self.dc.buffered_amount()
    }

    pub fn close(&self) {
        self.dc.close();
    }

    pub fn send(&self, buf: &[u8]) -> Result<(), Error> {
        self.dc.send_with_u8_array(buf)?;
        Ok(())
    }
}

impl Drop for DataChannel {
    fn drop(&mut self) {
        self.dc.close();
    }
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

#[cfg(test)]
mod test {
    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    pub async fn test_peer_connection_new() {
        let pc = PeerConnection::new(Default::default()).unwrap();
        pc.create_data_channel("test", Default::default()).unwrap();
    }

    #[wasm_bindgen_test]
    pub async fn test_peer_connection_communicate() {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));

        let pc1 = PeerConnection::new(Default::default()).unwrap();
        let pc1_gathered = std::sync::Arc::new(async_notify::Notify::new());
        pc1.set_on_ice_gathering_state_change(Some({
            let pc1_gathered = std::sync::Arc::clone(&pc1_gathered);
            move |ice_gathering_state| {
                if ice_gathering_state == IceGatheringState::Complete {
                    pc1_gathered.notify();
                }
            }
        }));

        let dc1 = pc1
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
        pc1.set_local_description(&pc1.create_offer().await.unwrap())
            .await
            .unwrap();
        pc1_gathered.notified().await;

        let pc2 = PeerConnection::new(Default::default()).unwrap();
        let pc2_gathered = std::sync::Arc::new(async_notify::Notify::new());
        pc2.set_on_ice_gathering_state_change(Some({
            let pc2_gathered = std::sync::Arc::clone(&pc2_gathered);
            move |ice_gathering_state| {
                if ice_gathering_state == IceGatheringState::Complete {
                    pc2_gathered.notify();
                }
            }
        }));

        let dc2 = pc2
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

        pc2.set_remote_description(&pc1.local_description().unwrap())
            .await
            .unwrap();

        pc2.set_local_description(&pc2.create_answer().await.unwrap())
            .await
            .unwrap();
        pc2_gathered.notified().await;

        pc1.set_remote_description(&pc2.local_description().unwrap())
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
