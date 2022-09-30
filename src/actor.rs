//! Basic definitions for actors

use crate::{
    addr::*,
    context::ActorContext,
    message_queue::{MessageQueue, QueuePayload},
    runner::*,
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActorState {
    Starting,
    Running,
    Stopping,
    Stopped,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stopping {
    Continue,
    Stop,
}

pub(crate) fn actor_create_impl<A: Actor, F: FnOnce(&mut ActorContext<A>) -> A + Send>(
    f: F,
) -> (
    A,
    Addr<A>,
    ActorContext<A>,
    UnboundedReceiver<QueuePayload<A>>,
) {
    let (msg_queue, msg_rx) = MessageQueue::new();
    let ret = Addr::<A> {
        msg_queue: Arc::from(msg_queue),
    };
    let weakaddr = ret.downgrade();
    let mut ctx = ActorContext::new(weakaddr);
    let actor = f(&mut ctx);
    (actor, ret, ctx, msg_rx)
}

#[async_trait]
pub trait Actor: 'static + Sized + Send {
    fn start(self) -> Addr<Self> {
        let (msg_queue, msg_rx) = MessageQueue::new();
        let ret = Addr::<Self> {
            msg_queue: Arc::from(msg_queue),
        };
        let weakaddr = ret.downgrade();
        tokio::spawn(actor_runner_loop(self, ActorContext::new(weakaddr), msg_rx));
        ret
    }
    fn create<F: FnOnce(&mut ActorContext<Self>) -> Self + Send>(f: F) -> Addr<Self> {
        let (actor, ret, ctx, msg_rx) = actor_create_impl(f);
        tokio::spawn(actor_runner_loop(actor, ctx, msg_rx));
        ret
    }
    async fn started(&mut self, _ctx: &mut ActorContext<Self>) {}
    async fn stopping(&mut self, _ctx: &mut ActorContext<Self>) -> Stopping {
        Stopping::Stop
    }
    async fn stopped(&mut self, _ctx: &mut ActorContext<Self>) {}
}

#[async_trait]
pub trait Handler<T: Send>: Actor {
    type Response: Send + 'static;
    async fn handle(&mut self, msg: T, ctx: &mut ActorContext<Self>) -> Self::Response;
}
