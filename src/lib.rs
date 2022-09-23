pub mod actor;
pub mod addr;
pub mod context;
pub mod error;
pub mod message_queue;
mod runner;
pub mod supervisor;

pub mod prelude {
    pub use crate::{
        actor::{Actor, ActorState, Handler},
        addr::{Addr, WeakAddr},
        context::ActorContext,
        error::ActorError,
        supervisor::{Supervised, Supervisor},
    };
    pub use async_trait::async_trait;
    pub use futures_util::stream::{Stream, StreamExt};
}

#[cfg(test)]
mod tests;
