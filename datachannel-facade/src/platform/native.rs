//! Native platform-specific functionality.

/// Native platform-specific extensions to [`crate::Configuration`].
pub trait ConfigurationExt {
    /// Set an address and port range to bind to.
    fn set_bind(&mut self, addr: std::net::IpAddr, port_range_start: u16, port_range_end: u16);

    /// If true, connections are multiplexed on the same UDP port.
    fn set_enable_ice_udp_mux(&mut self, value: bool);
}

impl ConfigurationExt for crate::Configuration {
    fn set_bind(&mut self, addr: std::net::IpAddr, port_range_begin: u16, port_range_end: u16) {
        self.sys.bind_address = Some(addr);
        self.sys.port_range_begin = port_range_begin;
        self.sys.port_range_end = port_range_end;
    }

    fn set_enable_ice_udp_mux(&mut self, value: bool) {
        self.sys.enable_ice_udp_mux = value;
    }
}
