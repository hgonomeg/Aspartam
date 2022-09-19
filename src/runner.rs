use crate::{
    actor::{Actor, ActorState, Stopping},
    addr::Addr,
    context::ActorContext,
    message_queue::{MessageQueue, QueuePayload},
    supervisor::Supervised,
};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

async fn stopping_check<A: Actor>(act: &mut A, ctx: &mut ActorContext<A>) {
    if ctx.state() == ActorState::Stopping {
        let new_state = match act.stopping(ctx).await {
            Stopping::Continue => ActorState::Running,
            Stopping::Stop => ActorState::Stopped,
        };
        ctx.set_state(new_state);
    }
}

pub(crate) async fn supervised_actor_runner_loop<A: Supervised>(
    mut act: A,
    mut ctx: ActorContext<A>,
    mut msg_rx: UnboundedReceiver<QueuePayload<A>>,
) {
    // starting phase
    assert_eq!(ctx.state(), ActorState::Starting);
    act.started(&mut ctx).await;
    if ctx.state() == ActorState::Starting {
        ctx.set_state(ActorState::Running);
    }
    stopping_check(&mut act, &mut ctx).await;
    if ctx.state() != ActorState::Stopped {
        loop {
            let mut _fresh_addr_opt = None;
            match msg_rx.recv().await {
                None => {
                    // We need to manually set the state to Stopping
                    ctx.set_state(ActorState::Stopping);
                    // At this point, calls to ctx.address() will panic
                    // which is due to the fact that receiving None means that
                    // we have no remaining Addresses referring to the actor.
                    //
                    // Thus we need to reset the context, in case if the actor
                    // wants to generate a new Addr in Actor::stopping()
                    let (new_msg_queue, new_rx) = MessageQueue::new();
                    _fresh_addr_opt = Some(Addr::<A> {
                        msg_queue: Arc::from(new_msg_queue),
                    });
                    ctx.reset_from(_fresh_addr_opt.as_ref().unwrap().downgrade());
                    msg_rx = new_rx;
                }
                Some(mut msg) => {
                    msg.handle(&mut act, &mut ctx).await;
                }
            }
            // Need to check if the state is Stopping
            //   which might be due to:
            //     1. the receiver yielding None
            //     2. calling ctx.stop() in Handler<M>::handle()
            stopping_check(&mut act, &mut ctx).await;
            if ctx.state() == ActorState::Stopped {
                break;
            }
        }
    }
    // final phase
    assert_eq!(ctx.state(), ActorState::Stopped);
    act.stopped(&mut ctx).await;
}

pub(crate) async fn actor_runner_loop<A: Actor>(
    mut act: A,
    mut ctx: ActorContext<A>,
    mut msg_rx: UnboundedReceiver<QueuePayload<A>>,
) {
    // starting phase
    assert_eq!(ctx.state(), ActorState::Starting);
    act.started(&mut ctx).await;
    if ctx.state() == ActorState::Starting {
        ctx.set_state(ActorState::Running);
    }
    stopping_check(&mut act, &mut ctx).await;
    if ctx.state() != ActorState::Stopped {
        loop {
            let mut _fresh_addr_opt = None;
            match msg_rx.recv().await {
                None => {
                    // We need to manually set the state to Stopping
                    ctx.set_state(ActorState::Stopping);
                    // At this point, calls to ctx.address() will panic
                    // which is due to the fact that receiving None means that
                    // we have no remaining Addresses referring to the actor.
                    //
                    // Thus we need to reset the context, in case if the actor
                    // wants to generate a new Addr in Actor::stopping()
                    let (new_msg_queue, new_rx) = MessageQueue::new();
                    _fresh_addr_opt = Some(Addr::<A> {
                        msg_queue: Arc::from(new_msg_queue),
                    });
                    ctx.reset_from(_fresh_addr_opt.as_ref().unwrap().downgrade());
                    msg_rx = new_rx;
                }
                Some(mut msg) => {
                    msg.handle(&mut act, &mut ctx).await;
                }
            }
            // Need to check if the state is Stopping
            //   which might be due to:
            //     1. the receiver yielding None
            //     2. calling ctx.stop() in Handler<M>::handle()
            stopping_check(&mut act, &mut ctx).await;
            if ctx.state() == ActorState::Stopped {
                break;
            }
        }
    }
    // final phase
    assert_eq!(ctx.state(), ActorState::Stopped);
    act.stopped(&mut ctx).await;
}
