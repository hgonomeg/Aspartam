//! Utilities for creating supervised actors    

use crate::{
    actor::{actor_create_impl, Actor},
    addr::Addr,
    context::ActorContext,
    runner::*,
};
use async_trait::async_trait;

pub struct Supervisor<A: Actor> {
    _phantom: std::marker::PhantomData<A>,
}

impl<A> Supervisor<A>
where
    A: Supervised,
{
    pub fn start<F: FnOnce(&mut ActorContext<A>) -> A + Send>(f: F) -> Addr<A> {
        let (actor, ret, ctx, msg_rx) = actor_create_impl(f);
        tokio::spawn(supervised_actor_runner_loop(actor, ctx, msg_rx));
        ret
    }
}

#[async_trait]
pub trait Supervised: Actor {
    async fn restarting(&mut self, _ctx: &mut ActorContext<Self>) {}
}
