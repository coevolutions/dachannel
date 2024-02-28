//! datachannel-facade is a library that abstracts over platform-specific WebRTC DataChannel implementations.
//!
//! It works both in the browser and natively (via [libdatachannel](https://libdatachannel.org)).
//!
//! The following docs are from [MDN](https://developer.mozilla.org/en-US/docs/Web/API/WebRTC_API).

mod sys;

pub mod platform;

/// The property RTCSessionDescription.type is a read-only string value which describes the description's type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SdpType {
    /// The session description object describes the initial proposal in an offer/answer exchange. The session
    /// negotiation process begins with an offer being sent from the caller to the callee.
    Offer,

    /// The SDP contained in the sdp property is the definitive choice in the exchange. In other words, this session
    /// description describes the agreed-upon configuration, and is being sent to finalize negotiation.
    Answer,

    /// The session description object describes a provisional answer; that is, a response to a previous offer that is
    /// not the final answer. It is usually employed by legacy hardware.
    Pranswer,

    /// This special type with an empty session description is used to roll back to the previous stable state.
    Rollback,
}

/// The RTCSessionDescription interface describes one end of a connection—or potential connection—and how it's
/// configured. Each RTCSessionDescription consists of a description type indicating which part of the offer/answer
/// negotiation process it describes and of the SDP descriptor of the session.
///
/// The process of negotiating a connection between two peers involves exchanging RTCSessionDescription objects back and
/// forth, with each description suggesting one combination of connection configuration options that the sender of the
/// description supports. Once the two peers agree upon a configuration for the connection, negotiation is complete.
#[derive(Clone, Debug)]
pub struct Description {
    /// An enum describing the session description's type.
    pub type_: SdpType,

    /// A string containing the SDP describing the session.
    pub sdp: String,
}

/// An object providing configuration options for the data channel. It can contain the following fields:
pub struct DataChannelOptions {
    /// Indicates whether or not messages sent on the RTCDataChannel are required to arrive at their destination in the
    /// same order in which they were sent (true), or if they're allowed to arrive out-of-order (false). Default: true.
    pub ordered: bool,

    /// The maximum number of milliseconds that attempts to transfer a message may take in unreliable mode. While this
    /// value is a 16-bit unsigned number, each user agent may clamp it to whatever maximum it deems appropriate.
    /// Default: null.
    pub max_packet_life_time: Option<u16>,

    /// The maximum number of times the user agent should attempt to retransmit a message which fails the first time in
    /// unreliable mode. While this value is a 16-bit unsigned number, each user agent may clamp it to whatever maximum
    // it deems appropriate. Default: null.
    pub max_retransmits: Option<u16>,

    /// The name of the sub-protocol being used on the RTCDataChannel, if any; otherwise, the empty string ("").
    /// Default: empty string (""). This string may not be longer than 65,535 bytes.
    pub protocol: String,

    /// By default (false), data channels are negotiated in-band, where one side calls createDataChannel, and the other
    /// side listens to the RTCDataChannelEvent event using the ondatachannel event handler. Alternatively (true), they
    /// can be negotiated out of-band, where both sides call createDataChannel with an agreed-upon ID. Default: false.
    pub negotiated: bool,

    /// A 16-bit numeric ID for the channel; permitted values are 0 to 65534. If you don't include this option, the user
    /// agent will select an ID for you.
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

/// The read-only connectionState property of the RTCPeerConnection interface indicates the current state of the peer
/// connection by returning one of the following string values: new, connecting, connected, disconnected, failed, or
/// closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeerConnectionState {
    /// At least one of the connection's ICE transports (RTCIceTransport or RTCDtlsTransport objects) is in the new
    /// state, and none of them are in one of the following states: connecting, checking, failed, disconnected, or all
    // of the connection's transports are in the closed state.
    New,

    /// One or more of the ICE transports are currently in the process of establishing a connection; that is, their
    /// iceConnectionState is either checking or connected, and no transports are in the failed state.
    Connecting,

    /// Every ICE transport used by the connection is either in use (state connected or completed) or is closed (state
    /// closed); in addition, at least one transport is either connected or completed.
    Connected,

    /// At least one of the ICE transports for the connection is in the disconnected state and none of the other
    /// transports are in the states: failed, connecting, or checking.
    Disconnected,

    /// One or more of the ICE transports on the connection is in the failed state.
    Failed,

    /// The RTCPeerConnection is closed.
    Closed,
}

/// The read-only property RTCPeerConnection.iceGatheringState returns a string that describes the connection's ICE
/// gathering state. This lets you detect, for example, when collection of ICE candidates has finished.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IceGatheringState {
    /// The peer connection was just created and hasn't done any networking yet.
    New,

    /// The ICE agent is in the process of gathering candidates for the connection.
    Gathering,

    /// The ICE agent has finished gathering candidates. If something happens that requires collecting new candidates,
    /// such as a new interface being added or the addition of a new ICE server, the state will revert to gathering to
    /// gather those candidates.
    Complete,
}

/// A string representing the current ICE transport policy. Possible values are:
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IceTransportPolicy {
    /// All ICE candidates will be considered. This is the default value.
    #[default]
    All,

    /// Only ICE candidates whose IP addresses are being relayed, such as those being passed through a TURN server, will
    /// be considered.
    Relay,
}

/// A server which may be used by the ICE agent; these are typically STUN and/or TURN servers.
#[derive(Debug)]
pub struct IceServer {
    /// This required property is either a single string or an array of strings, each specifying a URL which can be used
    /// to connect to the server.
    pub urls: Vec<String>,

    /// If the object represents a TURN server, then this is the username to use during the authentication.
    pub username: Option<String>,

    /// The credential to use when logging into the server. This is only used if the object represents a TURN server.
    pub credential: Option<String>,
}

/// An object providing options to configure the new connection:
#[derive(Debug, Default)]
pub struct Configuration {
    /// An array of objects, each describing one server which may be used by the ICE agent; these are typically STUN
    /// and/or TURN servers. If this isn't specified, the connection attempt will be made with no STUN or TURN server
    /// available, which limits the connection to local peers. Each object may have the following properties:
    pub ice_servers: Vec<IceServer>,

    /// A string representing the current ICE transport policy. Possible values are:
    pub ice_transport_policy: IceTransportPolicy,

    sys: sys::Configuration,
}

/// An underlying platform error.
#[derive(thiserror::Error, Debug)]
#[error("{0}")]
pub struct Error(Box<dyn std::error::Error + Send + Sync + 'static>);

/// The RTCPeerConnection interface represents a WebRTC connection between the local computer and a remote peer. It
/// provides methods to connect to a remote peer, maintain and monitor the connection, and close the connection once
/// it's no longer needed.
pub struct PeerConnection {
    inner: sys::PeerConnection,
}

impl PeerConnection {
    /// Returns a new RTCPeerConnection, representing a connection between the local device and a remote peer.
    pub fn new(config: Configuration) -> Result<Self, Error> {
        Ok(Self {
            inner: sys::PeerConnection::new(config)?,
        })
    }

    /// Closes the current peer connection.
    pub fn close(&self) -> Result<(), Error> {
        self.inner.close()
    }

    /// The RTCPeerConnection method setLocalDescription() changes the local description associated with the connection.
    /// This description specifies the properties of the local end of the connection, including the media format. The
    /// method takes a single parameter—the session description—and it returns a Promise which is fulfilled once the
    /// description has been changed, asynchronously.
    ///
    /// <div class="warning">
    /// In datachannel-facade, this will also perform createOffer/createAnswer behind the scenes. The resulting
    /// description can be retrieved by calling PeerConnection::local_description.
    /// </div>
    pub async fn set_local_description(&self, type_: SdpType) -> Result<(), Error> {
        self.inner.set_local_description(type_).await
    }

    /// The RTCPeerConnection method setRemoteDescription() sets the specified session description as the remote peer's
    /// current offer or answer. The description specifies the properties of the remote end of the connection, including
    /// the media format. The method takes a single parameter—the session description—and it returns a Promise which is
    /// fulfilled once the description has been changed, asynchronously.
    ///
    /// This is typically called after receiving an offer or answer from another peer over the signaling server. Keep in
    /// mind that if setRemoteDescription() is called while a connection is already in place, it means renegotiation is
    /// underway (possibly to adapt to changing network conditions).
    ///
    // Because descriptions will be exchanged until the two peers agree on a configuration, the description submitted by
    // calling setRemoteDescription() does not immediately take effect. Instead, the current connection configuration
    // remains in place until negotiation is complete. Only then does the agreed-upon configuration take effect.
    pub async fn set_remote_description(&self, description: &Description) -> Result<(), Error> {
        self.inner.set_remote_description(description).await
    }

    /// The read-only property RTCPeerConnection.localDescription returns an RTCSessionDescription describing the
    /// session for the local end of the connection. If it has not yet been set, this is null.
    pub fn local_description(&self) -> Result<Option<Description>, Error> {
        self.inner.local_description()
    }

    /// The read-only property RTCPeerConnection.remoteDescription returns a RTCSessionDescription describing the
    /// session (which includes configuration and media information) for the remote end of the connection. If this
    /// hasn't been set yet, this is null.
    ///
    /// The returned value typically reflects a remote description which has been received over the signaling server (as
    /// either an offer or an answer) and then put into effect by your code calling
    /// RTCPeerConnection.setRemoteDescription() in response.
    pub fn remote_description(&self) -> Result<Option<Description>, Error> {
        self.inner.remote_description()
    }

    /// Adds a new remote candidate to the RTCPeerConnection's remote description, which describes the state of the
    /// remote end of the connection.
    pub async fn add_ice_candidate(&self, cand: Option<&str>) -> Result<(), Error> {
        self.inner.add_ice_candidate(cand).await
    }

    /// An icecandidate event is sent to an RTCPeerConnection when:
    ///
    /// - An RTCIceCandidate has been identified and added to the local peer by a call to
    ///   RTCPeerConnection.setLocalDescription(),
    ///
    /// - Every RTCIceCandidate correlated with a particular username fragment and password combination (a generation)
    ///   has been so identified and added, and
    ///
    /// - All ICE gathering on all transports is complete.
    ///
    /// In the first two cases, the event handler should transmit the candidate to the remote peer over the signaling
    /// channel so the remote peer can add it to its set of remote candidates.
    pub fn set_on_ice_candidate(
        &mut self,
        cb: Option<impl Fn(Option<&str>) + Send + Sync + 'static>,
    ) {
        self.inner.set_on_ice_candidate(cb)
    }

    /// The icegatheringstatechange event is sent to the onicegatheringstatechange event handler on an RTCPeerConnection
    /// when the state of the ICE candidate gathering process changes. This signifies that the value of the connection's
    /// iceGatheringState property has changed.
    ///
    /// When ICE first starts to gather connection candidates, the value changes from new to gathering to indicate that
    /// the process of collecting candidate configurations for the connection has begun. When the value changes to
    /// complete, all of the transports that make up the RTCPeerConnection have finished gathering ICE candidates.
    pub fn set_on_ice_gathering_state_change(
        &mut self,
        cb: Option<impl Fn(IceGatheringState) + Send + Sync + 'static>,
    ) {
        self.inner.set_on_ice_gathering_state_change(cb)
    }

    /// The connectionstatechange event is sent to the onconnectionstatechange event handler on an RTCPeerConnection
    /// object after a new track has been added to an RTCRtpReceiver which is part of the connection. The new connection
    /// state can be found in connectionState, and is one of the string values: new, connecting, connected,
    /// disconnected, failed, or closed.
    pub fn set_on_connection_state_change(
        &mut self,
        cb: Option<impl Fn(PeerConnectionState) + Send + Sync + 'static>,
    ) {
        self.inner.set_on_connection_state_change(cb)
    }

    /// A datachannel event is sent to an RTCPeerConnection instance when an RTCDataChannel has been added to the
    /// connection, as a result of the remote peer calling RTCPeerConnection.createDataChannel().
    pub fn set_on_data_channel(
        &mut self,
        cb: Option<impl Fn(DataChannel) + Send + Sync + 'static>,
    ) {
        self.inner
            .set_on_data_channel(cb.map(|cb| move |dc| cb(DataChannel { inner: dc })))
    }

    /// The createDataChannel() method on the RTCPeerConnection interface creates a new channel linked with the remote
    /// peer, over which any kind of data may be transmitted. This can be useful for back-channel content, such as
    /// images, file transfer, text chat, game update packets, and so forth.
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

/// The RTCDataChannel interface represents a network channel which can be used for bidirectional peer-to-peer transfers
/// of arbitrary data. Every data channel is associated with an RTCPeerConnection, and each peer connection can have up
/// to a theoretical maximum of 65,534 data channels (the actual limit may vary from browser to browser).
///
/// To create a data channel and ask a remote peer to join you, call the RTCPeerConnection's createDataChannel() method.
/// The peer being invited to exchange data receives a datachannel event (which has type RTCDataChannelEvent) to let it
/// know the data channel has been added to the connection.
pub struct DataChannel {
    inner: sys::DataChannel,
}

impl DataChannel {
    /// The WebRTC open event is sent to an RTCDataChannel object's onopen event handler when the underlying transport
    /// used to send and receive the data channel's messages is opened or reopened.
    pub fn set_on_open(&mut self, cb: Option<impl Fn() + Send + Sync + 'static>) {
        self.inner.set_on_open(cb)
    }

    /// The close event is sent to the onclose event handler on an RTCDataChannel instance when the data transport for
    /// the data channel has closed. Before any further data can be transferred using RTCDataChannel, a new
    /// 'RTCDataChannel' instance must be created.
    pub fn set_on_close(&mut self, cb: Option<impl Fn() + Send + Sync + 'static>) {
        self.inner.set_on_close(cb)
    }

    /// A bufferedamountlow event is sent to an RTCDataChannel when the number of bytes currently in the outbound data
    /// transfer buffer falls below the threshold specified in bufferedAmountLowThreshold. bufferedamountlow events
    /// aren't sent if bufferedAmountLowThreshold is 0.
    pub fn set_on_buffered_amount_low(&mut self, cb: Option<impl Fn() + Send + Sync + 'static>) {
        self.inner.set_on_buffered_amount_low(cb)
    }

    /// A WebRTC error event is sent to an RTCDataChannel object's onerror event handler when an error occurs on the
    /// data channel.
    pub fn set_on_error(&mut self, cb: Option<impl Fn(Error) + Send + Sync + 'static>) {
        self.inner.set_on_error(cb)
    }

    /// The WebRTC message event is sent to the onmessage event handler on an RTCDataChannel object when a message has
    /// been received from the remote peer.
    pub fn set_on_message(&mut self, cb: Option<impl Fn(&[u8]) + Send + Sync + 'static>) {
        self.inner.set_on_message(cb)
    }

    /// The RTCDataChannel property bufferedAmountLowThreshold is used to specify the number of bytes of buffered
    /// outgoing data that is considered "low." The default value is 0. When the number of buffered outgoing bytes, as
    /// indicated by the bufferedAmount property, falls to or below this value, a bufferedamountlow event is fired. This
    /// event may be used, for example, to implement code which queues more messages to be sent whenever there's room to
    /// buffer them. Listeners may be added with onbufferedamountlow or addEventListener().
    ///
    /// The user agent may implement the process of actually sending data in any way it chooses; this may be done
    /// periodically during the event loop or truly asynchronously. As messages are actually sent, this value is reduced
    /// accordingly.
    pub fn set_buffered_amount_low_threshold(&self, value: u32) -> Result<(), crate::Error> {
        self.inner.set_buffered_amount_low_threshold(value)
    }

    /// The read-only RTCDataChannel property bufferedAmount returns the number of bytes of data currently queued to be
    /// sent over the data channel. The queue may build up as a result of calls to the send() method. This only includes
    /// data buffered by the user agent itself; it doesn't include any framing overhead or buffering done by the
    /// operating system or network hardware.
    ///
    /// The user agent may implement the process of actually sending data in any way it chooses; this may be done
    /// periodically during the event loop or truly asynchronously. As messages are actually sent, this value is reduced
    /// accordingly.
    pub fn buffered_amount(&self) -> Result<u32, crate::Error> {
        self.inner.buffered_amount()
    }

    /// The RTCDataChannel.close() method closes the RTCDataChannel. Either peer is permitted to call this method to
    /// initiate closure of the channel.
    ///
    /// Closure of the data channel is not instantaneous. Most of the process of closing the connection is handled
    /// asynchronously; you can detect when the channel has finished closing by watching for a close event on the data
    /// channel.
    pub fn close(&self) -> Result<(), crate::Error> {
        self.inner.close()
    }

    /// The send() method of the RTCDataChannel interface sends data across the data channel to the remote peer. This
    /// can be done any time except during the initial process of creating the underlying transport channel. Data sent
    /// before connecting is buffered if possible (or an error occurs if it's not possible), and is also buffered if
    /// sent while the connection is closing or closed.
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
