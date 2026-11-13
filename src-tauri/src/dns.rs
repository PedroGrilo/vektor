//! DNS test and switch. The test sends real DNS queries (UDP 53) to each public
//! resolver and to the PC's current one and times the answers. Applying writes
//! the servers into the TCP/IP settings of the connected adapters (IPv4 and,
//! where present, IPv6), journaled under [`TWEAK_ID`].

use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::error::{Error, Result};
use crate::journal::{self, Hive, RegValue};
use crate::win::registry;

pub const TWEAK_ID: &str = "dns-servers";
const TRIES: usize = 5;
const TIMEOUT: Duration = Duration::from_millis(900);
const QUERY_NAME: &str = "www.google.com";
const TCPIP: &str = r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces";
const TCPIP6: &str = r"SYSTEM\CurrentControlSet\Services\Tcpip6\Parameters\Interfaces";

struct Provider {
    id: &'static str,
    name: &'static str,
    v4: [&'static str; 2],
    v6: [&'static str; 2],
}

const PROVIDERS: &[Provider] = &[
    Provider { id: "cloudflare", name: "Cloudflare", v4: ["1.1.1.1", "1.0.0.1"], v6: ["2606:4700:4700::1111", "2606:4700:4700::1001"] },
    Provider { id: "google", name: "Google", v4: ["8.8.8.8", "8.8.4.4"], v6: ["2001:4860:4860::8888", "2001:4860:4860::8844"] },
    Provider { id: "quad9", name: "Quad9", v4: ["9.9.9.9", "149.112.112.112"], v6: ["2620:fe::fe", "2620:fe::9"] },
    Provider { id: "opendns", name: "OpenDNS", v4: ["208.67.222.222", "208.67.220.220"], v6: ["2620:119:35::35", "2620:119:53::53"] },
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsResult {
    /// provider id, or "current" for the PC's own DNS
    pub id: String,
    pub name: String,
    pub servers: Vec<String>,
    /// median answer time; None when it never answered
    pub latency_ms: Option<f64>,
    /// share of queries answered, 0..1
    pub success: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsStatus {
    /// IPv4 DNS servers in use on the connected adapters
    pub current: Vec<String>,
    /// provider applied by Vektor, if any
    pub applied: Option<String>,
}

// ---------- DNS wire format (just enough for one A query) ----------

pub fn build_query(id: u16, name: &str) -> Vec<u8> {
    let mut q = Vec::with_capacity(32);
    q.extend_from_slice(&id.to_be_bytes());
    q.extend_from_slice(&[0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 0]); // RD, 1 question
    for label in name.split('.').filter(|l| !l.is_empty()) {
        q.push(label.len().min(63) as u8);
        q.extend_from_slice(&label.as_bytes()[..label.len().min(63)]);
    }
    q.extend_from_slice(&[0, 0, 1, 0, 1]); // root, type A, class IN
    q
}

/// A reply to our query: same id, QR bit set, no server failure.
pub fn is_answer(id: u16, buf: &[u8]) -> bool {
    buf.len() >= 12 && buf[..2] == id.to_be_bytes() && buf[2] & 0x80 != 0 && buf[3] & 0x0F != 2
}

fn time_server(ip: IpAddr) -> Vec<f64> {
    let bind: SocketAddr = if ip.is_ipv4() { "0.0.0.0:0".parse().unwrap() } else { "[::]:0".parse().unwrap() };
    let Ok(sock) = UdpSocket::bind(bind) else { return Vec::new() };
    if sock.set_read_timeout(Some(TIMEOUT)).is_err() || sock.connect(SocketAddr::new(ip, 53)).is_err() {
        return Vec::new();
    }
    let mut times = Vec::new();
    let mut buf = [0u8; 512];
    for i in 0..TRIES {
        let id = (std::process::id() as u16).wrapping_mul(31).wrapping_add(i as u16);
        let start = Instant::now();
        if sock.send(&build_query(id, QUERY_NAME)).is_err() {
            continue;
        }
        // Skip stale replies from an earlier, timed-out try.
        while let Ok(n) = sock.recv(&mut buf) {
            if is_answer(id, &buf[..n]) {
                times.push(start.elapsed().as_secs_f64() * 1000.0);
                break;
            }
        }
    }
    times
}

pub fn median(v: &mut [f64]) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let m = v.len() / 2;
    Some(if v.len().is_multiple_of(2) { (v[m - 1] + v[m]) / 2.0 } else { v[m] })
}

fn measure(id: String, name: String, servers: Vec<String>) -> DnsResult {
    let mut times = servers
        .first()
        .and_then(|s| s.parse::<IpAddr>().ok())
        .map(time_server)
        .unwrap_or_default();
    let success = times.len() as f64 / TRIES as f64;
    let latency_ms = median(&mut times).map(|m| (m * 10.0).round() / 10.0);
    DnsResult { id, name, servers, latency_ms, success }
}

/// Time every provider and the current DNS, in parallel. Fastest first.
pub fn test() -> Result<Vec<DnsResult>> {
    let mut jobs: Vec<(String, String, Vec<String>)> = PROVIDERS
        .iter()
        .map(|p| (p.id.to_string(), p.name.to_string(), p.v4.iter().map(|s| s.to_string()).collect()))
        .collect();
    let current = current_servers();
    if !current.is_empty() && !PROVIDERS.iter().any(|p| p.v4[0] == current[0]) {
        jobs.insert(0, ("current".into(), "O teu DNS atual".into(), current));
    }
    let handles: Vec<_> = jobs
        .into_iter()
        .map(|(id, name, servers)| std::thread::spawn(move || measure(id, name, servers)))
        .collect();
    let mut out: Vec<DnsResult> = handles.into_iter().filter_map(|h| h.join().ok()).collect();
    out.sort_by(|a, b| match (a.latency_ms, b.latency_ms) {
        (Some(x), Some(y)) => x.total_cmp(&y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    Ok(out)
}

// ---------- adapters (registry) ----------

fn sz(hive_key: &str, name: &str) -> Option<String> {
    match registry::read(Hive::Hklm, hive_key, name) {
        Ok(Some(RegValue::Sz(s))) if !s.trim().is_empty() => Some(s),
        _ => None,
    }
}

fn has_gateway(key: &str) -> bool {
    ["DefaultGateway", "DhcpDefaultGateway"].iter().any(|n| match registry::read(Hive::Hklm, key, n) {
        Ok(Some(RegValue::MultiSz(v))) => v.iter().any(|g| !g.trim().is_empty() && g != "0.0.0.0"),
        Ok(Some(RegValue::Sz(s))) => !s.trim().is_empty(),
        _ => false,
    })
}

/// Interface GUIDs of adapters that are connected (have a default gateway).
fn active_interfaces() -> Vec<String> {
    registry::subkeys(Hive::Hklm, TCPIP).into_iter().filter(|g| has_gateway(&format!(r"{TCPIP}\{g}"))).collect()
}

pub fn split_servers(s: &str) -> Vec<String> {
    s.split([',', ' ']).map(str::trim).filter(|x| !x.is_empty()).map(String::from).collect()
}

fn current_servers() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for g in active_interfaces() {
        let key = format!(r"{TCPIP}\{g}");
        let s = sz(&key, "NameServer").or_else(|| sz(&key, "DhcpNameServer")).unwrap_or_default();
        for ip in split_servers(&s) {
            if !out.contains(&ip) {
                out.push(ip);
            }
        }
    }
    out
}

pub fn status() -> Result<DnsStatus> {
    let current = current_servers();
    let applied = journal::is_applied(TWEAK_ID)
        .then(|| PROVIDERS.iter().find(|p| current.first().is_some_and(|c| c == p.v4[0])).map(|p| p.name.to_string()))
        .flatten();
    Ok(DnsStatus { current, applied })
}

#[cfg(windows)]
fn flush_dns() {
    use std::os::windows::process::CommandExt;
    let _ = std::process::Command::new("ipconfig").arg("/flushdns").creation_flags(0x0800_0000).output();
}

#[cfg(not(windows))]
fn flush_dns() {}

pub fn apply(provider: &str) -> Result<DnsStatus> {
    if !cfg!(windows) {
        return Err(Error::Unsupported);
    }
    let p = PROVIDERS.iter().find(|p| p.id == provider).ok_or_else(|| Error::InvalidInput(format!("DNS desconhecido: {provider}")))?;
    let ifaces = active_interfaces();
    if ifaces.is_empty() {
        return Err(Error::Unavailable("nenhuma ligação de rede ativa encontrada".into()));
    }
    // Switching providers: go back to the original first so revert stays one step.
    if journal::is_applied(TWEAK_ID) {
        journal::revert(TWEAK_ID)?;
    }
    let v6 = registry::subkeys(Hive::Hklm, TCPIP6);
    let r: Result<()> = (|| {
        for g in &ifaces {
            registry::set_journaled(TWEAK_ID, Hive::Hklm, &format!(r"{TCPIP}\{g}"), "NameServer", &RegValue::Sz(p.v4.join(",")))?;
            if v6.iter().any(|x| x.eq_ignore_ascii_case(g)) {
                registry::set_journaled(TWEAK_ID, Hive::Hklm, &format!(r"{TCPIP6}\{g}"), "NameServer", &RegValue::Sz(p.v6.join(",")))?;
            }
        }
        Ok(())
    })();
    if let Err(e) = r {
        let _ = journal::revert(TWEAK_ID);
        return Err(e);
    }
    flush_dns();
    status()
}

pub fn revert() -> Result<DnsStatus> {
    journal::revert(TWEAK_ID)?;
    flush_dns();
    status()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_layout() {
        let q = build_query(0xABCD, "www.google.com");
        assert_eq!(&q[..4], &[0xAB, 0xCD, 0x01, 0x00]);
        assert_eq!(q[12], 3);
        assert_eq!(&q[13..16], b"www");
        assert_eq!(&q[q.len() - 5..], &[0, 0, 1, 0, 1]);
    }

    #[test]
    fn answer_check() {
        let mut r = build_query(7, "a.b");
        r[2] |= 0x80;
        assert!(is_answer(7, &r));
        assert!(!is_answer(8, &r));
        r[3] = 0x02; // SERVFAIL
        assert!(!is_answer(7, &r));
        assert!(!is_answer(7, &build_query(7, "a.b")));
    }

    /// Real queries to the public resolvers: `cargo test --lib dns -- --ignored`.
    #[test]
    #[ignore]
    fn live_test_answers() {
        let r = test().unwrap();
        assert!(r.iter().any(|x| x.latency_ms.is_some()), "{r:?}");
        for x in &r {
            println!("{:<12} {:?} ms ({:.0}%)", x.name, x.latency_ms, x.success * 100.0);
        }
    }

    #[test]
    fn median_and_split() {
        assert_eq!(median(&mut [3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(median(&mut [4.0, 1.0, 2.0, 3.0]), Some(2.5));
        assert_eq!(median(&mut []), None);
        assert_eq!(split_servers("1.1.1.1, 8.8.8.8 9.9.9.9"), ["1.1.1.1", "8.8.8.8", "9.9.9.9"]);
    }
}
