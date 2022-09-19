use crate::{
    addr::*,
    context::ActorContext,
    message_queue::{MessageQueue},
    runner::*,
};
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Clone,Copy,Debug,Eq,PartialEq)]
pub enum ActorState {
    Starting,
    Running,
    Stopping,
    Stopped
}

#[derive(Clone,Copy,Debug,Eq,PartialEq)]
pub enum Stopping {
    Continue,
    Stop
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
    async fn stopping(&mut self, _ctx: &mut ActorContext<Self>) -> Stopping { Stopping::Stop } 
    async fn stopped(&mut self, _ctx: &mut ActorContext<Self>) {}
}



#[async_trait]
pub trait Handler<T: Send>: Actor {
    type Response: Send + 'static;
    async fn handle(&mut self, msg: T, ctx: &mut ActorContext<Self>) -> Self::Response;
}
