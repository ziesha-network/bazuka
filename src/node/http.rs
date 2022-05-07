use super::PeerAddress;
use futures::future::join_all;

pub async fn group_request<F, R>(
    peers: &[PeerAddress],
    f: F,
) -> Vec<(PeerAddress, <R as futures::Future>::Output)>
where
    F: Fn(PeerAddress) -> R,
    R: futures::Future,
{
    peers
        .iter()
        .cloned()
        .zip(
            join_all(peers.iter().cloned().map(f).collect::<Vec<_>>())
                .await
                .into_iter(),
        )
        .collect()
}
