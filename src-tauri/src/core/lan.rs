//! LAN Discovery: listens for Minecraft's own "Open to LAN" broadcasts.
//!
//! When a vanilla Minecraft world enables "Open to LAN", the game itself
//! (no launcher, no server, no account beyond the one already playing)
//! broadcasts a small UDP multicast packet to `224.0.2.60:4445` roughly
//! every 1.5 seconds:
//!
//! ```text
//! [MOTD]<motd>[/MOTD][AD]<port>[/AD]
//! ```
//!
//! Any listener on the same local network can receive it - this is the one
//! "multiplayer-lite" feature this launcher can build for real without a
//! backend server, unlike Friends/Party (see `ComingSoonPanel`'s doc
//! comment), because the broadcasting side is stock Minecraft doing what it
//! already does.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddrV4};
use std::time::{Duration, Instant};

use serde::Serialize;
use socket2::{Domain, Socket, Type};
use tokio::net::UdpSocket;

pub const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 2, 60);
pub const MULTICAST_PORT: u16 = 4445;

/// A LAN game stops being listed if no fresh broadcast from it arrives
/// within this window. Vanilla rebroadcasts roughly every 1.5s, so this
/// comfortably survives a couple of dropped packets without keeping a
/// world that already closed "Open to LAN" listed for long.
const STALE_TIMEOUT: Duration = Duration::from_secs(5);
const SWEEP_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanGame {
    pub host: String,
    pub port: u16,
    pub motd: String,
}

/// Sent whenever the set of currently-open LAN games changes (a new
/// broadcast, or one going stale) - always the full current list, so the
/// frontend just replaces its state instead of reducing granular
/// added/removed events.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum LanEvent {
    Snapshot { games: Vec<LanGame> },
}

/// Where `LanEvent`s go - mirrors `core::downloads::ProgressSink` so
/// callers that don't care (tests) can use `LanEventSink::none()` instead
/// of threading `Option` handling through every call site.
#[derive(Clone)]
pub struct LanEventSink(Option<tokio::sync::mpsc::UnboundedSender<LanEvent>>);

impl LanEventSink {
    pub fn new(sender: tokio::sync::mpsc::UnboundedSender<LanEvent>) -> Self {
        Self(Some(sender))
    }

    pub fn none() -> Self {
        Self(None)
    }

    fn send(&self, event: LanEvent) {
        if let Some(sender) = &self.0 {
            let _ = sender.send(event);
        }
    }
}

/// Builds and binds the multicast-listening socket, joining the LAN
/// broadcast group. Split out from `run` so the Tauri command layer can
/// surface a real bind failure (e.g. no multicast-capable network
/// interface) to the frontend immediately, instead of it failing silently
/// inside a spawned background task.
pub fn bind_multicast_socket() -> std::io::Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
    socket.set_reuse_address(true)?;
    #[cfg(unix)]
    socket.set_reuse_port(true)?;
    socket.bind(&SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, MULTICAST_PORT).into())?;
    socket.set_nonblocking(true)?;
    socket.join_multicast_v4(&MULTICAST_ADDR, &Ipv4Addr::UNSPECIFIED)?;
    UdpSocket::from_std(socket.into())
}

/// Parses one broadcast payload into `(motd, port)`. Vanilla's own format
/// has no escaping for a literal `[MOTD]`/`[/MOTD]`/`[AD]`/`[/AD]` inside
/// the MOTD itself, so - same as Minecraft's own client - this just takes
/// the first matching pair of tags rather than trying to handle nesting.
fn parse_broadcast(payload: &str) -> Option<(String, u16)> {
    let motd_start = payload.find("[MOTD]")? + "[MOTD]".len();
    let motd_end = motd_start + payload[motd_start..].find("[/MOTD]")?;
    let motd = payload[motd_start..motd_end].to_string();

    let rest = &payload[motd_end..];
    let ad_start = rest.find("[AD]")? + "[AD]".len();
    let ad_end = ad_start + rest[ad_start..].find("[/AD]")?;
    let port: u16 = rest[ad_start..ad_end].trim().parse().ok()?;

    Some((motd, port))
}

fn snapshot(games: &HashMap<(IpAddr, u16), (LanGame, Instant)>) -> LanEvent {
    let mut list: Vec<LanGame> = games.values().map(|(game, _)| game.clone()).collect();
    list.sort_by(|a, b| a.host.cmp(&b.host).then(a.port.cmp(&b.port)));
    LanEvent::Snapshot { games: list }
}

/// Runs the receive loop against an already-bound socket until the process
/// exits or the caller aborts the task - there's no graceful internal
/// shutdown because `tokio::task::JoinHandle::abort` already cancels it
/// cleanly from outside, which is all the Tauri command layer needs to stop
/// listening when the LAN tab is no longer visible.
pub async fn run(socket: UdpSocket, events: LanEventSink) -> std::io::Result<()> {
    let mut games: HashMap<(IpAddr, u16), (LanGame, Instant)> = HashMap::new();
    let mut buf = [0u8; 2048];
    let mut sweep = tokio::time::interval(SWEEP_INTERVAL);
    // The first tick fires immediately - skip it so a needless empty
    // snapshot isn't emitted before any broadcast has had a chance to
    // arrive.
    sweep.tick().await;

    loop {
        tokio::select! {
            received = socket.recv_from(&mut buf) => {
                let (len, addr) = received?;
                let payload = String::from_utf8_lossy(&buf[..len]);
                if let Some((motd, port)) = parse_broadcast(&payload) {
                    let host = addr.ip();
                    games.insert(
                        (host, port),
                        (LanGame { host: host.to_string(), port, motd }, Instant::now()),
                    );
                    events.send(snapshot(&games));
                }
            }
            _ = sweep.tick() => {
                let before = games.len();
                games.retain(|_, (_, seen)| seen.elapsed() < STALE_TIMEOUT);
                if games.len() != before {
                    events.send(snapshot(&games));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_broadcast_reads_the_motd_and_port_out_of_a_real_vanilla_payload() {
        let payload = "[MOTD]A Minecraft Server[/MOTD][AD]25565[/AD]";
        assert_eq!(
            parse_broadcast(payload),
            Some(("A Minecraft Server".to_string(), 25565))
        );
    }

    #[test]
    fn parse_broadcast_handles_an_empty_motd() {
        let payload = "[MOTD][/MOTD][AD]12345[/AD]";
        assert_eq!(parse_broadcast(payload), Some(("".to_string(), 12345)));
    }

    #[test]
    fn parse_broadcast_rejects_a_missing_motd_tag() {
        assert_eq!(parse_broadcast("[AD]25565[/AD]"), None);
    }

    #[test]
    fn parse_broadcast_rejects_a_missing_ad_tag() {
        assert_eq!(parse_broadcast("[MOTD]A World[/MOTD]"), None);
    }

    #[test]
    fn parse_broadcast_rejects_a_non_numeric_port() {
        assert_eq!(
            parse_broadcast("[MOTD]A World[/MOTD][AD]not-a-port[/AD]"),
            None
        );
    }

    #[test]
    fn parse_broadcast_rejects_unrelated_garbage() {
        assert_eq!(parse_broadcast("just some random UDP noise"), None);
    }

    /// Real integration check, in the same "best-effort" spirit as
    /// `launch::tests::launches_a_real_java_process...`: binds the actual
    /// multicast socket, sends a real UDP packet to the real multicast
    /// group from a second socket, and proves it comes back out as a
    /// `Snapshot` event - real networking end to end, not just the pure
    /// parser. Skips (doesn't fail) if this sandbox has no multicast-capable
    /// loopback, the same posture the JDK-dependent test takes for a
    /// missing JDK.
    #[tokio::test]
    async fn receives_a_real_multicast_broadcast_and_reports_it_in_a_snapshot() {
        let socket = match bind_multicast_socket() {
            Ok(socket) => socket,
            Err(err) => {
                eprintln!(
                    "couldn't bind the LAN multicast socket in this environment ({err}) - skipping"
                );
                return;
            }
        };

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let handle = tokio::spawn(run(socket, LanEventSink::new(tx)));

        let Ok(sender) = UdpSocket::bind("0.0.0.0:0").await else {
            handle.abort();
            eprintln!("couldn't open a UDP socket to send a test broadcast - skipping");
            return;
        };
        let payload = b"[MOTD]A Test World[/MOTD][AD]25565[/AD]";
        let target = (MULTICAST_ADDR, MULTICAST_PORT);

        // Resends every 300ms rather than guessing one fixed delay - some
        // environments take a moment to actually join the multicast group
        // at the OS level after `join_multicast_v4` returns.
        let received = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let _ = sender.send_to(payload, target).await;
                if let Ok(Some(LanEvent::Snapshot { games })) =
                    tokio::time::timeout(Duration::from_millis(300), rx.recv()).await
                {
                    if !games.is_empty() {
                        return games;
                    }
                }
            }
        })
        .await;

        handle.abort();

        let Ok(games) = received else {
            eprintln!(
                "no multicast broadcast came back in this environment within 5s - sandboxes \
                 without loopback multicast routing can't run this check, skipping"
            );
            return;
        };

        assert_eq!(games.len(), 1);
        assert_eq!(games[0].port, 25565);
        assert_eq!(games[0].motd, "A Test World");
    }
}
