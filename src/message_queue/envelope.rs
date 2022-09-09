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
        let ret = act.handle(self.item.take().unwrap(), ctx).await;
        if let Some(tx) = self.tx.take() {
            if let Err(_e) = tx.send(ret) {
                panic!("Failed to send response: oneshot::Receiver must be dead.");
            }
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
