use crate::core::Core;
use crate::error::CustomError;
use crate::server::websocket::PleaseUpdate;
use actix::prelude::*;
use actix::{Actor, Context, Handler, Recipient};
use std::sync::Arc;
use std::time::Duration;

/// How often we update the server
const UPDATE_INTERVAL: Duration = Duration::from_secs(1);

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
    last_version: usize,
    core: Arc<Core>,
    subscribers: Vec<Recipient<PleaseUpdate>>,
}

impl Actor for UpdateMasterActor {
    type Context = Context<Self>;

    /// Method is called on actor start. We start the heartbeat process here.
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(UPDATE_INTERVAL, |act, _ctx| {
            if let Err(err) = act.tick() {
                log::error!("{}", err.message);
            }
        });
    }
}

impl UpdateMasterActor {
    pub fn new(core: Arc<Core>) -> UpdateMasterActor {
        UpdateMasterActor {
            last_version: 0,
            subscribers: Vec::new(),
            core,
        }
    }

    pub fn tick(&mut self) -> Result<(), CustomError> {
        // Tell the core to update itself if required
        Core::check_for_graph_update(self.core.clone())?;

        // Check if a new version the graph is ready
        let version = self.core.version()?;
        if version != self.last_version {
            self.last_version = version;
            self.send_please_update_message()
        }

        Ok(())
    }

    /// Send signal to all subscribers
    fn send_please_update_message(&mut self) {
        for subscr in &self.subscribers {
            if let Err(err) = subscr.do_send(PleaseUpdate) {
                log::error!("While sending PleaseUpdate message: {:?}", err);
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
