use super::messages::{PostMpnSolutionRequest, PostMpnSolutionResponse};
use super::{NodeContext, NodeError};
use crate::blockchain::Blockchain;
use crate::db::KvStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn post_mpn_solution<K: KvStore, B: Blockchain<K>>(
    context: Arc<RwLock<NodeContext<K, B>>>,
    req: PostMpnSolutionRequest,
) -> Result<PostMpnSolutionResponse, NodeError> {
    let mut ctx = context.write().await;
    if let Some(mpn_work_pool) = &mut ctx.mpn_work_pool {
        let mut accepted = 0;
        for (id, proof) in req.proofs.iter() {
            if mpn_work_pool.prove(*id, proof) {
                accepted += 1;
            }
        }
        println!("Got {} accepted SNARK proofs!", accepted);
        Ok(PostMpnSolutionResponse { accepted })
    } else {
        Ok(PostMpnSolutionResponse { accepted: 0 })
    }
}
