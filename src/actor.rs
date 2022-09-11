use crate::{
    addr::*,
    context::ActorContext,
    message_queue::{MessageQueue, QueuePayload},
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

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
    fn create<F: FnOnce(&mut ActorContext<Self>) -> Self + Send>(f: F) -> Addr<Self> {
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
