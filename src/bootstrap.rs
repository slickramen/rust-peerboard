use libp2p::{Multiaddr, PeerId};

pub struct BootstrapNode {
    pub peer_id: PeerId,
    pub addrs: Vec<Multiaddr>,
}

pub fn bootstrap_node() -> BootstrapNode {
    BootstrapNode {
        peer_id: "12D3KooWCvwqT3JUzVQczCvAVFa9EGzNqjHHSMVHVhm3RVyscCNY".parse().unwrap(),
        addrs: vec![
            "/ip4/170.64.177.57/tcp/8000".parse().unwrap(),
            "/ip4/170.64.177.57/udp/8000/quic-v1".parse().unwrap(),
        ],
    }
}