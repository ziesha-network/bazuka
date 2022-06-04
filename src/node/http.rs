use super::Peer;
use futures::future::join_all;

pub async fn group_request<F, R>(
    peers: &[Peer],
    f: F,
) -> Vec<(Peer, <R as futures::Future>::Output)>
where
    F: Fn(Peer) -> R,
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
