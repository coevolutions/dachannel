use super::*;

pub async fn test_peer_connection_new() {
    let pc = PeerConnection::new(Default::default()).unwrap();
    pc.create_data_channel("test", Default::default()).unwrap();
}

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
