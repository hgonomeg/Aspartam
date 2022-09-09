# Aspartam

Minimalistic actor framework based on tokio, inspired by actix

## Features

* Asynchronous actors
* Support for typed messages via generics

## Usage

TODO

## Why Aspartam

While `actix` is great, it makes using `async`/`await` problematic in message handlers. Workarounds exist but that continues to be a major pain-point. 

I decided to create something that feels similar to `actix` but requires a lot less hassle and has less complexity overall.


# TODO

* Documentation
* Simple actor lifecycle
  * Something like `actix`'s `Supervisor`
* Maybe something like `actix`'s `Recipient`
