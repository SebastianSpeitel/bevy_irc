#[allow(clippy::wildcard_imports)]
use crate::components::*;
use async_compat::CompatExt;
use bevy_ecs::prelude::*;
use bevy_time::{Real, Time};
use bevy_utils::{
    futures::{check_ready, now_or_never},
    tracing::{error, info, trace, warn},
};

use crate::irc_prelude as irc;

pub fn connect(
    mut commands: Commands,
    chats: Query<
        (Entity, &Connection),
        (Without<Connecting>, Or<(Without<Sender>, Without<Stream>)>),
    >,
) {
    for (id, con) in &chats {
        let mut entity = commands.entity(id);
        let config = con.into();
        info!(message = "Connecting", ?config);
        let fut = irc::Client::from_config(config);
        // let fut = Box::pin(fut);
        // let fut = Compat::new(boxed_fut);
        let connecting = Connecting::new(fut);
        entity.insert((connecting, Pinger::default()));
        entity.remove::<Registered>();
    }
}

pub fn poll_connecting(mut commands: Commands, mut chats: Query<(Entity, &mut Connecting)>) {
    for (id, mut connecting) in &mut chats {
        let mut fut = connecting.get_mut().unwrap().compat();

        if let Some(res) = check_ready(&mut fut) {
            let mut entity = commands.entity(id);
            entity.remove::<Connecting>();
            match res {
                Ok(mut client) => {
                    info!(message = "Connected", ?client);
                    entity.insert(Sender(client.sender()));
                    entity.insert(Stream(client.stream().unwrap()));
                }
                Err(e) => {
                    error!(message = "Failed to connect", error=%e);
                    continue;
                }
            }
        }
    }
}

pub fn ping(
    mut pingers: Query<(Entity, &mut Pinger)>,
    time: Res<Time<Real>>,
    mut commands: Commands,
) {
    for (id, mut pinger) in &mut pingers {
        if pinger.last_ping.tick(time.delta()).elapsed_secs() < 600.0 {
            return;
        }
        let ping = irc::Command::PING(String::new(), None);
        commands.trigger_targets(Outgoing::new(ping), id);
        pinger.last_ping.reset();
    }
}

pub fn identify(
    mut commands: Commands,
    chats: Query<(Entity, &Auth), (With<Sender>, Without<Registered>, Without<Identifying>)>,
) {
    for (id, auth) in &chats {
        commands.entity(id).insert(Identifying);
        info!(message = "Identifying", ?auth);
        if let Some(pass) = &auth.pass {
            commands.trigger_targets(Outgoing::new(irc::Command::PASS(pass.clone())), id);
        }
        commands.trigger_targets(Outgoing::new(irc::Command::NICK(auth.nick.clone())), id);
    }
}

pub fn join_channels(
    mut commands: Commands,
    chats: Query<
        (Entity, &Channels),
        (With<Registered>, Or<(Added<Registered>, Changed<Channels>)>),
    >,
) {
    for (id, channels) in &chats {
        info!(message = "Joining channels", ?channels);
        for channel in &channels.0 {
            let join = irc::Command::JOIN(channel.to_owned(), None, None);
            commands.trigger_targets(Outgoing::new(join), id);
        }
    }
}

pub fn request_capabilities(
    mut commands: Commands,
    chats: Query<
        (Entity, &Capabilities),
        (
            With<Registered>,
            Or<(Added<Registered>, Changed<Capabilities>)>,
        ),
    >,
) {
    for (id, caps) in &chats {
        info!(message = "Requesting capabilities", ?caps);
        let caps = caps
            .0
            .iter()
            .map(irc::Capability::as_ref)
            .collect::<Vec<_>>()
            .join(" ");
        let req = irc::Command::CAP(None, irc::CapSubCommand::REQ, None, Some(caps));

        commands.trigger_targets(Outgoing::new(req), id);
    }
}

pub fn poll_stream(
    mut commands: Commands,
    mut streams: Query<(Entity, &mut Stream)>,
    mut incoming: EventWriter<Incoming>,
) {
    use futures_util::StreamExt;
    for (id, mut stream) in &mut streams {
        loop {
            let Some(next) = now_or_never(stream.0.next()) else {
                break;
            };
            match next {
                None => {
                    warn!(message = "Stream ended", ?stream);
                    commands.entity(id).remove::<Stream>();
                    break;
                }
                Some(Ok(msg)) => {
                    trace!(message = "Received message", ?msg);
                    commands.trigger_targets(Incoming(msg.clone()), id);
                    incoming.send(Incoming(msg));
                }
                Some(Err(e)) => {
                    error!(message = "Failed to poll stream", error=%e, ?stream);
                    commands.entity(id).remove::<Stream>();
                    break;
                }
            }
        }
    }
}
