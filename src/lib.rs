use async_trait::async_trait;
use std::sync::{Arc, Weak};
use tokio::{
    sync::Mutex,
    sync::{mpsc, oneshot},
};
pub struct ActorContext<T: Actor> {
    address: WeakAddr<T>,
    msg_queue: MessageQueue<T>,
}
unsafe impl<T: Actor> Send for ActorContext<T> {}
#[async_trait]
trait EnvelopeProxy<A: Actor> {
    async fn handle(&mut self, act: &mut A, ctx: &mut ActorContext<A>);
}
struct Envelope<M: Send, R: Send> {
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
        let tx = self.tx.take().unwrap();
        if let Err(_e) = tx.send(ret) {
            panic!("Failed to send response: oneshot::Receiver must be dead.");
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
    pub fn pack<A>(self) -> QueuePayload<A>
    where
        A: Actor,
        Self: EnvelopeProxy<A>,
    {
        Box::from(self)
    }
}

type QueuePayload<T> = Box<dyn EnvelopeProxy<T> + Send>;
struct MessageQueue<T: Actor> {
    tx: mpsc::UnboundedSender<QueuePayload<T>>,
}

impl<T: Actor> MessageQueue<T> {
    fn new() -> (Self, mpsc::UnboundedReceiver<QueuePayload<T>>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { tx }, rx)
    }
    fn send<M>(&self, msg: M) -> oneshot::Receiver<<T as Handler<M>>::Response>
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
}

impl<T: Actor> ActorContext<T> {
    pub fn address(&self) -> Addr<T> {
        self.address.upgrade().unwrap()
    }
    fn empty(msg_queue: MessageQueue<T>) -> Self {
        Self {
            address: WeakAddr::<T>::empty(),
            msg_queue,
        }
    }
    fn set_weakaddr(&mut self, source: WeakAddr<T>) {
        self.address = source;
    }
}

#[derive(Clone)]
pub struct Addr<T: Actor> {
    _inner: Arc<Mutex<T>>,
    ctx: Arc<Mutex<ActorContext<T>>>,
}
unsafe impl<T: Actor> Send for Addr<T> {}

#[derive(Clone)]
pub struct WeakAddr<T: Actor> {
    _inner: Weak<Mutex<T>>,
    ctx: Weak<Mutex<ActorContext<T>>>,
}
unsafe impl<T: Actor> Send for WeakAddr<T> {}

impl<T: Actor> WeakAddr<T> {
    pub fn upgrade(&self) -> Option<Addr<T>> {
        Some(Addr::<T> {
            _inner: self._inner.upgrade()?,
            ctx: self.ctx.upgrade()?,
        })
    }
    fn empty() -> Self {
        Self {
            _inner: Weak::new(),
            ctx: Weak::new(),
        }
    }
}

impl<T: Actor> Addr<T> {
    pub async fn send<M>(&self, msg: M) -> <T as Handler<M>>::Response
    where
        M: 'static + Send,
        T: Handler<M>,
    {
        let ctx_lock = self.ctx.lock().await;
        let resp = ctx_lock.msg_queue.send(msg);
        drop(ctx_lock);
        resp.await.unwrap()
    }
    pub fn downgrade(&self) -> WeakAddr<T> {
        WeakAddr::<T> {
            _inner: Arc::<Mutex<T>>::downgrade(&self._inner),
            ctx: Arc::<Mutex<ActorContext<T>>>::downgrade(&self.ctx),
        }
    }
}

#[async_trait]
pub trait Actor: 'static + Sized + Send {
    async fn start(self) -> Addr<Self> {
        let (msg_queue, mut msg_rx) = MessageQueue::new();

        let ret = Addr::<Self> {
            _inner: Arc::from(Mutex::from(self)),
            ctx: Arc::from(Mutex::from(ActorContext::empty(msg_queue))),
        };
        let mut ctx_lock = ret.ctx.lock().await;
        ctx_lock.set_weakaddr(ret.downgrade());
        drop(ctx_lock);
        let weakaddr = ret.downgrade();

        tokio::spawn(async move {
            while let Some(mut msg) = msg_rx.recv().await {
                let owned = weakaddr.upgrade().unwrap();
                let mut act = owned._inner.lock().await;
                let mut ctx = owned.ctx.lock().await;
                msg.handle(&mut *act, &mut *ctx).await;
                drop(act);
                drop(ctx);
            }
        });
        ret
    }
    async fn started(&self, _ctx: &mut ActorContext<Self>) {}
}

#[async_trait]
pub trait Handler<T: Send>: Actor {
    type Response: Send + 'static;
    async fn handle(&mut self, msg: T, ctx: &mut ActorContext<Self>) -> Self::Response;
}

#[cfg(test)]
mod tests {
    use super::*;

    

    #[test]
    fn basic() {

    }
}
