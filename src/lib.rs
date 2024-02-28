mod sync_util;

mod channel;
mod connection;

pub use channel::*;
pub use connection::*;

pub use datachannel_facade::Error;
pub use datachannel_facade::IceServer;
pub use datachannel_facade::IceTransportPolicy;

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
    pub async fn test_connection_new() {
        let cb = Connection::builder(Default::default()).unwrap();
        cb.create_data_channel("test", Default::default()).unwrap();
        let _conn = cb.build();
    }

    #[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    pub async fn test_connection_communicate() {
        let cb1 = Connection::builder(Default::default()).unwrap();
        let chan1 = cb1
            .create_data_channel(
                "test",
                DataChannelOptions {
                    negotiated: true,
                    id: Some(1),
                    ..Default::default()
                },
            )
            .unwrap();
        let conn1 = cb1.build();
        conn1.set_local_description(SdpType::Offer).await.unwrap();
        conn1.ice_candidates_gathered().await;

        let cb2 = Connection::builder(Default::default()).unwrap();
        let chan2 = cb2
            .create_data_channel(
                "test",
                DataChannelOptions {
                    negotiated: true,
                    id: Some(1),
                    ..Default::default()
                },
            )
            .unwrap();

        let conn2 = cb2.build();
        conn2
            .set_remote_description(&conn1.local_description().unwrap().unwrap())
            .await
            .unwrap();

        conn2.set_local_description(SdpType::Answer).await.unwrap();
        conn2.ice_candidates_gathered().await;

        conn1
            .set_remote_description(&conn2.local_description().unwrap().unwrap())
            .await
            .unwrap();

        chan1.send(b"hello world!").await.unwrap();
        assert_eq!(chan2.recv().await.unwrap(), b"hello world!");

        chan2.send(b"goodbye world!").await.unwrap();
        assert_eq!(chan1.recv().await.unwrap(), b"goodbye world!");
    }

    #[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    pub async fn test_connection_communicate_nonnegotiated() {
        let cb1 = Connection::builder(Default::default()).unwrap();
        let chan1 = cb1.create_data_channel("test", Default::default()).unwrap();
        let conn1 = cb1.build();
        conn1.set_local_description(SdpType::Offer).await.unwrap();
        conn1.ice_candidates_gathered().await;

        let cb2 = Connection::builder(Default::default()).unwrap();
        let conn2 = cb2.build();
        conn2
            .set_remote_description(&conn1.local_description().unwrap().unwrap())
            .await
            .unwrap();

        conn2.set_local_description(SdpType::Answer).await.unwrap();
        conn2.ice_candidates_gathered().await;

        conn1
            .set_remote_description(&conn2.local_description().unwrap().unwrap())
            .await
            .unwrap();

        let chan2 = conn2.accept_channel().await.unwrap();

        chan1.send(b"hello world!").await.unwrap();
        assert_eq!(chan2.recv().await.unwrap(), b"hello world!");

        chan2.send(b"goodbye world!").await.unwrap();
        assert_eq!(chan1.recv().await.unwrap(), b"goodbye world!");
    }
}
