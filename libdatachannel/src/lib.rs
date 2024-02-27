use std::str::FromStr as _;

use num_traits::FromPrimitive as _;

#[derive(thiserror::Error, num_derive::FromPrimitive, Debug)]
pub enum Error {
    #[error("invalid argument")]
    Invalid = -1,

    #[error("runtime error")]
    Failure = -2,

    #[error("element not available")]
    NotAvail = -3,

    #[error("buffer too small")]
    TooSmall = -4,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum CertificateType {
    #[default]
    Default = libdatachannel_sys::rtcCertificateType_RTC_CERTIFICATE_DEFAULT,
    ECDSA = libdatachannel_sys::rtcCertificateType_RTC_CERTIFICATE_ECDSA,
    RSA = libdatachannel_sys::rtcCertificateType_RTC_CERTIFICATE_RSA,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum TransportPolicy {
    #[default]
    All = libdatachannel_sys::rtcTransportPolicy_RTC_TRANSPORT_POLICY_ALL,
    Relay = libdatachannel_sys::rtcTransportPolicy_RTC_TRANSPORT_POLICY_RELAY,
}

#[derive(Debug)]
pub enum SdpType {
    Offer,
    Answer,
    Pranswer,
    Rollback,
}

#[derive(thiserror::Error, Debug)]
#[error("unknown description type")]
pub struct SdpTypeParseError;

impl std::str::FromStr for SdpType {
    type Err = SdpTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "offer" => SdpType::Offer,
            "answer" => SdpType::Answer,
            "pranswer" => SdpType::Pranswer,
            "rollback" => SdpType::Rollback,
            _ => {
                return Err(SdpTypeParseError);
            }
        })
    }
}

impl SdpType {
    fn as_str(&self) -> &str {
        match self {
            SdpType::Offer => "offer",
            SdpType::Answer => "answer",
            SdpType::Pranswer => "pranswer",
            SdpType::Rollback => "rollback",
        }
    }
}

fn get_string(f: impl Fn(*mut i8, i32) -> i32) -> Result<std::ffi::CString, Error> {
    let n = check_error(f(std::ptr::null_mut(), 0))? as usize;
    let mut buf = vec![0u8; n as usize];
    assert_eq!(
        f(buf.as_mut_ptr() as *mut _, buf.len() as i32),
        buf.len() as i32
    );
    Ok(std::ffi::CString::from_vec_with_nul(buf).unwrap())
}

fn check_error(r: i32) -> Result<i32, Error> {
    if r < 0 {
        return Err(Error::from_i32(r as i32).unwrap_or(Error::Failure));
    }
    Ok(r)
}

#[derive(Debug)]
pub struct Description {
    pub type_: SdpType,
    pub sdp: String,
}

#[derive(Default)]
pub struct Configuration {
    pub ice_servers: Vec<String>,
    pub proxy_server: Option<String>,
    pub bind_address: Option<std::net::IpAddr>,
    pub certificate_type: CertificateType,
    pub ice_transport_policy: TransportPolicy,
    pub enable_ice_tcp: bool,
    pub enable_ice_udp_mux: bool,
    pub disable_auto_negotiation: bool,
    pub force_media_transport: bool,
    pub port_range_begin: u16,
    pub port_range_end: u16,
    pub mtu: i32,
    pub max_message_size: i32,
}

pub struct PeerConnection {
    id: i32,
    userdata: Box<PeerConnectionUserData>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, num_derive::FromPrimitive)]
#[repr(u32)]
pub enum State {
    Connecting = libdatachannel_sys::rtcState_RTC_CONNECTING,
    Connected = libdatachannel_sys::rtcState_RTC_CONNECTED,
    Disconnected = libdatachannel_sys::rtcState_RTC_DISCONNECTED,
    Failed = libdatachannel_sys::rtcState_RTC_FAILED,
    Closed = libdatachannel_sys::rtcState_RTC_CLOSED,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, num_derive::FromPrimitive)]
#[repr(u32)]
pub enum GatheringState {
    New = libdatachannel_sys::rtcGatheringState_RTC_GATHERING_NEW,
    InProgress = libdatachannel_sys::rtcGatheringState_RTC_GATHERING_INPROGRESS,
    Complete = libdatachannel_sys::rtcGatheringState_RTC_GATHERING_COMPLETE,
}

#[derive(Default)]
struct PeerConnectionUserData {
    on_local_description: Option<Box<dyn Fn(&str, SdpType)>>,
    on_local_candidate: Option<Box<dyn Fn(&str)>>,
    on_state_change: Option<Box<dyn Fn(State)>>,
    on_gathering_state_change: Option<Box<dyn Fn(GatheringState)>>,
    on_data_channel: Option<Box<dyn Fn(DataChannel)>>,
}

fn init_logger() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| unsafe {
        extern "C" fn log_cb(level: u32, message: *const std::ffi::c_char) {
            let message = unsafe { std::ffi::CStr::from_ptr(message) }.to_string_lossy();
            log::log!(
                match level {
                    libdatachannel_sys::rtcLogLevel_RTC_LOG_FATAL => {
                        panic!("{}", message);
                    }
                    libdatachannel_sys::rtcLogLevel_RTC_LOG_ERROR => log::Level::Error,
                    libdatachannel_sys::rtcLogLevel_RTC_LOG_WARNING => log::Level::Warn,
                    libdatachannel_sys::rtcLogLevel_RTC_LOG_INFO => log::Level::Info,
                    libdatachannel_sys::rtcLogLevel_RTC_LOG_DEBUG => log::Level::Debug,
                    libdatachannel_sys::rtcLogLevel_RTC_LOG_VERBOSE => log::Level::Trace,
                    _ => log::Level::Info,
                },
                "{}",
                message
            );
        }

        libdatachannel_sys::rtcInitLogger(
            libdatachannel_sys::rtcLogLevel_RTC_LOG_VERBOSE,
            Some(log_cb),
        );
    });
}

impl PeerConnection {
    pub fn new(mut config: Configuration) -> Result<Self, Error> {
        init_logger();

        let num_ice_servers = config.ice_servers.len();

        let raw_ice_servers = config
            .ice_servers
            .into_iter()
            .map(|s| std::ffi::CString::new(s.as_str()).unwrap())
            .collect::<Vec<_>>();

        let raw_ice_servers_arr = raw_ice_servers
            .iter()
            .map(|v| v.as_ptr())
            .chain(std::iter::once(std::ptr::null()))
            .collect::<Vec<_>>();

        let raw_proxy_server = config
            .proxy_server
            .take()
            .map(|s| std::ffi::CString::new(s.as_str()).unwrap());

        let raw_bind_address = config
            .bind_address
            .take()
            .map(|s| std::ffi::CString::new(s.to_string()).unwrap());

        let raw_config = libdatachannel_sys::rtcConfiguration {
            iceServers: if num_ice_servers > 0 {
                raw_ice_servers_arr.as_ptr() as *mut _
            } else {
                std::ptr::null_mut()
            },
            iceServersCount: num_ice_servers as i32,
            proxyServer: raw_proxy_server
                .as_ref()
                .map(|v| v.as_ptr())
                .unwrap_or(std::ptr::null_mut()),
            bindAddress: raw_bind_address
                .as_ref()
                .map(|v| v.as_ptr())
                .unwrap_or(std::ptr::null_mut()),
            certificateType: config.certificate_type as u32,
            iceTransportPolicy: config.ice_transport_policy as u32,
            enableIceTcp: config.enable_ice_tcp,
            enableIceUdpMux: config.enable_ice_udp_mux,
            disableAutoNegotiation: config.disable_auto_negotiation,
            forceMediaTransport: config.force_media_transport,
            portRangeBegin: config.port_range_begin,
            portRangeEnd: config.port_range_end,
            mtu: config.mtu,
            maxMessageSize: config.max_message_size,
        };
        let id = check_error(unsafe {
            libdatachannel_sys::rtcCreatePeerConnection(&raw_config as *const _)
        })?;
        let mut userdata: Box<PeerConnectionUserData> = Default::default();

        unsafe {
            extern "C" fn local_description_callback(
                _id: i32,
                sdp: *const std::ffi::c_char,
                type_: *const std::ffi::c_char,
                userdata: *mut std::ffi::c_void,
            ) {
                let ud = unsafe { &*(userdata as *mut PeerConnectionUserData) };
                if let Some(cb) = &ud.on_local_description {
                    cb(
                        unsafe { std::ffi::CStr::from_ptr(sdp) }.to_str().unwrap(),
                        SdpType::from_str(
                            unsafe { std::ffi::CStr::from_ptr(type_) }.to_str().unwrap(),
                        )
                        .unwrap(),
                    );
                }
            }
            libdatachannel_sys::rtcSetLocalDescriptionCallback(id, Some(local_description_callback))
        };

        unsafe {
            extern "C" fn local_candidate_callback(
                _id: i32,
                cand: *const std::ffi::c_char,
                _mid: *const std::ffi::c_char,
                userdata: *mut std::ffi::c_void,
            ) {
                let ud = unsafe { &*(userdata as *mut PeerConnectionUserData) };
                if let Some(cb) = &ud.on_local_candidate {
                    cb(unsafe { std::ffi::CStr::from_ptr(cand) }.to_str().unwrap());
                }
            }
            libdatachannel_sys::rtcSetLocalCandidateCallback(id, Some(local_candidate_callback))
        };

        unsafe {
            extern "C" fn state_change_callback(
                _id: i32,
                state: libdatachannel_sys::rtcState,
                userdata: *mut std::ffi::c_void,
            ) {
                let ud = unsafe { &*(userdata as *mut PeerConnectionUserData) };
                if let Some(cb) = &ud.on_state_change {
                    cb(State::from_u32(state).unwrap());
                }
            }
            libdatachannel_sys::rtcSetStateChangeCallback(id, Some(state_change_callback))
        };

        unsafe {
            extern "C" fn gathering_state_change_callback(
                _id: i32,
                state: libdatachannel_sys::rtcGatheringState,
                userdata: *mut std::ffi::c_void,
            ) {
                let ud = unsafe { &*(userdata as *mut PeerConnectionUserData) };
                if let Some(cb) = &ud.on_gathering_state_change {
                    cb(GatheringState::from_u32(state).unwrap());
                }
            }
            libdatachannel_sys::rtcSetGatheringStateChangeCallback(
                id,
                Some(gathering_state_change_callback),
            )
        };

        unsafe {
            extern "C" fn data_channel_callback(
                _id: i32,
                id: i32,
                userdata: *mut std::ffi::c_void,
            ) {
                let ud = unsafe { &*(userdata as *mut PeerConnectionUserData) };
                if let Some(cb) = &ud.on_data_channel {
                    cb(DataChannel::from_raw(id))
                }
            }
            libdatachannel_sys::rtcSetDataChannelCallback(id, Some(data_channel_callback))
        };

        unsafe {
            libdatachannel_sys::rtcSetUserPointer(
                id,
                userdata.as_mut() as *mut PeerConnectionUserData as *mut _,
            );
        }

        Ok(Self { id, userdata })
    }

    pub fn close(&self) -> Result<(), Error> {
        check_error(unsafe { libdatachannel_sys::rtcClosePeerConnection(self.id) })?;
        Ok(())
    }

    pub fn set_local_description(&self, type_: Option<SdpType>) -> Result<(), Error> {
        let raw_typ = type_.map(|v| std::ffi::CString::new(v.as_str()).unwrap());
        check_error(unsafe {
            libdatachannel_sys::rtcSetLocalDescription(
                self.id,
                raw_typ
                    .as_ref()
                    .map(|v| v.as_ptr())
                    .unwrap_or(std::ptr::null_mut()),
            )
        })?;
        Ok(())
    }

    pub fn set_remote_description(&self, desc: &Description) -> Result<(), Error> {
        let raw_sdp = std::ffi::CString::new(desc.sdp.as_str()).unwrap();
        let raw_typ = std::ffi::CString::new(desc.type_.as_str()).unwrap();
        check_error(unsafe {
            libdatachannel_sys::rtcSetRemoteDescription(self.id, raw_sdp.as_ptr(), raw_typ.as_ptr())
        })?;
        Ok(())
    }

    pub fn add_remote_candidate(&self, cand: &str) -> Result<(), Error> {
        let raw_cand = std::ffi::CString::new(cand).unwrap();
        check_error(unsafe {
            libdatachannel_sys::rtcAddRemoteCandidate(self.id, raw_cand.as_ptr(), std::ptr::null())
        })?;
        Ok(())
    }

    pub fn local_description(&self) -> Result<Description, Error> {
        Ok(Description {
            type_: SdpType::from_str(
                get_string(|buf, n| unsafe {
                    libdatachannel_sys::rtcGetLocalDescriptionType(self.id, buf, n)
                })?
                .to_str()
                .unwrap(),
            )
            .unwrap(),
            sdp: get_string(|buf, n| unsafe {
                libdatachannel_sys::rtcGetLocalDescription(self.id, buf, n)
            })?
            .to_str()
            .unwrap()
            .to_string(),
        })
    }

    pub fn remote_description(&self) -> Result<Description, Error> {
        Ok(Description {
            type_: SdpType::from_str(
                get_string(|buf, n| unsafe {
                    libdatachannel_sys::rtcGetRemoteDescriptionType(self.id, buf, n)
                })?
                .to_str()
                .unwrap(),
            )
            .unwrap(),
            sdp: get_string(|buf, n| unsafe {
                libdatachannel_sys::rtcGetRemoteDescription(self.id, buf, n)
            })?
            .to_str()
            .unwrap()
            .to_string(),
        })
    }

    pub fn local_address(&self) -> Result<std::net::SocketAddr, Error> {
        Ok(
            get_string(|buf, n| unsafe {
                libdatachannel_sys::rtcGetLocalAddress(self.id, buf, n)
            })?
            .to_str()
            .unwrap()
            .parse()
            .unwrap(),
        )
    }

    pub fn remote_address(&self) -> Result<std::net::SocketAddr, Error> {
        Ok(get_string(|buf, n| unsafe {
            libdatachannel_sys::rtcGetRemoteAddress(self.id, buf, n)
        })?
        .to_str()
        .unwrap()
        .parse()
        .unwrap())
    }

    pub fn max_data_channel_stream(&self) -> Result<u32, Error> {
        Ok(check_error(unsafe { libdatachannel_sys::rtcGetMaxDataChannelStream(self.id) })? as u32)
    }

    pub fn remote_max_message_size(&self) -> Result<u32, Error> {
        Ok(check_error(unsafe { libdatachannel_sys::rtcGetRemoteMaxMessageSize(self.id) })? as u32)
    }

    pub fn create_data_channel(
        &self,
        label: &str,
        init: DataChannelOptions,
    ) -> Result<DataChannel, Error> {
        let raw_label = std::ffi::CString::new(label).unwrap();

        let raw_protocol = std::ffi::CString::new(init.protocol).unwrap();

        let raw_data_channel_init = libdatachannel_sys::rtcDataChannelInit {
            reliability: libdatachannel_sys::rtcReliability {
                unordered: init.reliability.unordered,
                unreliable: init.reliability.unreliable,
                maxPacketLifeTime: init.reliability.max_packet_life_time,
                maxRetransmits: init.reliability.max_retransmits,
            },
            protocol: raw_protocol.as_ptr(),
            negotiated: init.negotiated,
            manualStream: init.stream.is_some(),
            stream: init.stream.unwrap_or(0),
        };

        let id = check_error(unsafe {
            libdatachannel_sys::rtcCreateDataChannelEx(
                self.id,
                raw_label.as_ptr(),
                &raw_data_channel_init as *const _,
            )
        })?;

        Ok(DataChannel::from_raw(id))
    }

    pub fn set_on_local_description(&mut self, cb: Option<impl Fn(&str, SdpType) + 'static>) {
        self.userdata.on_local_description = cb.map(|f| Box::new(f) as _);
    }

    pub fn set_on_local_candidate(&mut self, cb: Option<impl Fn(&str) + 'static>) {
        self.userdata.on_local_candidate = cb.map(|f| Box::new(f) as _);
    }

    pub fn set_on_state_change(&mut self, cb: Option<impl Fn(State) + 'static>) {
        self.userdata.on_state_change = cb.map(|f| Box::new(f) as _);
    }

    pub fn set_on_gathering_state_change(&mut self, cb: Option<impl Fn(GatheringState) + 'static>) {
        self.userdata.on_gathering_state_change = cb.map(|f| Box::new(f) as _);
    }

    pub fn set_on_data_channel(&mut self, cb: Option<impl Fn(DataChannel) + 'static>) {
        self.userdata.on_data_channel = cb.map(|f| Box::new(f) as _);
    }
}

impl Drop for PeerConnection {
    fn drop(&mut self) {
        unsafe {
            assert_eq!(libdatachannel_sys::rtcDeletePeerConnection(self.id), 0);
        }
    }
}

pub struct DataChannel {
    id: i32,
    userdata: Box<DataChannelUserData>,
}

#[derive(Default)]
struct DataChannelUserData {
    on_open: Option<Box<dyn Fn()>>,
    on_closed: Option<Box<dyn Fn()>>,
    on_error: Option<Box<dyn Fn(&str)>>,
    on_message: Option<Box<dyn Fn(&[u8])>>,
    on_buffered_amount_low: Option<Box<dyn Fn()>>,
    on_available: Option<Box<dyn Fn()>>,
}

impl DataChannel {
    fn from_raw(id: i32) -> Self {
        let mut userdata: Box<DataChannelUserData> = Default::default();

        unsafe {
            extern "C" fn open_callback(_id: i32, userdata: *mut std::ffi::c_void) {
                let ud = unsafe { &*(userdata as *mut DataChannelUserData) };
                if let Some(cb) = &ud.on_open {
                    cb();
                }
            }
            libdatachannel_sys::rtcSetOpenCallback(id, Some(open_callback))
        };

        unsafe {
            extern "C" fn closed_callback(_id: i32, userdata: *mut std::ffi::c_void) {
                let ud = unsafe { &*(userdata as *mut DataChannelUserData) };
                if let Some(cb) = &ud.on_closed {
                    cb();
                }
            }
            libdatachannel_sys::rtcSetClosedCallback(id, Some(closed_callback));
        };

        unsafe {
            extern "C" fn error_callback(
                _id: i32,
                error: *const std::ffi::c_char,
                userdata: *mut std::ffi::c_void,
            ) {
                let ud = unsafe { &*(userdata as *mut DataChannelUserData) };
                if let Some(cb) = &ud.on_error {
                    cb(&unsafe { std::ffi::CStr::from_ptr(error) }.to_str().unwrap());
                }
            }
            libdatachannel_sys::rtcSetErrorCallback(id, Some(error_callback))
        };

        unsafe {
            extern "C" fn message_callback(
                _id: i32,
                message: *const std::ffi::c_char,
                size: i32,
                userdata: *mut std::ffi::c_void,
            ) {
                let ud = unsafe { &*(userdata as *mut DataChannelUserData) };
                if let Some(cb) = &ud.on_message {
                    cb(unsafe { std::slice::from_raw_parts(message as *const _, size as usize) });
                }
            }
            libdatachannel_sys::rtcSetMessageCallback(id, Some(message_callback))
        };

        unsafe {
            extern "C" fn buffered_amount_low(_id: i32, userdata: *mut std::ffi::c_void) {
                let ud = unsafe { &*(userdata as *mut DataChannelUserData) };
                if let Some(cb) = &ud.on_buffered_amount_low {
                    cb();
                }
            }
            libdatachannel_sys::rtcSetBufferedAmountLowCallback(id, Some(buffered_amount_low))
        };

        unsafe {
            extern "C" fn available(_id: i32, userdata: *mut std::ffi::c_void) {
                let ud = unsafe { &*(userdata as *mut DataChannelUserData) };
                if let Some(cb) = &ud.on_available {
                    cb();
                }
            }
            libdatachannel_sys::rtcSetAvailableCallback(id, Some(available))
        };

        unsafe {
            libdatachannel_sys::rtcSetUserPointer(
                id,
                userdata.as_mut() as *mut DataChannelUserData as *mut _,
            );
        }

        DataChannel { id, userdata }
    }

    pub fn send(&self, buf: &[u8]) -> Result<(), Error> {
        check_error(unsafe {
            libdatachannel_sys::rtcSendMessage(self.id, buf.as_ptr() as *const _, buf.len() as i32)
        })?;
        Ok(())
    }

    pub fn close(&self) -> Result<(), Error> {
        check_error(unsafe { libdatachannel_sys::rtcClose(self.id) })?;
        Ok(())
    }

    pub fn is_open(&self) -> bool {
        unsafe { libdatachannel_sys::rtcIsOpen(self.id) }
    }

    pub fn is_closed(&self) -> bool {
        unsafe { libdatachannel_sys::rtcIsClosed(self.id) }
    }

    pub fn max_message_size(&self) -> Result<usize, Error> {
        Ok(check_error(unsafe { libdatachannel_sys::rtcMaxMessageSize(self.id) })? as usize)
    }

    pub fn buffered_amount(&self) -> Result<usize, Error> {
        Ok(check_error(unsafe { libdatachannel_sys::rtcGetBufferedAmount(self.id) })? as usize)
    }

    pub fn set_buffered_amount_low_threshold(&self, amount: usize) -> Result<(), Error> {
        check_error(unsafe {
            libdatachannel_sys::rtcSetBufferedAmountLowThreshold(self.id, amount as i32)
        })?;
        Ok(())
    }

    pub fn available_amount(&self) -> Result<usize, Error> {
        Ok(check_error(unsafe { libdatachannel_sys::rtcGetAvailableAmount(self.id) })? as usize)
    }

    pub fn set_on_open(&mut self, cb: Option<impl Fn() + 'static>) {
        self.userdata.on_open = cb.map(|f| Box::new(f) as _);
    }

    pub fn set_on_closed(&mut self, cb: Option<impl Fn() + 'static>) {
        self.userdata.on_closed = cb.map(|f| Box::new(f) as _);
    }

    pub fn set_on_buffered_amount_low(&mut self, cb: Option<impl Fn() + 'static>) {
        self.userdata.on_buffered_amount_low = cb.map(|f| Box::new(f) as _);
    }

    pub fn set_on_error(&mut self, cb: Option<impl Fn(&str) + 'static>) {
        self.userdata.on_error = cb.map(|f| Box::new(f) as _);
    }

    pub fn set_on_message(&mut self, cb: Option<impl Fn(&[u8]) + 'static>) {
        self.userdata.on_message = cb.map(|f| Box::new(f) as _);
    }
}

impl Drop for DataChannel {
    fn drop(&mut self) {
        unsafe {
            assert_eq!(libdatachannel_sys::rtcDeleteDataChannel(self.id), 0);
        }
    }
}

#[derive(Default)]
pub struct Reliability {
    pub unordered: bool,
    pub unreliable: bool,
    pub max_packet_life_time: u32,
    pub max_retransmits: u32,
}

#[derive(Default)]
pub struct DataChannelOptions {
    pub reliability: Reliability,
    pub protocol: String,
    pub negotiated: bool,
    pub stream: Option<u16>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    pub fn test_peer_connection_new() {
        let pc = PeerConnection::new(Default::default()).unwrap();
        pc.create_data_channel("test", Default::default()).unwrap();
    }

    #[test]
    pub fn test_peer_connection_communicate() {
        let mut pc1 = PeerConnection::new(Default::default()).unwrap();
        let pc1_gathered =
            std::sync::Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
        pc1.set_on_gathering_state_change(Some({
            let pc1_gathered = std::sync::Arc::clone(&pc1_gathered);
            move |ice_gathering_state| {
                if ice_gathering_state == GatheringState::Complete {
                    let (lock, condvar) = &*pc1_gathered;
                    *lock.lock().unwrap() = true;
                    condvar.notify_one();
                }
            }
        }));

        let mut dc1 = pc1
            .create_data_channel(
                "test",
                DataChannelOptions {
                    negotiated: true,
                    stream: Some(1),
                    ..Default::default()
                },
            )
            .unwrap();
        let dc1_open =
            std::sync::Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
        dc1.set_on_open(Some({
            let dc1_open = std::sync::Arc::clone(&dc1_open);
            move || {
                let (lock, condvar) = &*dc1_open;
                *lock.lock().unwrap() = true;
                condvar.notify_one();
            }
        }));
        pc1.set_local_description(Some(SdpType::Offer)).unwrap();
        let _pc1_gathered_guard = pc1_gathered
            .1
            .wait_while(pc1_gathered.0.lock().unwrap(), |ready| !*ready)
            .unwrap();

        let mut pc2 = PeerConnection::new(Default::default()).unwrap();
        let pc2_gathered =
            std::sync::Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
        pc2.set_on_gathering_state_change(Some({
            let pc2_gathered = std::sync::Arc::clone(&pc2_gathered);
            move |ice_gathering_state| {
                if ice_gathering_state == GatheringState::Complete {
                    let (lock, condvar) = &*pc2_gathered;
                    *lock.lock().unwrap() = true;
                    condvar.notify_one();
                }
            }
        }));

        let mut dc2 = pc2
            .create_data_channel(
                "test",
                DataChannelOptions {
                    negotiated: true,
                    stream: Some(1),
                    ..Default::default()
                },
            )
            .unwrap();
        let dc2_open =
            std::sync::Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
        dc2.set_on_open(Some({
            let dc2_open = std::sync::Arc::clone(&dc2_open);
            move || {
                let (lock, condvar) = &*dc2_open;
                *lock.lock().unwrap() = true;
                condvar.notify_one();
            }
        }));

        let (tx1, rx1) = std::sync::mpsc::sync_channel(0);
        dc1.set_on_message(Some(move |msg: &[u8]| {
            tx1.send(msg.to_vec()).unwrap();
        }));

        let (tx2, rx2) = std::sync::mpsc::sync_channel(0);
        dc2.set_on_message(Some(move |msg: &[u8]| {
            tx2.send(msg.to_vec()).unwrap();
        }));

        pc2.set_remote_description(&pc1.local_description().unwrap())
            .unwrap();
        pc1.set_remote_description(&pc2.local_description().unwrap())
            .unwrap();

        let _pc2_gathered_guard = pc2_gathered
            .1
            .wait_while(pc2_gathered.0.lock().unwrap(), |ready| !*ready)
            .unwrap();

        let _dc1_open_guard = dc1_open
            .1
            .wait_while(dc1_open.0.lock().unwrap(), |ready| !*ready)
            .unwrap();
        let _dc2_open_guard = dc2_open
            .1
            .wait_while(dc2_open.0.lock().unwrap(), |ready| !*ready)
            .unwrap();

        dc1.send(b"hello world!").unwrap();
        assert_eq!(rx2.recv().unwrap(), b"hello world!");

        dc2.send(b"goodbye world!").unwrap();
        assert_eq!(rx1.recv().unwrap(), b"goodbye world!");
    }
}
