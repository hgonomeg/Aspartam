//! Execution context for actors

use crate::{
    actor::{Actor, ActorState, Handler},
    addr::{Addr, WeakAddr},
};
use futures_util::stream::{Stream, StreamExt};
/// Actor execution context 
/// 
/// It allows an actor to manage its' lifecycle, 
/// retrieve its' address and enqueue messages for later processing.
pub struct ActorContext<T: Actor> {
    address: WeakAddr<T>,
    state: ActorState,
}
unsafe impl<T: Actor> Send for ActorContext<T> {}

impl<T: Actor> ActorContext<T> {
    #[inline]
    /// Retrieves the current state of the actor
    pub fn state(&self) -> ActorState {
        self.state
    }
    /// Causes an actor to enter [ActorState::Stopping] state
    /// and then, as a result, potentially to stop.
    /// 
    /// It's adequate to call this function when the actor needs to stop, 
    /// for example, due to an error condition.
    pub fn stop(&mut self) {
        self.state = ActorState::Stopping
    }
    #[inline]
    /// Returns the actors' address.
    /// 
    /// It is meant to be used to pass the address to other actors.
    pub fn address(&self) -> Addr<T> {
        self.address.upgrade().unwrap()
    }
    #[inline]
    /// Queues a message to be later processed by the actor.
    /// 
    /// This function should be used if an actor needs to send a message to itself.
    /// 
    /// It behaves analogously to [Addr::do_send].
    pub fn notify<M>(&self, msg: M)
    where
        M: 'static + Send,
        T: Handler<M>,
    {
        self.address().do_send(msg)
    }
    #[inline]
    /// Returns [WeakAddr] of the actor.
    pub fn weak_address(&self) -> WeakAddr<T> {
        self.address.clone()
    }
    /// Forwards messages from the given [Stream] to the actor's message queue
    /// 
    /// The actor will not be dropped as long as the stream produces values
    pub fn add_stream<S, M>(&self, mut s: S)
    where
        S: 'static + Stream<Item = M> + Unpin + Send,
        M: 'static + Send,
        T: Handler<M>,
    {
        let addr = self.address.upgrade().unwrap();
        tokio::spawn(async move {
            while let Some(msg) = s.next().await {
                let _ = addr.send(msg).await;
            }
        });
    }
    /// Creates new [ActorContext] from the given [WeakAddr].
    /// 
    /// The initial state is [ActorState::Starting]
    /// 
    /// The context will not be valid if the [WeakAddr] refers to a dropped actor
    pub(crate) fn new(weakaddr: WeakAddr<T>) -> Self {
        Self {
            address: weakaddr,
            state: ActorState::Starting,
        }
    }
    /// Sets the state of the actor to the given value
    /// 
    /// For internal use.
    pub(crate) fn set_state(&mut self, state: ActorState) {
        self.state = state;
    }
    /// Replaces the internal [WeakAddr]
    /// 
    /// For internal use only.
    pub(crate) fn reset_from(&mut self, weakaddr: WeakAddr<T>) {
        self.address = weakaddr;
    }
}
