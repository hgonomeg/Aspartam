//! Actor addresses

use crate::{
    actor::{Actor, Handler},
    error::*,
    message_queue::MessageQueue,
};
use std::sync::{Arc, Weak};

/// Basic actor address type.
/// 
/// Internally uses reference counting.
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
    pub async fn send<M>(&self, msg: M) -> Result<<T as Handler<M>>::Response, ActorError>
    where
        M: 'static + Send,
        T: Handler<M>,
    {
        let resp = self.msg_queue.send(msg)?;
        Ok(resp.await?)
    }
    pub fn do_send<M>(&self, msg: M)
    where
        M: 'static + Send,
        T: Handler<M>,
    {
        self.msg_queue.do_send(msg)
    }
    pub fn try_send<M>(&self, msg: M) -> Result<(), ActorError>
    where
        M: 'static + Send,
        T: Handler<M>,
    {
        self.msg_queue.try_send(msg)
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
