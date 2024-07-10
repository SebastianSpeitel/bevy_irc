use bevy_app::{App, AppExit, ScheduleRunnerPlugin};
use bevy_irc::prelude::*;
use bevy_log::LogPlugin;

fn main() -> AppExit {
    let mut app = App::new();
    app.add_plugins((
        IRCPlugin,
        LogPlugin::default(),
        ScheduleRunnerPlugin::default(),
    ));

    app.world_mut().spawn((
        Connection::twitch(),
        Auth::new("justinfan1234").password(""),
    ));

    app.run()
}
