use crate::{
    actor::{Actor, ActorState, Handler},
    addr::{Addr, WeakAddr},
};
use futures_util::stream::{Stream, StreamExt};

pub struct ActorContext<T: Actor> {
    address: WeakAddr<T>,
    state: ActorState,
}
unsafe impl<T: Actor> Send for ActorContext<T> {}

impl<T: Actor> ActorContext<T> {
    #[inline]
    pub fn state(&self) -> ActorState {
        self.state
    }
    pub fn stop(&mut self) {
        self.state = ActorState::Stopping
    }
    #[inline]
    pub fn address(&self) -> Addr<T> {
        self.address.upgrade().unwrap()
    }
    #[inline]
    pub fn notify<M>(&self, msg: M)
    where
        M: 'static + Send,
        T: Handler<M>,
    {
        self.address().do_send(msg)
    }
    #[inline]
    pub fn weak_address(&self) -> WeakAddr<T> {
        self.address.clone()
    }
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
    pub(crate) fn new(weakaddr: WeakAddr<T>) -> Self {
        Self {
            address: weakaddr,
            state: ActorState::Starting,
        }
    }
    pub(crate) fn set_state(&mut self, state: ActorState) {
        self.state = state;
    }
    pub(crate) fn reset_from(&mut self, weakaddr: WeakAddr<T>) {
        self.address = weakaddr;
    }
}
