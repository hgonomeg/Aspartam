//! Utilities for creating supervised actors    

use crate::{
    actor::{actor_create_impl, Actor},
    addr::Addr,
    context::ActorContext,
    runner::*,
};
use async_trait::async_trait;


#[async_trait]
/// Special trait allowing actors to restart after failure,
/// i.e. to restart after the actor stops but still has valid addresses pointing to it
pub trait Supervised: Actor {
    /// Called after the actor has stopped and is about to begin its' lifecycle again.
    async fn restarting(&mut self, _ctx: &mut ActorContext<Self>) {}

    /// Uses the given closure to start a [Supervised] actor
    fn create_supervised<F: FnOnce(&mut ActorContext<Self>) -> Self + Send>(f: F) -> Addr<Self> {
        let (actor, ret, ctx, msg_rx) = actor_create_impl(f);
        tokio::spawn(supervised_actor_runner_loop(actor, ctx, msg_rx));
        ret
    }
}
