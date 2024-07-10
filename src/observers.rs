#[allow(clippy::wildcard_imports)]
use crate::components::*;
use bevy_ecs::prelude::*;
use bevy_utils::tracing::{debug, error, info, trace};

use crate::irc_prelude as irc;

pub fn send<T: Into<irc::Message> + std::fmt::Debug + Clone>(
    trigger: Trigger<Outgoing<T>>,
    sender: Query<&Sender>,
    mut commands: Commands,
) {
    let msg = &trigger.event().0;
    let id = trigger.entity();
    let sender = match sender.get(id) {
        Ok(sender) => sender,
        Err(e) => {
            error!(message = "Failed to get sender", error=%e);
            return;
        }
    };
    trace!(message = "Sending message", ?msg);
    if let Err(e) = sender.send(msg.to_owned()) {
        error!(message = "Failed to send message", error=%e);
        commands.entity(id).remove::<Sender>();
    }
}

pub fn on_ping(trigger: Trigger<Incoming<irc::Command>>, mut commands: Commands) {
    let cmd = &trigger.event().0;
    let id = trigger.entity();
    if let irc::Command::PING(srv, ..) = cmd {
        debug!("Received PING");
        let pong = irc::Command::PONG(srv.to_owned(), None);
        commands.trigger_targets(Outgoing(pong), id);
    }
}

pub fn on_welcome(trigger: Trigger<Incoming<irc::Command>>, mut commands: Commands) {
    let msg = &trigger.event().0;
    if let irc::Command::Response(irc::Response::RPL_WELCOME, args) = msg {
        info!(message = "Registered", ?args);
        if let Some(mut entity) = commands.get_entity(trigger.entity()) {
            entity.remove::<Identifying>();
            entity.insert(Registered);
        }
    }
}
