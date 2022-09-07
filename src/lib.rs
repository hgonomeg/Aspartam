use async_trait::async_trait;
use std::{sync::{Arc, Weak}, marker::PhantomData};
use tokio::{sync::Mutex,sync::mpsc};
use futures_util::{stream,sink};
pub struct ActorContext<T: Actor> {
    address: WeakAddr<T>,
    msg_queue: MessageQueue<T>
}

trait EnvelopeProxy<A: Actor>{

}
struct Envelope<T>(Box<dyn EnvelopeProxy<T> + Send>);


impl<T> Envelope<T> {
    // fn new() -> Self {
        
    // }
}

struct MessageQueue<T: Actor> {
    tx: mpsc::UnboundedSender<Envelope<T>>
}

impl<T: Actor> MessageQueue<T> {
    fn new() -> (Self,mpsc::UnboundedReceiver<Envelope<T>>) {
        let (tx,rx) = mpsc::unbounded_channel();
        (Self {
            tx
        },rx)
    }
    async fn send<M>(&self, msg: M) where T: Handler<M>, M: Send {

    }
}

impl<T: Actor> ActorContext<T> {
    pub fn address(&self) -> Addr<T> {
        self.address.upgrade().unwrap()
    }
    fn empty(msg_queue: MessageQueue<T>) -> Self {
        Self {
            address: WeakAddr::<T>::empty(),
            msg_queue
        }
    }
    fn set_weakaddr(&mut self, source: WeakAddr<T>) {
        self.address = source;
    }
}

#[derive(Clone)]
pub struct Addr<T: Actor> {
    _inner: Arc<Mutex<T>>,
    ctx: Arc<Mutex<ActorContext<T>>>
}

#[derive(Clone)]
pub struct WeakAddr<T: Actor> {
    _inner: Weak<Mutex<T>>,
    ctx: Weak<Mutex<ActorContext<T>>>
}

impl<T: Actor> WeakAddr<T> {
    pub fn upgrade(&self) -> Option<Addr<T>> {
        Some(
            Addr::<T> {
                _inner: self._inner.upgrade()?,
                ctx: self.ctx.upgrade()?
            }
        )
    }
    fn empty() -> Self {
        Self {
            _inner: Weak::new(),
            ctx: Weak::new()
        }
    }
}

impl<T: Actor> Addr<T> {
    pub async fn send<M>(&self, msg: M) where M: Send, T: Handler<M> {
       let ctx_lock = self.ctx.lock().await; 
       ctx_lock.msg_queue.send(msg).await;
    }
    pub fn downgrade(&self) -> WeakAddr<T> {
        WeakAddr::<T> {
            _inner: Arc::<Mutex<T>>::downgrade(&self._inner),
            ctx: Arc::<Mutex<ActorContext<T>>>::downgrade(&self.ctx)
        }
    }
}

#[async_trait]
pub trait Actor: 'static + Sized {
    async fn start(self) -> Addr<Self> {
        let (msg_queue,mut msg_rx) = MessageQueue::new();
        
        tokio::spawn(async move { 
            while let Some(msg) = msg_rx.recv().await {

            }
        });

        let ret = Addr::<Self> { 
            _inner: Arc::from(Mutex::from(self)),
            ctx: Arc::from(Mutex::from(ActorContext::empty(msg_queue)))
        };
        let mut ctx_lock = ret.ctx.lock().await;
        ctx_lock.set_weakaddr(ret.downgrade());
        drop(ctx_lock);
        ret
    }
    async fn started();
}

#[async_trait]
pub trait Handler<T: Send>: Actor {
    async fn handle(&mut self, msg: T, ctx: &mut ActorContext<Self>);
}

#[cfg(test)]
mod tests {
    use super::*;

}
