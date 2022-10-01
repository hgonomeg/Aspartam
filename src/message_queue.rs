//! Internal message queue implementation

use crate::{actor::*, error::*};
use tokio::sync::{mpsc, oneshot};

mod envelope;
use envelope::*;

/// The type used for wrapping enqueued messages
pub(crate) type QueuePayload<T> = Box<dyn EnvelopeProxy<T> + Send>;

/// Message queue wraps a sender for [QueuePayload]
pub(crate) struct MessageQueue<T: Actor> {
    tx: mpsc::UnboundedSender<QueuePayload<T>>,
}

impl<T: Actor> Clone for MessageQueue<T> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl<T: Actor> MessageQueue<T> {
    /// New message queue with its' corresponding receiver
    pub fn new() -> (Self, mpsc::UnboundedReceiver<QueuePayload<T>>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { tx }, rx)
    }
    pub fn send<M>(
        &self,
        msg: M,
    ) -> Result<oneshot::Receiver<<T as Handler<M>>::Response>, ActorError>
    where
        T: Handler<M>,
        M: 'static + Send,
    {
        let (tx, rx) = oneshot::channel();
        let envelope = Envelope::new(msg, tx).pack();
        self.tx.send(envelope)?;
        Ok(rx)
    }
    pub fn try_send<M>(&self, msg: M) -> Result<(), ActorError>
    where
        T: Handler<M>,
        M: 'static + Send,
    {
        let envelope = Envelope::new_no_sender(msg).pack();
        Ok(self.tx.send(envelope)?)
    }
    pub fn do_send<M>(&self, msg: M)
    where
        T: Handler<M>,
        M: 'static + Send,
    {
        let envelope = Envelope::new_no_sender(msg).pack();
        // do send just ignores errors
        let _ = self.tx.send(envelope);
    }
}
