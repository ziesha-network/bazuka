use futures::future::join_all;

pub async fn group_request<F, R, P>(peers: &[P], f: F) -> Vec<(P, <R as futures::Future>::Output)>
where
    F: Fn(&P) -> R,
    R: futures::Future,
    P: Clone,
{
    peers
        .iter()
        .cloned()
        .zip(
            join_all(peers.iter().map(f).collect::<Vec<_>>())
                .await
                .into_iter(),
        )
        .collect()
}
