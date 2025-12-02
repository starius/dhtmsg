use std::{
    collections::HashSet,
    net::{SocketAddrV4, UdpSocket},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use clap::Parser;
use log::{error, info, warn};
use mainline::Id;
use rand::{RngCore, thread_rng};
use sha1::{Digest, Sha1};

#[derive(Parser, Debug)]
#[command(
    name = "dhtmsg",
    about = "Tiny UDP hello over BitTorrent DHT peer discovery"
)]
struct Args {
    /// Local identifier hex string (random if omitted)
    #[arg(long)]
    id: Option<String>,

    /// Target peer identifier hex string to contact (derives infohash)
    #[arg(long)]
    peer: Option<String>,

    /// Re-announce interval in seconds
    #[arg(long, default_value_t = 45)]
    announce_secs: u64,
}

fn main() -> Result<()> {
    init_logging();
    let args = Args::parse();

    let local_id = args.id.clone().unwrap_or_else(random_hex_id);
    let local_infohash = derive_infohash(&local_id)?;
    info!("local ID: {local_id}");
    info!("derived infohash: {}", local_infohash);

    let socket = UdpSocket::bind(("0.0.0.0", 0)).context("failed to bind UDP socket")?;
    socket
        .set_nonblocking(true)
        .context("failed to set socket to non-blocking")?;
    let hello_port = socket
        .local_addr()
        .context("failed to read bound port")?
        .port();
    info!("hello socket bound on UDP port {hello_port}");

    let dht = mainline::Dht::builder()
        .build()
        .context("failed to start DHT node")?;
    info!("DHT socket listening on {}", dht.info().local_addr());

    info!("bootstrapping the DHT...");
    thread::sleep(Duration::from_secs(2));
    info!("bootstrapped: {}", dht.bootstrapped());

    announce(&dht, local_infohash);

    let recv_socket = socket.try_clone().context("failed to clone UDP socket")?;
    let recv_id = local_id.clone();
    thread::spawn(move || recv_loop(recv_socket, recv_id));

    if let Some(peer_id) = args.peer.as_deref() {
        let peer_infohash = derive_infohash(peer_id)?;
        info!("peer ID: {peer_id}");
        info!("peer infohash: {}", peer_infohash);
        lookup_and_hello(
            dht,
            socket,
            local_id,
            local_infohash,
            peer_infohash,
            args.announce_secs,
        );
    } else {
        info!("no peer provided; announcing and waiting for inbound hello. Ctrl+C to quit.");
        idle_announce_loop(dht, local_infohash, args.announce_secs);
    }

    Ok(())
}

fn init_logging() {
    use simplelog::{ColorChoice, ConfigBuilder, LevelFilter, TermLogger, TerminalMode};

    let config = ConfigBuilder::new()
        .set_time_level(LevelFilter::Off)
        .set_level_padding(simplelog::LevelPadding::Right)
        .build();

    let _ = TermLogger::init(
        LevelFilter::Info,
        config,
        TerminalMode::Mixed,
        ColorChoice::Auto,
    );
}

fn random_hex_id() -> String {
    let mut bytes = [0u8; 16];
    thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn derive_infohash(id_hex: &str) -> Result<Id> {
    let raw_id = hex::decode(id_hex).with_context(|| format!("invalid hex ID string: {id_hex}"))?;
    let mut hasher = Sha1::new();
    hasher.update(&raw_id);
    let digest = hasher.finalize();
    Id::from_bytes(digest.as_slice()).context("failed to convert digest into infohash")
}

fn announce(dht: &mainline::Dht, infohash: Id) {
    // Use implied (observed) external port; we don't need to know it locally.
    match dht.announce_peer(infohash, None) {
        Ok(_) => info!("announced infohash {} (implied port)", infohash),
        Err(err) => warn!("announce failed: {err}"),
    }
}

fn recv_loop(socket: UdpSocket, local_id: String) {
    let mut buf = [0u8; 1500];
    loop {
        match socket.recv_from(&mut buf) {
            Ok((len, peer)) => {
                let msg = String::from_utf8_lossy(&buf[..len]);
                info!("received hello from {peer}: {msg}");
                let ack = format!("hello-ack from {local_id}");
                if let Err(err) = socket.send_to(ack.as_bytes(), peer) {
                    warn!("failed to send ack to {peer}: {err}");
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(200));
            }
            Err(err) => {
                error!("UDP recv error: {err}");
                thread::sleep(Duration::from_secs(1));
            }
        }
    }
}

fn lookup_and_hello(
    dht: mainline::Dht,
    socket: UdpSocket,
    local_id: String,
    local_infohash: Id,
    peer_infohash: Id,
    announce_secs: u64,
) {
    let mut seen: HashSet<SocketAddrV4> = HashSet::new();
    let mut last_announce = Instant::now();
    info!("starting lookup loop; Ctrl+C to stop.");
    loop {
        if last_announce.elapsed() >= Duration::from_secs(announce_secs) {
            announce(&dht, local_infohash);
            last_announce = Instant::now();
        }

        let iter = dht.get_peers(peer_infohash);
        for peers in iter {
            for addr in peers {
                if seen.insert(addr) {
                    info!("found peer candidate {addr}, sending hello...");
                    if let Err(err) = send_hello(&socket, addr, &local_id) {
                        warn!("failed to send hello to {addr}: {err}");
                    }
                }
            }
        }

        thread::sleep(Duration::from_secs(5));
    }
}

fn send_hello(socket: &UdpSocket, addr: SocketAddrV4, local_id: &str) -> Result<()> {
    let payload = format!("hello from {local_id}");
    socket
        .send_to(payload.as_bytes(), addr)
        .with_context(|| format!("sending hello to {addr}"))?;
    Ok(())
}

fn idle_announce_loop(dht: mainline::Dht, infohash: Id, announce_secs: u64) {
    let mut last_announce = Instant::now();
    loop {
        if last_announce.elapsed() >= Duration::from_secs(announce_secs) {
            announce(&dht, infohash);
            last_announce = Instant::now();
        }

        thread::sleep(Duration::from_secs(5));
    }
}
