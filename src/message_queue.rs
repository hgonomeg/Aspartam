use crate::actor::*;
use tokio::sync::{
    mpsc,
    oneshot,
};

mod envelope;
use envelope::*;

pub(crate) type QueuePayload<T> = Box<dyn EnvelopeProxy<T> + Send>;

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
    pub fn new() -> (Self, mpsc::UnboundedReceiver<QueuePayload<T>>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { tx }, rx)
    }
    pub fn send<M>(&self, msg: M) -> oneshot::Receiver<<T as Handler<M>>::Response>
    where
        T: Handler<M>,
        M: 'static + Send,
    {
        let (tx, rx) = oneshot::channel();
        let envelope = Envelope::new(msg, tx).pack();
        if let Err(_e) = self.tx.send(envelope) {
            panic!("Failed to enqueue message for actor. Receiver must be dead.");
        }
        rx
    }
    pub fn do_send<M>(&self, msg: M)
    where
        T: Handler<M>,
        M: 'static + Send,
    {
        let envelope = Envelope::new_no_sender(msg).pack();
        if let Err(_e) = self.tx.send(envelope) {
            panic!("Failed to enqueue message for actor. Receiver must be dead.");
        }
    }
}
