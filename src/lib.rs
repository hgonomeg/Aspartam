use async_trait::async_trait;
use std::sync::{Arc, Weak};
use tokio::sync::Mutex;
pub struct ActorContext<T: Actor> {
    address: WeakAddr<T>,
}

impl<T: Actor> ActorContext<T> {
    pub fn address(&self) -> Addr<T> {
        self.address.upgrade().unwrap()
    }
}

#[derive(Clone)]
pub struct Addr<T> {
    _inner: Arc<Mutex<T>>,
}

#[derive(Clone)]
pub struct WeakAddr<T> {
    _inner: Weak<Mutex<T>>,
}

impl<T> WeakAddr<T> {
    pub fn upgrade(&self) -> Option<Addr<T>> {
        self._inner.upgrade().map(|x| Addr::<T> { _inner: x })
    }
}

impl<T> Addr<T> {
    pub async fn send(msg: T) {}
    pub fn downgrade(&self) -> WeakAddr<T> {
        WeakAddr::<T> {
            _inner: Arc::<tokio::sync::Mutex<T>>::downgrade(&self._inner),
        }
    }
}

pub trait Actor: Sized {
    
}

#[async_trait]
pub trait Handler<T: Send>: Actor {
    async fn handle(&mut self, msg: T, ctx: &mut ActorContext<Self>);
}

#[cfg(test)]
mod tests {
    use super::*;

}
