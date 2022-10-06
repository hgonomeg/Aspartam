# Aspartam

Minimalistic actor framework based on tokio, inspired by actix.

Aspartam tries to keep it simple and easy to use.

Messages are processed sequentially. 

## Features

* Asynchronous actors
* Support for typed messages via dynamic dispatch
* Support for asynchronous message handlers, via async-trait
* Actor supervision

## Usage

TODO

## Aspartam vs Actix

TODO

## Why Aspartam

While `actix` is great, it makes using `async`/`await` problematic in message handlers. Workarounds exist but that continues to be a major pain-point. 

I decided to create something that feels similar to `actix` but requires a lot less hassle and has less complexity overall.


## TODO

* Maybe something like `actix`'s `Recipient`
* Consider something like `ctx.after_future(fut,closure(actor,fut::Output,ctx))` or `AspartamFutureExt::then_for_actor(closure(actor,fut::Output,ctx) -> fut)` to mimic actix's `ActorFuture`
* Add API to allow running a future after stream ends
