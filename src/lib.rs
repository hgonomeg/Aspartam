use async_trait::async_trait;
use futures_util::stream::{Stream, StreamExt};
use std::sync::{Arc, Weak};
use tokio::sync::{
    mpsc::{self, UnboundedReceiver},
    oneshot,
};
pub struct ActorContext<T: Actor> {
    address: WeakAddr<T>,
}
unsafe impl<T: Actor> Send for ActorContext<T> {}

impl<T: Actor> ActorContext<T> {
    pub fn address(&self) -> Addr<T> {
        self.address.upgrade().unwrap()
    }
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
    fn new(weakaddr: WeakAddr<T>) -> Self {
        Self { address: weakaddr }
    }
}
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

type QueuePayload<T> = Box<dyn EnvelopeProxy<T> + Send>;

struct MessageQueue<T: Actor> {
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
    fn do_send<M>(&self, msg: M)
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

pub struct Addr<T: Actor> {
    msg_queue: Arc<MessageQueue<T>>,
}
impl<T: Actor> Clone for Addr<T> {
    fn clone(&self) -> Self {
        Self {
            msg_queue: self.msg_queue.clone(),
        }
    }
}
unsafe impl<T: Actor> Send for Addr<T> {}

impl<T: Actor> Addr<T> {
    pub async fn send<M>(&self, msg: M) -> <T as Handler<M>>::Response
    where
        M: 'static + Send,
        T: Handler<M>,
    {
        let resp = self.msg_queue.send(msg);
        resp.await.unwrap()
    }
    pub fn do_send<M>(&self, msg: M)
    where
        M: 'static + Send,
        T: Handler<M>,
    {
        self.msg_queue.do_send(msg)
    }
    pub fn downgrade(&self) -> WeakAddr<T> {
        WeakAddr::<T> {
            msg_queue: Arc::downgrade(&self.msg_queue),
        }
    }
}

pub struct WeakAddr<T: Actor> {
    msg_queue: Weak<MessageQueue<T>>,
}
impl<T: Actor> Clone for WeakAddr<T> {
    fn clone(&self) -> Self {
        Self {
            msg_queue: Weak::clone(&self.msg_queue),
        }
    }
}
unsafe impl<T: Actor> Send for WeakAddr<T> {}

impl<T: Actor> WeakAddr<T> {
    pub fn upgrade(&self) -> Option<Addr<T>> {
        Some(Addr::<T> {
            msg_queue: self.msg_queue.upgrade()?,
        })
    }
}

#[async_trait]
pub trait Actor: 'static + Sized + Send {
    fn start(self) -> Addr<Self> {
        let (msg_queue, msg_rx) = MessageQueue::new();
        let ret = Addr::<Self> {
            msg_queue: Arc::from(msg_queue),
        };
        let weakaddr = ret.downgrade();
        tokio::spawn(actor_runner_loop(self, ActorContext::new(weakaddr), msg_rx));
        ret
    }
    fn create<F: Fn(&mut ActorContext<Self>) -> Self + Send>(f: F) -> Addr<Self> {
        let (msg_queue, msg_rx) = MessageQueue::new();
        let ret = Addr::<Self> {
            msg_queue: Arc::from(msg_queue),
        };
        let weakaddr = ret.downgrade();
        let mut ctx = ActorContext::new(weakaddr);
        tokio::spawn(actor_runner_loop(f(&mut ctx), ctx, msg_rx));
        ret
    }
    async fn started(&mut self, _ctx: &mut ActorContext<Self>) {}
    async fn stopped(&mut self, _ctx: &mut ActorContext<Self>) {}
}

async fn actor_runner_loop<A: Actor>(
    mut act: A,
    mut ctx: ActorContext<A>,
    mut msg_rx: UnboundedReceiver<QueuePayload<A>>,
) {
    act.started(&mut ctx).await;
    while let Some(mut msg) = msg_rx.recv().await {
        msg.handle(&mut act, &mut ctx).await;
    }
    act.stopped(&mut ctx).await;
}

#[async_trait]
pub trait Handler<T: Send>: Actor {
    type Response: Send + 'static;
    async fn handle(&mut self, msg: T, ctx: &mut ActorContext<Self>) -> Self::Response;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Runtime::new().unwrap()
    }

    #[test]
    fn basic_messages() {
        struct Ping;
        struct Pong;

        struct Game;

        impl Actor for Game {}
        #[async_trait]
        impl Handler<Ping> for Game {
            type Response = Pong;
            async fn handle(
                &mut self,
                _msg: Ping,
                _ctx: &mut ActorContext<Self>,
            ) -> Self::Response {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                Pong
            }
        }

        get_runtime().block_on(async {
            let game = Game.start();
            let _pong = game.send(Ping).await;
        });
    }

    #[test]
    fn data_sanity() {
        struct Incrementor {
            request_count: usize,
        }
        impl Actor for Incrementor {}
        #[async_trait]
        impl Handler<u32> for Incrementor {
            type Response = u32;
            async fn handle(&mut self, msg: u32, _ctx: &mut ActorContext<Self>) -> Self::Response {
                self.request_count += 1;
                msg + 1
            }
        }
        struct GetRequestCount;
        #[async_trait]
        impl Handler<GetRequestCount> for Incrementor {
            type Response = usize;
            async fn handle(
                &mut self,
                _msg: GetRequestCount,
                _ctx: &mut ActorContext<Self>,
            ) -> Self::Response {
                self.request_count
            }
        }

        get_runtime().block_on(async {
            let incrementor = Incrementor { request_count: 0 }.start();
            assert_eq!(incrementor.send(GetRequestCount).await, 0);
            assert_eq!(incrementor.send(2).await, 3);
            assert_eq!(incrementor.send(GetRequestCount).await, 1);
            assert_eq!(incrementor.send(7).await, 8);
            assert_eq!(incrementor.send(9).await, 10);
            assert_eq!(incrementor.send(GetRequestCount).await, 3);
            let mut i = 0;
            while i < 5000 {
                let r = incrementor.send(i).await;
                i += 1;
                assert_eq!(r, i);
            }
            assert_eq!(incrementor.send(GetRequestCount).await, 5003);
        });
    }
    #[test]
    fn memory_leaks() {
        struct DropMe {
            tx: Option<oneshot::Sender<()>>,
        }
        impl Actor for DropMe {}
        impl Drop for DropMe {
            fn drop(&mut self) {
                self.tx.take().unwrap().send(()).unwrap();
            }
        }
        get_runtime().block_on(async {
            let (tx, rx) = oneshot::channel();
            let d = DropMe { tx: Some(tx) }.start();
            drop(d);
            rx.await.unwrap();
        });
    }
    #[test]
    fn actor_create() {
        struct Secondary {
            _prim: WeakAddr<Primary>,
        }
        impl Actor for Secondary {}
        struct Primary {
            _sec: Addr<Secondary>,
        }
        impl Actor for Primary {}

        get_runtime().block_on(async {
            let _prim = Primary::create(move |a| {
                let this = a.address();
                Primary {
                    _sec: Secondary {
                        _prim: this.downgrade(),
                    }
                    .start(),
                }
            });
        })
    }
    #[test]
    fn handling_streams() {
        struct Sh {
            tx: Option<oneshot::Sender<usize>>,
            message_count: usize,
        }
        impl Actor for Sh {}
        #[derive(Clone)]
        struct Ping;
        struct As<S> {
            stream: S,
        }
        unsafe impl<S> Send for As<S> {}
        #[async_trait]
        impl Handler<Ping> for Sh {
            type Response = ();
            async fn handle(
                &mut self,
                _msg: Ping,
                _ctx: &mut ActorContext<Self>,
            ) -> Self::Response {
                self.message_count += 1;
            }
        }
        #[async_trait]
        impl<S> Handler<As<S>> for Sh
        where
            S: 'static + Stream<Item = Ping> + Unpin + Send,
        {
            type Response = ();
            async fn handle(&mut self, msg: As<S>, ctx: &mut ActorContext<Self>) -> Self::Response {
                ctx.add_stream(msg.stream);
            }
        }
        impl Drop for Sh {
            fn drop(&mut self) {
                self.tx.take().unwrap().send(self.message_count).unwrap();
            }
        }
        get_runtime().block_on(async {
            let (tx, rx) = oneshot::channel();
            let d = Sh {
                tx: Some(tx),
                message_count: 0,
            }
            .start();
            let stream = futures_util::stream::iter(std::iter::repeat(Ping).take(1000));
            d.do_send(As { stream });

            let stream2 = futures_util::stream::iter(std::iter::repeat(Ping).take(5000));
            d.do_send(As { stream: stream2 });

            let stream3 = futures_util::stream::iter(std::iter::repeat(Ping).take(10000));
            d.do_send(As { stream: stream3 });

            d.do_send(Ping);
            d.do_send(Ping);
            d.do_send(Ping);
            d.do_send(Ping);
            d.do_send(Ping);

            drop(d);
            assert_eq!(rx.await.unwrap(), 16005);
        });
    }
}
