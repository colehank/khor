//! Fixed-plan UDP packets. All counters and windows use the local endpoint clock.
use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::net::UdpSocket;
use tokio::time::{Instant, MissedTickBehavior};

pub(super) const SIZE: usize = 1200;
pub(super) const RATE: u64 = 70_000_000;
pub(super) const TICK_US: u64 = 5000;
pub(super) const PACKETS: usize = (20 * RATE / (SIZE as u64 * 8)) as usize;
const WORDS: usize = PACKETS.div_ceil(64);

pub(super) struct Sent {
    pub local: SocketAddr,
    pub peer: SocketAddr,
    pub elapsed_us: u64,
    pub sent: Vec<u64>,
    pub failed: Vec<u64>,
    pub by_second: [u64; 21],
    pub ticks: u64,
    pub max_lateness_us: u64,
    pub late_over_10ms: u64,
    pub first_error: Option<String>,
}

pub(super) struct Received {
    pub local: SocketAddr,
    pub peer: Option<SocketAddr>,
    pub elapsed_us: u64,
    pub first_after_us: Option<u64>,
    pub received: Vec<u64>,
    pub by_second: [u64; 25],
    pub datagrams: u64,
    pub duplicates: u64,
    pub malformed: u64,
    pub foreign: u64,
    pub out_of_range: u64,
}

impl Sent {
    pub(super) fn record(&self) -> Value {
        json!({"local":self.local,"peer":self.peer,"elapsed_us":self.elapsed_us,
            "sent":self.sent,"failed":self.failed,"by_second":self.by_second,
            "ticks":self.ticks,"max_lateness_us":self.max_lateness_us,
            "late_over_10ms":self.late_over_10ms,"first_error":self.first_error})
    }
}

impl Received {
    pub(super) fn record(&self) -> Value {
        json!({"local":self.local,"peer":self.peer,"elapsed_us":self.elapsed_us,
            "first_after_us":self.first_after_us,"received":self.received,"by_second":self.by_second,
            "datagrams":self.datagrams,"duplicates":self.duplicates,"malformed":self.malformed,
            "foreign":self.foreign,"out_of_range":self.out_of_range})
    }
}

pub(super) fn population(words: &[u64]) -> u64 {
    words.iter().map(|w| u64::from(w.count_ones())).sum()
}

fn mark(words: &mut [u64], seq: usize) {
    words[seq / 64] |= 1 << (seq % 64);
}

fn packet(seq: usize, tag: u64) -> [u8; SIZE] {
    let mut bytes = [0; SIZE];
    bytes[..8].copy_from_slice(&tag.to_le_bytes());
    bytes[8..16].copy_from_slice(&(seq as u64).to_le_bytes());
    for (index, byte) in bytes[16..].iter_mut().enumerate() {
        *byte = (seq as u8).wrapping_add(index as u8).wrapping_add(0x61);
    }
    bytes
}

pub(super) async fn send(ip: Ipv4Addr, peer: SocketAddr, tag: u64) -> Sent {
    let socket = UdpSocket::bind((ip, 0)).await.unwrap();
    socket.connect(peer).await.unwrap();
    let began = Instant::now();
    let end = began + Duration::from_secs(20);
    let mut result = Sent {
        local: socket.local_addr().unwrap(),
        peer,
        elapsed_us: 0,
        sent: vec![0; WORDS],
        failed: vec![0; WORDS],
        by_second: [0; 21],
        ticks: 0,
        max_lateness_us: 0,
        late_over_10ms: 0,
        first_error: None,
    };
    let mut timer = tokio::time::interval_at(began, Duration::from_micros(TICK_US));
    timer.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut previous_slot = None;
    'paced: loop {
        let scheduled = timer.tick().await;
        let now = Instant::now();
        let slot = now.duration_since(began).as_micros() as u64 / TICK_US;
        if slot >= 20_000_000 / TICK_US || now >= end {
            break;
        }
        if previous_slot == Some(slot) {
            continue;
        }
        previous_slot = Some(slot);
        let late = now.saturating_duration_since(scheduled).as_micros() as u64;
        result.ticks += 1;
        result.max_lateness_us = result.max_lateness_us.max(late);
        result.late_over_10ms += u64::from(late > 10_000);
        let divisor = SIZE as u64 * 8 * 1_000_000;
        let first = (slot * TICK_US * RATE / divisor) as usize;
        let last = (((slot + 1) * TICK_US * RATE / divisor) as usize).min(PACKETS);
        for seq in first..last {
            let bytes = packet(seq, tag);
            let sent = tokio::time::timeout_at(end, socket.send(&bytes)).await;
            match sent {
                Ok(Ok(SIZE)) => {
                    mark(&mut result.sent, seq);
                    let second = began.elapsed().as_secs().min(20) as usize;
                    result.by_second[second] += 1;
                }
                other => {
                    mark(&mut result.failed, seq);
                    if result.first_error.is_none() {
                        result.first_error = Some(format!("{other:?}"));
                    }
                    if other.is_err() {
                        break 'paced;
                    }
                }
            }
        }
    }
    result.elapsed_us = began.elapsed().as_micros() as u64;
    result
}

pub(super) async fn receive(
    ip: Ipv4Addr,
    expected: Ipv4Addr,
    tag: u64,
    ready: tokio::sync::oneshot::Sender<SocketAddr>,
) -> Received {
    let socket = UdpSocket::bind((ip, 0)).await.unwrap();
    let began = Instant::now();
    let end = began + Duration::from_secs(25);
    let mut result = Received {
        local: socket.local_addr().unwrap(),
        peer: None,
        elapsed_us: 0,
        first_after_us: None,
        received: vec![0; WORDS],
        by_second: [0; 25],
        datagrams: 0,
        duplicates: 0,
        malformed: 0,
        foreign: 0,
        out_of_range: 0,
    };
    ready.send(result.local).unwrap();
    let mut first = None;
    loop {
        let mut bytes = [0; SIZE + 1];
        let received = tokio::time::timeout_at(end, socket.recv_from(&mut bytes)).await;
        let (length, peer) = match received {
            Ok(Ok(value)) => value,
            Ok(Err(error)) => panic!("UDP receiver: {error}"),
            Err(_) => break,
        };
        result.datagrams += 1;
        if peer.ip() != std::net::IpAddr::V4(expected) || result.peer.is_some_and(|p| p != peer) {
            result.foreign += 1;
            continue;
        }
        if length != SIZE || bytes[..8] != tag.to_le_bytes() {
            result.malformed += 1;
            continue;
        }
        let seq = u64::from_le_bytes(bytes[8..16].try_into().unwrap()) as usize;
        if seq >= PACKETS {
            result.out_of_range += 1;
            continue;
        }
        if bytes[..SIZE] != packet(seq, tag) {
            result.malformed += 1;
            continue;
        }
        if result.received[seq / 64] & (1 << (seq % 64)) != 0 {
            result.duplicates += 1;
            continue;
        }
        let now = Instant::now();
        let first = first.get_or_insert_with(|| {
            result.first_after_us = Some(began.elapsed().as_micros() as u64);
            now
        });
        result.peer = Some(peer);
        mark(&mut result.received, seq);
        let second = now.duration_since(*first).as_secs().min(24) as usize;
        result.by_second[second] += 1;
    }
    result.elapsed_us = began.elapsed().as_micros() as u64;
    result
}

pub(super) fn rate(buckets: &[u64]) -> f64 {
    buckets[5..15].iter().sum::<u64>() as f64 * SIZE as f64 * 8.0 / 10_000_000.0
}

pub(super) fn valid(sent: &Sent, received: &Received) -> bool {
    let sent_count = population(&sent.sent);
    let received_count = population(&received.received);
    population(&sent.failed) == 0
        && sent.first_error.is_none()
        && sent_count * 100 >= PACKETS as u64 * 99
        && (19_000_000..=21_000_000).contains(&sent.elapsed_us)
        && (65.0..=75.0).contains(&rate(&sent.by_second))
        && sent.max_lateness_us <= 20_000
        && sent.late_over_10ms * 100 <= sent.ticks
        && received.first_after_us.is_some_and(|t| t <= 2_000_000)
        && (24_000_000..=26_000_000).contains(&received.elapsed_us)
        && sent
            .sent
            .iter()
            .zip(&received.received)
            .all(|(s, r)| r & !s == 0)
        && received.peer == Some(sent.local)
        && received.local == sent.peer
        && sent.by_second.iter().sum::<u64>() == sent_count
        && received.by_second.iter().sum::<u64>() == received_count
        && received.datagrams == received_count
        && received.duplicates == 0
        && received.malformed == 0
        && received.foreign == 0
        && received.out_of_range == 0
}
