#![warn(missing_docs)]
#![allow(clippy::type_complexity, clippy::needless_pass_by_value)]

//! # TODO: Add documentation

use bevy_utils::tracing::warn;
pub use irc;

/// Bevy components
pub mod components;
/// Bevy systems
mod systems;
/// Utilities for using the Twitch IRC
#[cfg(feature = "twitch")]
pub mod twitch;

mod irc_prelude {
    pub use irc::client::prelude::*;
    pub use irc::client::ClientStream;
    pub use irc::error::Error;
    pub use irc::proto::CapSubCommand;
}

#[allow(missing_docs)]
pub mod prelude {
    pub use super::IRCPlugin;
    pub use crate::components::*;
}

/// Bevy plugin to connect and manage IRC connections
///
/// # Example
/// ```
/// use bevy_irc::prelude::*;
/// use bevy_app::prelude::*;
///
/// let mut app = App::new();
///
/// let irc = app.world_mut().spawn((
///     Connection::new("irc.example.com", 6667),
///     Auth::new("bevy"),
///     Channels(vec!["#bevy".to_owned()]),
/// ));
///
/// app.add_plugins(IRCPlugin);
/// ```
pub struct IRCPlugin;

impl bevy_app::Plugin for IRCPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        use bevy_app::Update;

        if !app.is_plugin_added::<bevy_time::TimePlugin>() {
            app.add_plugins(bevy_time::TimePlugin);
        }

        app.add_event::<components::Incoming>();
        app.world_mut()
            .observe(systems::send::<irc_prelude::Message>);
        app.world_mut()
            .observe(systems::send::<irc_prelude::Command>);

        app.add_systems(
            Update,
            (
                systems::connect,
                systems::poll_connecting,
                systems::identify,
                systems::request_capabilities,
                systems::join_channels,
                systems::poll_stream,
                systems::ping,
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::prelude::*;
    #[test]
    fn test_connection() {
        let mut app = bevy_app::App::new();
        app.add_plugins(bevy_log::LogPlugin::default());
        // app.add_plugins(bevy_app::ScheduleRunnerPlugin::default());
        app.add_plugins(IRCPlugin);

        app.world_mut().spawn((
            Connection::new("irc.example.com", 6667),
            Auth::new("bevy"),
            Channels(vec!["#bevy".to_owned()]),
        ));

        println!("Running app");

        app.run();
    }
}
