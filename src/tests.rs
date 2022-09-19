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
        async fn handle(&mut self, _msg: Ping, _ctx: &mut ActorContext<Self>) -> Self::Response {
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
        async fn handle(&mut self, _msg: Ping, _ctx: &mut ActorContext<Self>) -> Self::Response {
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

#[test]
fn do_send_gets_delivered() {
    use std::collections::HashSet;

    #[derive(Hash, Eq, PartialEq, Clone)]
    struct Fingerprint {
        some_data: u32,
        some_text: String,
    }

    #[derive(Default)]
    struct Memorizer {
        hashset: HashSet<Fingerprint>,
    }
    impl Actor for Memorizer {}

    struct HasFingerprint(Fingerprint);

    #[async_trait]
    impl Handler<HasFingerprint> for Memorizer {
        type Response = bool;

        async fn handle(
            &mut self,
            msg: HasFingerprint,
            _ctx: &mut ActorContext<Self>,
        ) -> Self::Response {
            self.hashset.contains(&msg.0)
        }
    }

    #[async_trait]
    impl Handler<Fingerprint> for Memorizer {
        type Response = bool;

        async fn handle(
            &mut self,
            msg: Fingerprint,
            _ctx: &mut ActorContext<Self>,
        ) -> Self::Response {
            self.hashset.insert(msg)
        }
    }
    get_runtime().block_on(async {
        let memo = Memorizer::default().start();
        let do_send_fingerprints = vec![
            Fingerprint {
                some_data: 775,
                some_text: "Hello".to_owned(),
            },
            Fingerprint {
                some_data: 221,
                some_text: "Good morning".to_owned(),
            },
            Fingerprint {
                some_data: 348,
                some_text: "Good afternoon".to_owned(),
            },
            Fingerprint {
                some_data: 726,
                some_text: "Hi".to_owned(),
            },
            Fingerprint {
                some_data: 823,
                some_text: "Good evening".to_owned(),
            },
        ];
        let mut send_fingerprints = vec![
            Fingerprint {
                some_data: 135,
                some_text: "Water".to_owned(),
            },
            Fingerprint {
                some_data: 776,
                some_text: "Stone".to_owned(),
            },
            Fingerprint {
                some_data: 285,
                some_text: "Fire".to_owned(),
            },
            Fingerprint {
                some_data: 431,
                some_text: "Air".to_owned(),
            },
        ];
        let not_sent_fingerprints = vec![
            Fingerprint {
                some_data: 113,
                some_text: "Underwater".to_owned(),
            },
            Fingerprint {
                some_data: 663,
                some_text: "UFO".to_owned(),
            },
            Fingerprint {
                some_data: 667,
                some_text: "Bermuda".to_owned(),
            },
        ];
        let list = send_fingerprints.clone();
        let m = memo.clone();
        let send_job = tokio::spawn(async move {
            for i in list.into_iter() {
                m.send(i).await;
            }
        });

        for i in do_send_fingerprints.iter().cloned() {
            memo.do_send(i);
        }

        let last_one = Fingerprint {
            some_data: 720,
            some_text: "Essence".to_owned(),
        };

        send_fingerprints.push(last_one.clone());
        let all_sent_fingerprints = [send_fingerprints, do_send_fingerprints].concat();

        send_job.await.unwrap();
        memo.send(last_one).await;

        for i in not_sent_fingerprints.into_iter() {
            assert_eq!(memo.send(HasFingerprint(i)).await, false);
        }

        for i in all_sent_fingerprints.into_iter() {
            assert_eq!(memo.send(HasFingerprint(i)).await, true);
        }
    })
}

#[test]
fn basic_actor_lifecycle() {
    use crate::actor::Stopping;
    //use std::time::Duration;

    struct DummyHandler {
        should_terminate: Option<bool>,
        stopped_notifier: Option<oneshot::Sender<()>>,
    }

    #[async_trait]
    impl Actor for DummyHandler {
        async fn stopped(&mut self, _ctx: &mut ActorContext<Self>) {
            self.stopped_notifier.take().unwrap().send(()).unwrap()
        }
        async fn stopping(&mut self, _ctx: &mut ActorContext<Self>) -> Stopping {
            match self.should_terminate {
                None => {
                    panic!("This should be unreachable. Actor is stopping without a reason.")
                }
                Some(non_recoverable_state) => {
                    if non_recoverable_state {
                        Stopping::Stop
                    } else {
                        Stopping::Continue
                    }
                }
            }
        }
    }

    enum DummyResult {
        Allright,
        RecoverableError,
        StopProcessing,
    }

    struct NeverDelivered;

    struct ShouldBeDelivered(oneshot::Sender<()>);

    impl ShouldBeDelivered {
        pub fn new() -> (Self, oneshot::Receiver<()>) {
            let (tx, rx) = oneshot::channel();
            (Self(tx), rx)
        }
    }

    #[async_trait]
    impl Handler<NeverDelivered> for DummyHandler {
        type Response = ();
        async fn handle(
            &mut self,
            _item: NeverDelivered,
            _ctx: &mut ActorContext<Self>,
        ) -> Self::Response {
            panic!("This is meant to be unreachable. Actor did not stop.");
        }
    }

    #[async_trait]
    impl Handler<DummyResult> for DummyHandler {
        type Response = ();
        async fn handle(
            &mut self,
            item: DummyResult,
            ctx: &mut ActorContext<Self>,
        ) -> Self::Response {
            match item {
                DummyResult::Allright => {
                    self.should_terminate = None;
                }
                DummyResult::RecoverableError => {
                    self.should_terminate = Some(false);
                    ctx.stop();
                }
                DummyResult::StopProcessing => {
                    self.should_terminate = Some(true);
                    ctx.stop();
                }
            }
        }
    }

    #[async_trait]
    impl Handler<ShouldBeDelivered> for DummyHandler {
        type Response = ();
        async fn handle(
            &mut self,
            item: ShouldBeDelivered,
            _ctx: &mut ActorContext<Self>,
        ) -> Self::Response {
            let tx = item.0;
            tx.send(()).unwrap();
        }
    }

    get_runtime().block_on(async {
        let (tx, stopped_notifier) = oneshot::channel();
        let actor = DummyHandler {
            stopped_notifier: Some(tx),
            should_terminate: None,
        }
        .start();

        let mut rxs = vec![];
        for _i in 0..100 {
            let (vibe_check, rx) = ShouldBeDelivered::new();
            actor.do_send(vibe_check);
            actor.do_send(DummyResult::Allright);
            actor.do_send(DummyResult::RecoverableError);
            rxs.push(rx);
        }
        for rx in rxs.drain(..) {
            // ensure that "stopping -> continue" cycles do not drop events
            rx.await.unwrap();
        }
        actor.send(DummyResult::StopProcessing).await;

        // With current error-handling this has to cause panic.
        // actor.do_send(NeverDelivered);

        //ensure that actor gets stopped
        stopped_notifier.await.unwrap();

        // With current error-handling this has to cause panic. Can be uncommented after the API gets changed.

        // let act = actor.clone();
        // let never_finishing = tokio::spawn(async move { act.send(NeverDelivered).await; });
        // assert!(tokio::time::timeout(Duration::from_millis(200), never_finishing).await.is_err());
    })
}
