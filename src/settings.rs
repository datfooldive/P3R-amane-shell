use std::time::Duration;

use amane::{
    AccessPoint, Animation, Audio, Battery, Brightness, Button, Center, Color, Column, Easing,
    Gradient, Horizontal, Key, Keyboard, Layer, LayerWindow, Link, Margin, Network, Padding,
    Parent, Point, Pointer, Rectangle, Row, ScrollArea, Service, Shadow, SpaceBetween, Start,
    TextInput, Vertical, Widget, children,
};

use crate::clock::Clock;
use crate::theme::{self, AQUA, DEEP, INK, NAVY, RED, color, heading, icon, label};

const WIDTH: f32 = 440.0;
const HEIGHT: f32 = 580.0;
const GAP: i32 = 16;
const TRACK: f32 = 300.0;
const PASSWORD: &str = "p3r-wifi-password";

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Battery,
    Audio,
    Network,
}

const TABS: [(Tab, &str, &str); 3] = [
    (Tab::Battery, "\u{f240}", "POWER"),
    (Tab::Audio, "\u{f028}", "AUDIO"),
    (Tab::Network, "\u{f1eb}", "NETWORK"),
];

pub struct Settings {
    open: bool,
    tab: Tab,
    slide: Animation,
    // the secured network waiting for a password
    asking: Option<String>,
}

impl Service for Settings {
    fn new() -> Self {
        Self {
            open: false,
            tab: Tab::Audio,
            slide: Animation::new(0.0).duration(Duration::from_millis(280)).easing(Easing::Out),
            asking: None,
        }
    }

    fn interval() -> Duration {
        Duration::from_secs(3600)
    }
}

// clicking the chip of the open tab closes the panel, any other chip switches to its tab
pub fn show(tab: Tab) {
    let mut settings = Settings::write();
    if settings.open && settings.tab == tab {
        settings.open = false;
        settings.slide.to(0.0);
        return;
    }
    if !settings.open {
        settings.slide.to(1.0);
        crate::player::close();
        crate::tray_menu::close();
    }
    settings.open = true;
    settings.tab = tab;
    settings.asking = None;
    if tab == Tab::Network {
        Network::scan();
    }
}

pub fn close() {
    if !Settings::read().open {
        return;
    }
    let mut settings = Settings::write();
    settings.open = false;
    settings.asking = None;
    settings.slide.to(0.0);
}

fn key(key: Key) {
    if key == Key::Escape {
        if Settings::read().asking.is_some() {
            Settings::write().asking = None;
        } else {
            close();
        }
    }
}

pub fn view() -> LayerWindow {
    let settings = Settings::read();
    let clock = Clock::read();
    let slide = settings.slide.value();
    let shown = settings.open || slide > 0.01;
    let accent = theme::accent(clock.dark_hour());

    /*
     * the window never moves or resizes, and never hides: resizing a layer surface on open
     * showed a frame at the old size first, and visible(false) can map it again by itself
     * after the hide; closed, it just draws nothing and lets every click through
     */
    let window = LayerWindow::new()
        .width(WIDTH + GAP as f32 + 20.0)
        .height(HEIGHT + 20.0)
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Right)
        .layer(Layer::Overlay)
        .margin(Margin { top: 6, right: 0, bottom: 0, left: 0 })
        .namespace("p3r-settings")
        .keyboard(if settings.open { Keyboard::OnDemand } else { Keyboard::None })
        .on_key(key);

    if !shown {
        return window.click_through().child(Rectangle::new().width(Parent).height(Parent));
    }

    let body: Box<dyn Widget> = match settings.tab {
        Tab::Battery => Box::new(power_page(accent)),
        Tab::Audio => Box::new(audio_page(accent)),
        Tab::Network => Box::new(network_page(&settings, accent)),
    };

    window.child(
        Rectangle::new()
            .width(Parent)
            .height(Parent)
            .padding(Padding { top: 0.0, right: GAP as f32 + 20.0, bottom: 20.0, left: 0.0 })
            .child(
                Rectangle::new()
                    .width(WIDTH)
                    .height(HEIGHT)
                    // slides in from behind the right edge of the screen
                    .translate((1.0 - slide) * (WIDTH + GAP as f32 + 30.0), 0.0)
                    .opacity(slide)
                    .fill(Gradient::linear(160.0, [(0.0, theme::with_alpha(NAVY, 0xf4)), (1.0, theme::with_alpha(DEEP, 0xf0))]))
                    .border(2.0, color(accent))
                    .shadow(Shadow::drop(color(accent)).blur(0.0).offset(10.0, 10.0).opacity(0.45))
                    .child(
                        Column::new(children![
                            header(settings.tab, accent),
                            Rectangle::new()
                                .width(Parent)
                                .height(Parent)
                                .padding(Padding { top: 18.0, right: 22.0, bottom: 18.0, left: 22.0 })
                                .child(Column::new(vec![body]).width(Parent).height(Parent)),
                        ])
                        .width(Parent)
                        .height(Parent),
                    ),
            ),
    )
}

fn header(current: Tab, accent: &str) -> Rectangle {
    let mut tabs: Vec<Box<dyn Widget>> = Vec::new();
    for (tab, glyph, name) in TABS {
        let selected = tab == current;
        let ink = if selected { INK } else { "#ffffff" };
        let plate = Rectangle::new()
            .width(118.0)
            .height(38.0)
            .rotate(-8.0)
            .align_child(Center, Center)
            .cursor(Pointer)
            .on_click(move |_| show_tab(tab))
            .child(Row::new(children![icon(glyph, 14.0, ink), heading(name, 16.0, ink)]).gap(8.0).align(Center));
        let plate = if selected {
            plate.fill(Color::WHITE).shadow(Shadow::drop(color(accent)).blur(0.0).offset(5.0, 5.0)).scale(1.06)
        } else {
            plate.fill(theme::with_alpha(INK, 0xcc)).border(1.5, theme::with_alpha(accent, 0x77))
        };
        tabs.push(Box::new(plate));
    }

    Rectangle::new()
        .width(Parent)
        .height(70.0)
        .fill(theme::with_alpha(INK, 0xaa))
        .padding(Padding { top: 0.0, right: 16.0, bottom: 0.0, left: 16.0 })
        .align_child(Center, Center)
        .child(Row::new(tabs).gap(12.0).align(Center))
}

// switching tabs from inside the panel never closes it
fn show_tab(tab: Tab) {
    if Settings::read().tab != tab {
        show(tab);
    }
}

fn section(name: &str, accent: &str) -> Rectangle {
    Rectangle::new()
        .width(Parent)
        .height(26.0)
        .fill(Gradient::linear(90.0, [(0.0, color(accent)), (1.0, theme::with_alpha(accent, 0x00))]))
        .padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 10.0 })
        .align_child(Start, Center)
        .child(heading(name, 13.0, INK))
}

// a track you press or drag along; `set` gets a value from 0 to 100
fn slider(value: u8, accent: &str, muted: bool, set: fn(u8)) -> Rectangle {
    let fraction = value.min(100) as f32 / 100.0;
    let hue = if muted { "#4a5f99" } else { accent };
    let pick = move |point: Point| set(((point.x / TRACK).clamp(0.0, 1.0) * 100.0).round() as u8);

    Rectangle::new()
        .width(TRACK)
        .height(22.0)
        .fill(theme::with_alpha(INK, 0xff))
        .border(1.5, theme::with_alpha(accent, 0x66))
        .clip()
        .cursor(Pointer)
        .on_drag(pick)
        .on_scroll(move |scroll| set((value as i32 - scroll.y.round() as i32 * 5).clamp(0, 100) as u8))
        .child(
            Rectangle::new()
                .width((TRACK * fraction).max(1.0))
                .height(22.0)
                .fill(Gradient::linear(90.0, [(0.0, color(hue)), (1.0, color(if muted { "#6f86c6" } else { AQUA }))])),
        )
}

fn toggle_button(glyph: &str, on: bool, action: impl Fn(Button) + 'static) -> Rectangle {
    let (fill, ink) = if on { (Color::WHITE, INK) } else { (theme::with_alpha(RED, 0xdd), "#ffffff") };
    Rectangle::new()
        .width(44.0)
        .height(34.0)
        .rotate(-8.0)
        .fill(fill)
        .align_child(Center, Center)
        .cursor(Pointer)
        .on_click(action)
        .child(icon(glyph, 16.0, ink))
}

fn level_row(glyph: &str, value: u8, muted: bool, accent: &str, set: fn(u8), mute: fn()) -> Row {
    Row::new(children![
        toggle_button(glyph, !muted, move |_| mute()),
        slider(value, accent, muted, set),
        heading(if muted { String::from("--") } else { value.to_string() }, 18.0, "#ffffff"),
    ])
    .gap(14.0)
    .align(Center)
}

fn audio_page(accent: &str) -> Column {
    let audio = Audio::read();

    Column::new(children![
        section("OUTPUT", accent),
        level_row(if audio.muted() { "\u{f026}" } else { "\u{f028}" }, audio.volume(), audio.muted(), accent, Audio::set_volume, Audio::toggle_mute),
        section("MICROPHONE", accent),
        level_row(
            if audio.microphone_muted() { "\u{f131}" } else { "\u{f130}" },
            audio.microphone_volume(),
            audio.microphone_muted(),
            accent,
            Audio::set_microphone_volume,
            Audio::toggle_microphone_mute,
        ),
        label("Drag or scroll a bar to change it. Click the plate to mute.", 12.0, "#6f86c6").wrap().max_lines(2),
    ])
    .gap(16.0)
    .width(Parent)
}

fn power_page(accent: &str) -> Column {
    let battery = Battery::read();
    let brightness = Brightness::read();

    let mut rows: Vec<Box<dyn Widget>> = vec![Box::new(section("BATTERY", accent))];

    if battery.present() {
        let percent = battery.percent();
        let state = if battery.full() {
            "FULLY CHARGED"
        } else if battery.charging() {
            "CHARGING"
        } else {
            "ON BATTERY"
        };
        let hue = if percent <= 15 && !battery.charging() { RED } else { accent };
        let mut cells: Vec<Box<dyn Widget>> = Vec::new();
        for step in 0..10 {
            let lit = (step as u8) * 10 < percent;
            cells.push(Box::new(
                Rectangle::new()
                    .width(26.0)
                    .height(30.0)
                    .rotate(-12.0)
                    .fill(if lit { color(hue) } else { theme::with_alpha(INK, 0xff) })
                    .border(1.0, theme::with_alpha(accent, 0x55)),
            ));
        }
        rows.push(Box::new(
            Row::new(children![heading(format!("{percent}%"), 54.0, "#ffffff"), heading(state, 18.0, hue)]).gap(16.0).align(Center),
        ));
        rows.push(Box::new(Row::new(cells).gap(6.0)));
    } else {
        rows.push(Box::new(heading("ON AC POWER", 30.0, "#ffffff")));
        rows.push(Box::new(label("No battery found on this machine.", 13.0, "#6f86c6")));
    }

    rows.push(Box::new(section("BRIGHTNESS", accent)));
    if brightness.present() {
        rows.push(Box::new(
            Row::new(children![
                Rectangle::new().width(44.0).height(34.0).align_child(Center, Center).child(icon("\u{f185}", 18.0, accent)),
                slider(brightness.percent(), accent, false, Brightness::set),
                heading(brightness.percent().to_string(), 18.0, "#ffffff"),
            ])
            .gap(14.0)
            .align(Center),
        ));
    } else {
        rows.push(Box::new(label("No backlight to control.", 13.0, "#6f86c6")));
    }

    Column::new(rows).gap(16.0).width(Parent)
}

fn network_page(settings: &Settings, accent: &str) -> Column {
    let network = Network::read();
    let wifi = network.wifi_enabled();

    let status = match network.link() {
        Link::Offline => String::from("OFFLINE"),
        Link::Wired => String::from("WIRED"),
        Link::Wifi => format!("{}  {}%", network.ssid().to_uppercase(), network.strength()),
        Link::Other => String::from("CONNECTED"),
    };
    let status = if network.connecting() { String::from("CONNECTING...") } else { status };

    let switch = Rectangle::new()
        .width(96.0)
        .height(34.0)
        .rotate(-8.0)
        .fill(if wifi { Color::WHITE } else { theme::with_alpha(RED, 0xdd) })
        .align_child(Center, Center)
        .cursor(Pointer)
        .on_click(move |_| Network::set_wifi(!wifi))
        .child(heading(if wifi { "WI-FI ON" } else { "WI-FI OFF" }, 13.0, if wifi { INK } else { "#ffffff" }));

    let rescan = Rectangle::new()
        .width(40.0)
        .height(34.0)
        .rotate(-8.0)
        .fill(theme::with_alpha(INK, 0xff))
        .border(1.5, color(accent))
        .align_child(Center, Center)
        .cursor(Pointer)
        .on_click(|_| Network::scan())
        .child(icon("\u{f021}", 14.0, accent));

    let mut rows: Vec<Box<dyn Widget>> = vec![
        Box::new(
            Row::new(children![
                Column::new(children![label("CONNECTION", 11.0, accent), heading(status, 20.0, "#ffffff").elide()]).gap(2.0).width(Parent),
                Row::new(children![rescan, switch]).gap(10.0).align(Center),
            ])
            .width(Parent)
            .justify(SpaceBetween)
            .align(Center),
        ),
        Box::new(section("NETWORKS IN RANGE", accent)),
    ];

    if let Some(ssid) = &settings.asking {
        rows.push(Box::new(password_row(ssid, accent)));
    }

    let mut list: Vec<Box<dyn Widget>> = Vec::new();
    if !wifi {
        list.push(Box::new(label("Turn Wi-Fi on to see networks.", 13.0, "#6f86c6")));
    } else if network.access_points().is_empty() {
        list.push(Box::new(label("SCANNING...", 14.0, AQUA)));
    }
    for access_point in network.access_points() {
        list.push(Box::new(access_point_row(access_point, accent)));
    }

    rows.push(Box::new(ScrollArea::new("p3r-wifi-list", Column::new(list).gap(6.0).width(Parent)).width(Parent).height(Parent)));

    Column::new(rows).gap(14.0).width(Parent).height(Parent)
}

fn access_point_row(access_point: &AccessPoint, accent: &str) -> Rectangle {
    let active = access_point.active();
    let ssid = String::from(access_point.ssid());
    let needs_password = access_point.secured() && !access_point.saved();
    let ink = if active { INK } else { "#ffffff" };
    let bars = match access_point.strength() {
        0..=25 => "▂___",
        26..=50 => "▂▄__",
        51..=75 => "▂▄▆_",
        _ => "▂▄▆█",
    };

    let mut tags: Vec<Box<dyn Widget>> = Vec::new();
    if access_point.secured() {
        tags.push(Box::new(icon("\u{f023}", 11.0, if active { INK } else { "#6f86c6" })));
    }
    tags.push(Box::new(label(bars, 12.0, if active { INK } else { accent })));

    let row = Rectangle::new()
        .width(Parent)
        .height(38.0)
        .padding(Padding { top: 0.0, right: 12.0, bottom: 0.0, left: 12.0 })
        .align_child(Start, Center)
        .cursor(Pointer)
        .on_click(move |_| {
            if active {
                Network::disconnect();
            } else if needs_password {
                Settings::write().asking = Some(ssid.clone());
                TextInput::set_text(PASSWORD, "");
            } else {
                Network::connect(&ssid, None);
            }
        })
        .child(
            Row::new(children![
                label(access_point.ssid(), 15.0, ink).elide(),
                Row::new(tags).gap(8.0).align(Center),
            ])
            .width(Parent)
            .justify(SpaceBetween)
            .align(Center),
        );

    if active {
        row.fill(color(accent)).shadow(Shadow::drop(Color::WHITE).blur(0.0).offset(4.0, 4.0))
    } else {
        row.fill(theme::with_alpha(INK, 0x99))
    }
}

fn password_row(ssid: &str, accent: &str) -> Column {
    let target = ssid.to_string();
    Column::new(children![
        label(format!("PASSWORD FOR {}", ssid.to_uppercase()), 11.0, accent),
        Rectangle::new()
            .width(Parent)
            .height(40.0)
            .rotate(-2.0)
            .fill(Color::WHITE)
            .shadow(Shadow::drop(color(accent)).blur(0.0).offset(5.0, 5.0))
            .padding(Padding { top: 0.0, right: 12.0, bottom: 0.0, left: 12.0 })
            .align_child(Start, Center)
            .child(
                TextInput::new(PASSWORD)
                    .placeholder("enter password, then Enter")
                    .password()
                    .focused()
                    .size(16.0)
                    .on_submit(move |password| {
                        Network::connect(&target, Some(&password));
                        Settings::write().asking = None;
                    }),
            ),
    ])
    .gap(6.0)
    .width(Parent)
    .align(Start)
}

