use crate::server::websocket::PleaseUpdate;
use actix::prelude::*;
use actix::{Actor, Context, Handler, Recipient};
use std::time::Duration;

/// How often we update the server
const UPDATE_INTERVAL: Duration = Duration::from_secs(5);

/// Subscribe to process signals.
#[derive(Message)]
#[rtype(result = "()")]
pub struct Subscribe(pub Recipient<PleaseUpdate>);

/// Unsubscribe from process signals.
#[derive(Message)]
#[rtype(result = "()")]
pub struct Unsubscribe(pub Recipient<PleaseUpdate>);

/// Actor that provides signal subscriptions
pub struct UpdateMasterActor {
    subscribers: Vec<Recipient<PleaseUpdate>>,
}

impl UpdateMasterActor {
    pub fn new() -> UpdateMasterActor {
        UpdateMasterActor {
            subscribers: Vec::new(),
        }
    }
}

impl Actor for UpdateMasterActor {
    type Context = Context<Self>;

    /// Method is called on actor start. We start the heartbeat process here.
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(UPDATE_INTERVAL, |act, _ctx| {
            act.send_please_update_message();
        });
    }
}

impl UpdateMasterActor {
    /// Send signal to all subscribers
    fn send_please_update_message(&mut self) {
        for subscr in &self.subscribers {
            if let Err(err) = subscr.do_send(PleaseUpdate) {
                println!("{:?}", err);
            }
        }
    }
}

/// Subscribe to signals
impl Handler<Subscribe> for UpdateMasterActor {
    type Result = ();

    fn handle(&mut self, msg: Subscribe, _: &mut Self::Context) {
        self.subscribers.push(msg.0);
    }
}

/// Unsubscribe
impl Handler<Unsubscribe> for UpdateMasterActor {
    type Result = ();

    fn handle(&mut self, msg: Unsubscribe, _: &mut Self::Context) {
        self.subscribers.retain(|x| x != &msg.0);
    }
}
