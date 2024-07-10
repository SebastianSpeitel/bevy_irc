use std::{
    ops::{Deref, DerefMut},
    sync::Mutex,
};

use bevy_ecs::prelude::*;
use bevy_time::Stopwatch;
use bevy_utils::{BoxedFuture, ConditionalSend};

use crate::irc_prelude as irc;

/// Bevy component containing connection info
#[derive(Component, Clone, Debug)]
pub struct Connection {
    host: String,
    port: u16,
}

impl Connection {
    /// Create a connection component to the given host and port
    ///
    /// # Example
    /// ```
    /// use bevy_irc::prelude::*;
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
    /// use bevy_irc::prelude::*;
    ///
    /// let connection = Connection::twitch();
    /// ```
    #[cfg(feature = "twitch")]
    #[must_use]
    pub fn twitch() -> Self {
        Self {
            host: "irc.chat.twitch.tv".to_owned(),
            port: 6697,
        }
    }
}

impl From<&Connection> for irc::Config {
    #[inline]
    fn from(con: &Connection) -> Self {
        irc::Config {
            server: Some(con.host.clone()),
            port: Some(con.port),
            ping_time: Some(u32::MAX),
            ..Default::default()
        }
    }
}

/// Bevy component containing the IRC sender
#[derive(Component, Debug)]
pub struct Sender(pub(crate) irc::Sender);

impl Deref for Sender {
    type Target = irc::Sender;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Bevy component containing the IRC authentication info
///
/// # Example
/// ```
/// use bevy_irc::prelude::*;
///
/// let nick_only = Auth::new("nick");
/// let with_password = Auth::new("nick").password("password");
/// ```
#[derive(Component, Debug)]
pub struct Auth {
    /// Nickname send using the `NICK` command
    pub nick: String,
    /// Password sent using the `PASS` command
    pub pass: Option<String>,
}

impl Auth {
    /// Create a new authentication component with the given nickname
    pub fn new(nickname: impl AsRef<str>) -> Self {
        Self {
            nick: nickname.as_ref().to_owned(),
            pass: None,
        }
    }

    /// Set the password for the authentication component
    #[inline]
    #[must_use]
    pub fn password(self, password: impl AsRef<str>) -> Self {
        Self {
            pass: Some(password.as_ref().to_owned()),
            ..self
        }
    }
}

/// Bevy component containing the channels the client should be in
#[derive(Component, Debug, Default)]
pub struct Channels(pub Vec<String>);

/// Bevy component containing the capabilities the client should request
///
/// # Example
/// ```
/// use bevy_irc::{prelude::*, irc::client::prelude::*};
///
/// let capabilities = Capabilities(vec![
///   Capability::AwayNotify,
///   Capability::ServerTime,
/// ]);
/// ```
#[derive(Component, Debug)]
pub struct Capabilities(pub Vec<irc::Capability>);

/// Bevy component containing the IRC client stream
#[derive(Component, Debug)]
pub struct Stream(pub(crate) irc::ClientStream);

#[derive(Component)]
pub(crate) struct Connecting(Mutex<BoxedFuture<'static, Result<irc::Client, irc::Error>>>);

impl Connecting {
    #[inline]
    pub fn new(
        fut: impl std::future::Future<Output = Result<irc::Client, irc::Error>>
            + ConditionalSend
            + 'static,
    ) -> Self {
        Self(Mutex::new(Box::pin(fut)))
    }
}

impl Deref for Connecting {
    type Target = Mutex<BoxedFuture<'static, Result<irc::Client, irc::Error>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Connecting {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Component, Debug)]
pub(crate) struct Registered;

#[derive(Component, Debug)]
pub(crate) struct Identifying;

#[derive(Event, Debug, Default)]
pub(crate) struct Pinger {
    pub(crate) last_ping: Stopwatch,
}

/// Bevy Event for incoming IRC messages and commands
#[derive(Event, Debug, Clone)]
pub struct Incoming<T>(pub(crate) T);

impl<T> Deref for Incoming<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Bevy Event for outgoing IRC messages and commands
#[derive(Event, Debug, Clone)]
pub struct Outgoing<T>(pub(crate) T);

impl<T> Deref for Outgoing<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Outgoing<irc::Command> {
    /// Create a new outgoing command event
    #[inline]
    #[must_use]
    pub fn new(command: irc::Command) -> Self {
        Self(command)
    }
}

impl Outgoing<irc::Message> {
    /// Create a new outgoing message event
    #[inline]
    #[must_use]
    pub fn new(message: irc::Message) -> Self {
        Self(message)
    }
}
