//! Helpers for hiding generics via dynamic dispatch

use super::QueuePayload;
use crate::{actor::*, context::ActorContext};
use async_trait::async_trait;
use tokio::sync::oneshot;

#[async_trait]
pub(crate) trait EnvelopeProxy<A: Actor> {
    async fn handle(&mut self, act: &mut A, ctx: &mut ActorContext<A>);
}
pub(crate) struct Envelope<M: Send, R: Send> {
    item: Option<M>,
    tx: Option<oneshot::Sender<R>>,
}

#[async_trait]
impl<A, M> EnvelopeProxy<A> for Envelope<M, <A as Handler<M>>::Response>
where
    A: Actor,
    A: Handler<M>,
    M: Send,
{
    async fn handle(&mut self, act: &mut A, ctx: &mut ActorContext<A>) {
        let item = self.item.take().unwrap();
        if let Some(tx) = self.tx.take() {
            // If the sender got closed, the future created by Addr::send() got dropped.
            // No need to process the message.
            if ! tx.is_closed() {
                let ret = act.handle(item, ctx).await;
                // We shouldn't panic when this fails:
                let _ = tx.send(ret);
                // This might happen when the future created by Addr::send() gets dropped right after the message got handled
            }
        } else {
            // handles Addr::do_send() messages
            let _ = act.handle(item, ctx).await;
        }
    }
}

impl<M: 'static + Send, R: 'static + Send> Envelope<M, R> {
    pub fn new(item: M, tx: oneshot::Sender<R>) -> Self {
        Self {
            item: Some(item),
            tx: Some(tx),
        }
    }
    pub fn new_no_sender(item: M) -> Self {
        Self {
            item: Some(item),
            tx: None,
        }
    }
    pub fn pack<A>(self) -> QueuePayload<A>
    where
        A: Actor,
        Self: EnvelopeProxy<A>,
    {
        Box::from(self)
    }
}
