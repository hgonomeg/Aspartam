//! Utilities for creating supervised actors    

use crate::{
    actor::{actor_create_impl, Actor},
    addr::Addr,
    context::ActorContext,
    runner::*,
};
use async_trait::async_trait;

/// Helper type used to create [Supervised] actors
pub struct Supervisor<A: Actor> {
    _phantom: std::marker::PhantomData<A>,
}

impl<A> Supervisor<A>
where
    A: Supervised,
{
    /// Uses the given closure to start a [Supervised] actor
    pub fn start<F: FnOnce(&mut ActorContext<A>) -> A + Send>(f: F) -> Addr<A> {
        let (actor, ret, ctx, msg_rx) = actor_create_impl(f);
        tokio::spawn(supervised_actor_runner_loop(actor, ctx, msg_rx));
        ret
    }
}

#[async_trait]
/// Special trait allowing actors to restart after failure,
/// i.e. to restart after the actor stops but still has valid addresses pointing to it
pub trait Supervised: Actor {
    /// Called after the actor has stopped and is about to begin its' lifecycle again.
    async fn restarting(&mut self, _ctx: &mut ActorContext<Self>) {}
}
