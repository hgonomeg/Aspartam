//! Minimalistic actor framework based on tokio, inspired by actix.
//! 
//! Aspartam tries to keep it simple and easy to use.
//! 
//! Messages are processed sequentially. 
//! 
//! ## Features
//!
//! * Asynchronous actors
//! * Support for typed messages via dynamic dispatch
//! * Support for asynchronous message handlers, via async-trait
//! * Actor supervision

pub mod actor;
pub mod addr;
pub mod context;
pub mod error;
#[doc(hidden)]
pub mod message_queue;
mod runner;
pub mod supervisor;

pub mod prelude {
    //! Everything you need, re-exported
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
