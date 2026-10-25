//! OS-assigned loopback ports must also be usable by an HTTP browser.
use std::{future::Future, io, net::Ipv4Addr};
use tokio::net::TcpListener;

// Port numbers from https://fetch.spec.whatwg.org/#port-blocking (2026-10-25).
// Keep the browser's protection enabled; choose another local port instead.
const BLOCKED_PORTS: &[u16] = &[
    0, 1, 7, 9, 11, 13, 15, 17, 19, 20, 21, 22, 23, 25, 37, 42, 43, 53, 69, 77, 79, 87, 95, 101,
    102, 103, 104, 109, 110, 111, 113, 115, 117, 119, 123, 135, 137, 139, 143, 161, 179, 389, 427,
    465, 512, 513, 514, 515, 526, 530, 531, 532, 540, 548, 554, 556, 563, 587, 601, 636, 989, 990,
    993, 995, 1719, 1720, 1723, 2049, 3659, 4045, 4190, 5060, 5061, 6000, 6566, 6665, 6666, 6667,
    6668, 6669, 6679, 6697, 10080,
];

pub(super) async fn bind() -> io::Result<TcpListener> {
    bind_with(|| TcpListener::bind((Ipv4Addr::LOCALHOST, 0))).await
}

async fn bind_with<F, B>(mut bind_next: F) -> io::Result<TcpListener>
where
    F: FnMut() -> B,
    B: Future<Output = io::Result<TcpListener>>,
{
    // Retain rejected allocations until selection finishes, so the OS cannot
    // immediately hand the same blocked port back to the next bind(0).
    let mut rejected = Vec::new();
    for _ in 0..=BLOCKED_PORTS.len() {
        let listener = bind_next().await?;
        if !BLOCKED_PORTS.contains(&listener.local_addr()?.port()) {
            return Ok(listener);
        }
        rejected.push(listener);
    }
    Err(io::Error::new(
        io::ErrorKind::AddrNotAvailable,
        "브라우저로 접속할 수 있는 로컬 포트를 찾지 못했습니다",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn blocked_allocation_is_retried_and_released_without_disabling_browser_protection() {
        let mut first = None;
        for port in [6000, 6667, 10080, 6566] {
            if let Ok(listener) = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await {
                first = Some(listener);
                break;
            }
        }
        let rejected_port = first
            .as_ref()
            .expect("one test port must be free")
            .local_addr()
            .unwrap()
            .port();
        let mut attempts = 0;
        let selected = bind_with(|| {
            attempts += 1;
            let listener = first.take();
            async move {
                match listener {
                    Some(listener) => Ok(listener),
                    None => TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await,
                }
            }
        })
        .await
        .unwrap();
        assert!(attempts >= 2);
        let address = selected.local_addr().unwrap();
        assert!(address.ip().is_loopback());
        assert!(!BLOCKED_PORTS.contains(&address.port()));
        tokio::net::TcpStream::connect(address).await.unwrap();
        TcpListener::bind((Ipv4Addr::LOCALHOST, rejected_port))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn allocation_failure_is_returned_without_an_unbounded_retry() {
        let mut attempts = 0;
        let result = bind_with(|| {
            attempts += 1;
            async {
                Err(io::Error::new(
                    io::ErrorKind::AddrNotAvailable,
                    "synthetic allocation failure",
                ))
            }
        })
        .await;
        assert_eq!(attempts, 1);
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::AddrNotAvailable);
    }
}
