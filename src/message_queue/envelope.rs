//! Helpers for hiding generics via dynamic dispatch

use super::QueuePayload;
use crate::{actor::*, context::ActorContext};
use async_trait::async_trait;
use tokio::sync::oneshot;

/// A helper trait to hide generic message type behind a layer of dynamic dispatch
#[async_trait]
pub(crate) trait EnvelopeProxy<A: Actor> {
    /// Type-agnostic message handler for the envelope container, responsible for calling type-specific message handler
    async fn handle(&mut self, act: &mut A, ctx: &mut ActorContext<A>);
}

/// The generic envelope structure, used for wrapping queueed messages and their response-senders
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
    /// Used when the message response won't be handled
    pub fn new_no_sender(item: M) -> Self {
        Self {
            item: Some(item),
            tx: None,
        }
    }
    /// Wraps the message in a trait-object, abstracting away its' type
    pub fn pack<A>(self) -> QueuePayload<A>
    where
        A: Actor,
        Self: EnvelopeProxy<A>,
    {
        Box::from(self)
    }
}
