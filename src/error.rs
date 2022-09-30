//! Stores aspartam's error type

use crate::{actor::Actor, message_queue::QueuePayload};
use thiserror::Error;
use tokio::sync::mpsc::error::SendError as TokioSendError;
use tokio::sync::oneshot::error::RecvError as TokioRecvError;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ActorError {
    #[error("Failed to enqueue new message for actor. Actor has most likely stopped.")]
    CannotSend,
    #[error("The actor has most likely stopped before the message could be handled.")]
    MessageLost,
}

impl<T> From<TokioSendError<QueuePayload<T>>> for ActorError
where
    T: Actor,
{
    fn from(_: TokioSendError<QueuePayload<T>>) -> Self {
        Self::CannotSend
    }
}

impl From<TokioRecvError> for ActorError {
    fn from(_: TokioRecvError) -> Self {
        Self::MessageLost
    }
}
