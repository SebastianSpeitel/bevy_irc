#![deny(missing_docs)]

//! # TODO: Add documentation

use std::ops::{Deref, DerefMut};

use async_compat::Compat;
use bevy_ecs::prelude::*;
use bevy_tasks::{AsyncComputeTaskPool, Task};
use futures_lite::{future, StreamExt};
pub use irc;
use irc::client::prelude::*;
use irc::proto::Capability;
use irc::proto::Command;
use irc::proto::Message;
use log::{error, info, trace};
#[cfg(feature = "twitch")]
pub mod twitch;

/// Bevy component containing connection info
#[derive(Component)]
pub struct Connection {
    host: String,
    port: u16,
}

impl Connection {
    /// Create a connection component to the given host and port
    ///
    /// # Example
    /// ```
    /// use bevy_irc::Connection;
    ///
    /// let connection = Connection::new("irc.freenode.net", 6667);
    /// ```
    pub fn new(host: impl AsRef<str>, port: u16) -> Self {
        Self {
            host: host.as_ref().to_owned(),
            port,
        }
    }

    /// Create a connection component to the twitch IRC server
    ///
    /// # Example
    /// ```
    /// use bevy_irc::Connection;
    ///
    /// let connection = Connection::twitch();
    /// ```
    #[cfg(feature = "twitch")]
    pub fn twitch() -> Self {
        Self {
            host: "irc.chat.twitch.tv".to_owned(),
            port: 6697,
        }
    }
}

/// Bevy component containing the connected IRC client
#[derive(Component, Debug)]
pub struct Client(irc::client::Client);

impl Deref for Client {
    type Target = irc::client::Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Client {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Notype for IRC Messages to derive Event
#[derive(Event, Debug)]
pub struct MessageEvent(pub Message);

impl Deref for MessageEvent {
    type Target = Message;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MessageEvent {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Message> for MessageEvent {
    fn from(msg: Message) -> Self {
        Self(msg)
    }
}

/// Bevy component containing the IRC authentication info
///
/// # Example
/// ```
/// use bevy_irc::Authentication;
///
/// let nick_only = Authentication::new("nick");
/// let with_password = Authentication::new("nick").password("password");
/// ```
#[derive(Component)]
pub struct Authentication {
    nick: String,
    pass: Option<String>,
}

impl Authentication {
    /// Create a new authentication component with the given nickname
    pub fn new(nickname: impl AsRef<str>) -> Self {
        Self {
            nick: nickname.as_ref().to_owned(),
            pass: None,
        }
    }

    /// Set the password for the authentication component
    pub fn password(&mut self, password: impl AsRef<str>) -> &mut Self {
        self.pass = Some(password.as_ref().to_owned());
        self
    }
}

/// Bevy component containing the channels the client should be in
#[derive(Component)]
pub struct Channels(pub Vec<String>);

/// Bevy component containing the capabilities the client should request
///
/// # Example
/// ```
/// use bevy_irc::{Capabilities, irc::proto::Capability};
///
/// let capabilities = Capabilities(vec![
///   Capability::AwayNotify,
///   Capability::ServerTime,
/// ]);
/// ```
#[derive(Component)]
pub struct Capabilities(pub Vec<Capability>);

#[derive(Component)]
struct Stream(irc::client::ClientStream);

#[derive(Component)]
struct Connecting(Task<Result<irc::client::Client, irc::error::Error>>);

#[derive(Component)]
struct Identified;

fn connect(mut commands: Commands, chats: Query<(Entity, &Connection), Added<Connection>>) {
    let pool = AsyncComputeTaskPool::get();

    for (chat, con) in chats.iter() {
        let task = pool.spawn({
            let config = Config {
                server: Some(con.host.to_owned()),
                port: Some(con.port),
                ping_time: Some(u32::MAX),
                ..Default::default()
            };

            Compat::new(irc::client::Client::from_config(config))
        });
        commands.entity(chat).insert(Connecting(task));
    }
}

fn finish_connect(mut commands: Commands, mut chats: Query<(Entity, &mut Connecting)>) {
    for (chat, mut connecting) in chats.iter_mut() {
        match future::block_on(future::poll_once(&mut connecting.0)) {
            Some(Ok(mut client)) => {
                info!("Connected");
                commands.entity(chat).remove::<Connecting>();

                if let Ok(stream) = client.stream() {
                    commands.entity(chat).insert(Stream(stream));
                } else {
                    error!("Failed to get stream");
                }

                commands.entity(chat).insert(Client(client));
            }
            Some(Err(e)) => {
                error!("Failed to connect: {}", e);
                commands.entity(chat).remove::<Connecting>();
            }
            None => {}
        }
    }
}

fn identify(
    mut commands: Commands,
    mut chats: Query<(Entity, &Client, &Authentication), Without<Identified>>,
) {
    for (chat, client, auth) in chats.iter_mut() {
        info!("Identifying as {}", auth.nick);
        if let Some(pass) = auth.pass.as_ref() {
            if let Err(e) = client.send(Command::PASS(pass.to_owned())) {
                error!("Failed to send PASS: {}", e);
                continue;
            }
        }
        if let Err(e) = client.send(Command::NICK(auth.nick.to_owned())) {
            error!("Failed to send NICK: {}", e);
            continue;
        }
        commands.entity(chat).insert(Identified);
    }
}

fn join_and_part(
    mut chats: Query<
        (&Client, &Channels),
        (With<Identified>, Or<(Added<Identified>, Changed<Channels>)>),
    >,
) {
    for (client, channels) in chats.iter_mut() {
        info!("Joining and parting channels");
        let current = client.list_channels().unwrap_or_default();

        let to_join = channels.0.iter().filter(|c| !current.contains(c));
        let to_part = current.iter().filter(|c| !channels.0.contains(c));

        for channel in to_join {
            info!("Joining {}", channel);
            client
                .send(Command::JOIN(channel.to_owned(), None, None))
                .unwrap_or_else(|e| {
                    error!("Failed to send JOIN {}: {}", channel, e);
                });
        }

        for channel in to_part {
            info!("Parting {}", channel);
            client
                .send(Command::PART(channel.to_owned(), None))
                .unwrap_or_else(|e| {
                    error!("Failed to send PART {}: {}", channel, e);
                });
        }
    }
}
fn capabilities(
    mut chats: Query<
        (&Client, &Capabilities),
        (
            With<Identified>,
            Or<(Added<Identified>, Changed<Capabilities>)>,
        ),
    >,
) {
    for (client, caps) in chats.iter_mut() {
        info!("Requesting capabilities");

        client.send_cap_req(&caps.0).unwrap_or_else(|e| {
            error!("Failed to request capabilities: {}", e);
        });
    }
}

fn receive(mut writer: EventWriter<MessageEvent>, mut streams: Query<&mut Stream>) {
    for mut stream in streams.iter_mut() {
        while let Some(resp) = future::block_on(future::poll_once(&mut stream.0.next())).flatten() {
            match resp {
                Ok(msg) => {
                    trace!("Received: {:?}", msg.to_string().trim_end());
                    writer.send(msg.into());
                }
                Err(e) => {
                    error!("Failed to receive: {}", e);
                }
            }
        }
    }
}

/// Bevy plugin to connect and manage IRC connections
///
/// # Example
/// ```
/// use bevy_irc::{IRCPlugin, Connection, Authentication, Channels};
/// use bevy_app::prelude::*;
///
/// let mut app = App::new();
///
/// let irc = app.world.spawn((
///     Connection::new("irc.example.com", 6667),
///     Authentication::new("bevy"),
///     Channels(vec!["#bevy".to_owned()]),
/// ));
///
/// app.add_plugin(IRCPlugin);
/// ```
pub struct IRCPlugin;

impl bevy_app::Plugin for IRCPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        use bevy_app::Update;
        AsyncComputeTaskPool::init(Default::default);

        app.add_event::<MessageEvent>();
        app.add_systems(Update, connect);
        app.add_systems(Update, finish_connect);
        app.add_systems(Update, identify);
        app.add_systems(Update, join_and_part);
        app.add_systems(Update, capabilities);
        app.add_systems(Update, receive);
    }
}
