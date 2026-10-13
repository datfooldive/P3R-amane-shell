mod clock;
mod hud;
mod hyprland;
mod lock_screen;
mod menu;
mod player;
mod settings;
mod theme;
mod tray;
mod tray_menu;
mod visualizer;
mod wallpaper;

use amane::{App, Lock};

fn main() {
    App::new()
        .font(theme::DISPLAY)
        .ipc("menu", |_| menu::toggle())
        .ipc("settings", |arguments| {
            let tab = match arguments.first().map(String::as_str) {
                Some("power" | "battery") => settings::Tab::Battery,
                Some("network" | "wifi") => settings::Tab::Network,
                _ => settings::Tab::Audio,
            };
            settings::show(tab);
            String::from("ok")
        })
        .ipc("player", |_| {
            player::toggle();
            String::from("ok")
        })
        .ipc("lock", |_| {
            Lock::start();
            String::from("locked")
        })
        .window_per_monitor(wallpaper::view)
        .window_per_monitor(hud::view)
        .window(settings::view)
        .window(player::view)
        .window(tray_menu::view)
        .window(menu::view)
        .lock(lock_screen::view)
        .run();
}
