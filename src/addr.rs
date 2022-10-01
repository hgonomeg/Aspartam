//! Actor addresses

use crate::{
    actor::{Actor, Handler},
    error::*,
    message_queue::MessageQueue,
};
use std::sync::{Arc, Weak};

/// Address of an actor
/// 
/// Addresses are the objects through which you can interact with actors
/// 
/// Internally uses reference counting
pub struct Addr<T: Actor> {
    pub(crate) msg_queue: Arc<MessageQueue<T>>,
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
    /// Sends a message to the actor
    /// and asynchronously waits for its' response.
    /// 
    /// This function will fail if the actor is unable to process the message.
    /// 
    /// This function should not be used by actor to send messages to themselves, as it will result in a deadlock.
    /// [crate::context::ActorContext::notify] should be used for that purpose.
    pub async fn send<M>(&self, msg: M) -> Result<<T as Handler<M>>::Response, ActorError>
    where
        M: 'static + Send,
        T: Handler<M>,
    {
        let resp = self.msg_queue.send(msg)?;
        Ok(resp.await?)
    }
    /// Sends a message to the actor without waiting for response, ignoring all errors.
    pub fn do_send<M>(&self, msg: M)
    where
        M: 'static + Send,
        T: Handler<M>,
    {
        self.msg_queue.do_send(msg)
    }
    /// Sends a message to the actor without waiting for response.
    /// Fails if the message cannot be enqueued.
    pub fn try_send<M>(&self, msg: M) -> Result<(), ActorError>
    where
        M: 'static + Send,
        T: Handler<M>,
    {
        self.msg_queue.try_send(msg)
    }
    /// Returns a non-owning version of the address.
    /// 
    /// It can be used to prevent memory leaks resulting from circular references.
    pub fn downgrade(&self) -> WeakAddr<T> {
        WeakAddr::<T> {
            msg_queue: Arc::downgrade(&self.msg_queue),
        }
    }
}

/// A non-owning versions of actor address.
///
/// It can be used to prevent memory leaks resulting from circular references.
/// 
/// Much like with [Arc] and [Weak], the weak address does not 
/// necessarily refer to an actor that still exists in the memory.
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
    /// Try to retrieve a reference to the actor, if it still exists
    pub fn upgrade(&self) -> Option<Addr<T>> {
        Some(Addr::<T> {
            msg_queue: self.msg_queue.upgrade()?,
        })
    }
}
