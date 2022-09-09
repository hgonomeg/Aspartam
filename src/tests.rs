use crate::prelude::*;
use tokio::sync::oneshot;

fn get_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

#[test]
fn basic_messages() {
    struct Ping;
    struct Pong;

    struct Game;

    impl Actor for Game {}
    #[async_trait]
    impl Handler<Ping> for Game {
        type Response = Pong;
        async fn handle(
            &mut self,
            _msg: Ping,
            _ctx: &mut ActorContext<Self>,
        ) -> Self::Response {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            Pong
        }
    }

    get_runtime().block_on(async {
        let game = Game.start();
        let _pong = game.send(Ping).await;
    });
}

#[test]
fn data_sanity() {
    struct Incrementor {
        request_count: usize,
    }
    impl Actor for Incrementor {}
    #[async_trait]
    impl Handler<u32> for Incrementor {
        type Response = u32;
        async fn handle(&mut self, msg: u32, _ctx: &mut ActorContext<Self>) -> Self::Response {
            self.request_count += 1;
            msg + 1
        }
    }
    struct GetRequestCount;
    #[async_trait]
    impl Handler<GetRequestCount> for Incrementor {
        type Response = usize;
        async fn handle(
            &mut self,
            _msg: GetRequestCount,
            _ctx: &mut ActorContext<Self>,
        ) -> Self::Response {
            self.request_count
        }
    }

    get_runtime().block_on(async {
        let incrementor = Incrementor { request_count: 0 }.start();
        assert_eq!(incrementor.send(GetRequestCount).await, 0);
        assert_eq!(incrementor.send(2).await, 3);
        assert_eq!(incrementor.send(GetRequestCount).await, 1);
        assert_eq!(incrementor.send(7).await, 8);
        assert_eq!(incrementor.send(9).await, 10);
        assert_eq!(incrementor.send(GetRequestCount).await, 3);
        let mut i = 0;
        while i < 5000 {
            let r = incrementor.send(i).await;
            i += 1;
            assert_eq!(r, i);
        }
        assert_eq!(incrementor.send(GetRequestCount).await, 5003);
    });
}
#[test]
fn memory_leaks() {
    struct DropMe {
        tx: Option<oneshot::Sender<()>>,
    }
    impl Actor for DropMe {}
    impl Drop for DropMe {
        fn drop(&mut self) {
            self.tx.take().unwrap().send(()).unwrap();
        }
    }
    get_runtime().block_on(async {
        let (tx, rx) = oneshot::channel();
        let d = DropMe { tx: Some(tx) }.start();
        drop(d);
        rx.await.unwrap();
    });
}
#[test]
fn actor_create() {
    struct Secondary {
        _prim: WeakAddr<Primary>,
    }
    impl Actor for Secondary {}
    struct Primary {
        _sec: Addr<Secondary>,
    }
    impl Actor for Primary {}

    get_runtime().block_on(async {
        let _prim = Primary::create(move |a| {
            let this = a.address();
            Primary {
                _sec: Secondary {
                    _prim: this.downgrade(),
                }
                .start(),
            }
        });
    })
}
#[test]
fn handling_streams() {
    struct Sh {
        tx: Option<oneshot::Sender<usize>>,
        message_count: usize,
    }
    impl Actor for Sh {}
    #[derive(Clone)]
    struct Ping;
    struct As<S> {
        stream: S,
    }
    unsafe impl<S> Send for As<S> {}
    #[async_trait]
    impl Handler<Ping> for Sh {
        type Response = ();
        async fn handle(
            &mut self,
            _msg: Ping,
            _ctx: &mut ActorContext<Self>,
        ) -> Self::Response {
            self.message_count += 1;
        }
    }
    #[async_trait]
    impl<S> Handler<As<S>> for Sh
    where
        S: 'static + Stream<Item = Ping> + Unpin + Send,
    {
        type Response = ();
        async fn handle(&mut self, msg: As<S>, ctx: &mut ActorContext<Self>) -> Self::Response {
            ctx.add_stream(msg.stream);
        }
    }
    impl Drop for Sh {
        fn drop(&mut self) {
            self.tx.take().unwrap().send(self.message_count).unwrap();
        }
    }
    get_runtime().block_on(async {
        let (tx, rx) = oneshot::channel();
        let d = Sh {
            tx: Some(tx),
            message_count: 0,
        }
        .start();
        let stream = futures_util::stream::iter(std::iter::repeat(Ping).take(1000));
        d.send(As { stream }).await;

        let stream2 = futures_util::stream::iter(std::iter::repeat(Ping).take(5000));
        d.send(As { stream: stream2 }).await;

        let stream3 = futures_util::stream::iter(std::iter::repeat(Ping).take(10000));
        d.send(As { stream: stream3 }).await;

        d.send(Ping).await;
        d.send(Ping).await;
        d.send(Ping).await;
        d.send(Ping).await;
        d.send(Ping).await;

        drop(d);
        assert_eq!(rx.await.unwrap(), 16005);
    });
}