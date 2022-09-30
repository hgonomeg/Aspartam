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

/// Represents the current lifecycle state of the actor
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActorState {
    /// About to begin processing messages
    Starting,
    /// Normal operation
    Running,
    /// State indicating that the actor might stop, giving it a chance to react 
    Stopping,
    /// The actor is dead
    Stopped,
}

/// Value indicating whether a stopping actor should stop or continue running
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stopping {
    Continue,
    Stop,
}

/// Inner implementation of actor creation logic
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

/// The actor trait
#[async_trait]
pub trait Actor: 'static + Sized + Send {
    /// Starts the actor, consuming the structure and returning an address to the actor.
    fn start(self) -> Addr<Self> {
        let (msg_queue, msg_rx) = MessageQueue::new();
        let ret = Addr::<Self> {
            msg_queue: Arc::from(msg_queue),
        };
        let weakaddr = ret.downgrade();
        tokio::spawn(actor_runner_loop(self, ActorContext::new(weakaddr), msg_rx));
        ret
    }
    /// Uses the given closure to build and start the actor, returning its' address.
    /// 
    /// This method should be used in case when access to [ActorContext] is required at the time of initialization.
    fn create<F: FnOnce(&mut ActorContext<Self>) -> Self + Send>(f: F) -> Addr<Self> {
        let (actor, ret, ctx, msg_rx) = actor_create_impl(f);
        tokio::spawn(actor_runner_loop(actor, ctx, msg_rx));
        ret
    }
    /// Called when the actor is about to begin processing messages.
    async fn started(&mut self, _ctx: &mut ActorContext<Self>) {}
    /// Called when the actor is in stopping state.
    /// 
    /// It can be overriden to react to this condition and possibly go back to normal operation.
    async fn stopping(&mut self, _ctx: &mut ActorContext<Self>) -> Stopping {
        Stopping::Stop
    }
    /// Called when the actors stops.
    async fn stopped(&mut self, _ctx: &mut ActorContext<Self>) {}
}

/// Trait used to specify how an actor handles a given message type
#[async_trait]
pub trait Handler<T: Send>: Actor {
    /// Associated type expressing response type for a given type of incoming message
    type Response: Send + 'static;
    /// The method used to handle messages of a given type
    async fn handle(&mut self, msg: T, ctx: &mut ActorContext<Self>) -> Self::Response;
}
