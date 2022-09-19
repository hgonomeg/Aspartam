use crate::{
    actor::{Actor, ActorState},
    context::ActorContext,
    message_queue::QueuePayload,
};
use tokio::sync::mpsc::UnboundedReceiver;

pub(crate) async fn actor_runner_loop<A: Actor>(
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