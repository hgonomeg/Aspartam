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

struct FinishedActor<A: Actor> {
    actor: A,
    ctx: ActorContext<A>,
    died_from_dropping_last_reference: bool,
    msg_rx: UnboundedReceiver<QueuePayload<A>>,
}

async fn actor_runner_loop_impl<A: Actor>(
    mut act: A,
    mut ctx: ActorContext<A>,
    mut msg_rx: UnboundedReceiver<QueuePayload<A>>,
) -> FinishedActor<A> {
    // starting phase
    assert_eq!(ctx.state(), ActorState::Starting);
    act.started(&mut ctx).await;
    if ctx.state() == ActorState::Starting {
        ctx.set_state(ActorState::Running);
    }
    stopping_check(&mut act, &mut ctx).await;
    let mut died_from_dropping_last_reference = false;
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
                    died_from_dropping_last_reference = true;
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
                // just exit, leaving everything as is
                break;
            } else {
                // the actor might have just recovered from the Stopping state.
                // We need to clear the boolean flag.
                died_from_dropping_last_reference = false;
            }
        }
    }
    // final phase
    assert_eq!(ctx.state(), ActorState::Stopped);
    act.stopped(&mut ctx).await;
    FinishedActor {
        actor: act,
        ctx,
        died_from_dropping_last_reference,
        msg_rx,
    }
}

/// Should be very similar to [actor_runner_loop] except that the actor gets restarted when it's Stopped.
///
/// The actor might actually die when all references to it are dropped.
pub(crate) async fn supervised_actor_runner_loop<A: Supervised>(
    mut act: A,
    mut ctx: ActorContext<A>,
    mut msg_rx: UnboundedReceiver<QueuePayload<A>>,
) {
    loop {
        let finished_actor = actor_runner_loop_impl(act, ctx, msg_rx).await;
        if finished_actor.died_from_dropping_last_reference {
            break;
        } else {
            act = finished_actor.actor;
            ctx = finished_actor.ctx;
            msg_rx = finished_actor.msg_rx;
            act.restarting(&mut ctx).await;
            ctx.set_state(ActorState::Starting);
        }
    }
}

pub(crate) async fn actor_runner_loop<A: Actor>(
    act: A,
    ctx: ActorContext<A>,
    msg_rx: UnboundedReceiver<QueuePayload<A>>,
) {
    let _ = actor_runner_loop_impl(act, ctx, msg_rx).await;
}
