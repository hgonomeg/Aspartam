pub mod actor;
pub mod addr;
pub mod context;
pub mod message_queue;

pub mod prelude {
    pub use crate::{
        actor::{Actor, Handler},
        addr::{Addr, WeakAddr},
        context::ActorContext,
    };
    pub use async_trait::async_trait;
    pub use futures_util::stream::{Stream, StreamExt};
}

#[cfg(test)]
mod tests;
