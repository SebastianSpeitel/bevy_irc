use bevy_app::{App, AppExit, ScheduleRunnerPlugin};
use bevy_irc::prelude::*;
use bevy_log::LogPlugin;
use std::time::Duration;

fn main() -> AppExit {
    let mut app = App::new();
    app.add_plugins((
        IRCPlugin,
        LogPlugin::default(),
        ScheduleRunnerPlugin::run_loop(Duration::from_millis(240)),
    ));

    app.world_mut().spawn((
        Connection::twitch(),
        Auth::new("justinfan9999").password("123"),
    ));

    app.run()
}
