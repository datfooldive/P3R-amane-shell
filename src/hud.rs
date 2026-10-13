use amane::{
    Audio, Battery, Button, Canvas, Center, Color, Column, Full, Gradient, Image, Layer, LayerWindow, Media,
    Monitor, Network, Padding, Parent, Path, Pointer, Rectangle, Row, Service, Shadow, Shape, Stack,
    SpaceBetween, Vertical, Widget, Workspace, Workspaces, Zone, children, shapes,
};

use crate::clock::Clock;
use crate::hyprland;
use crate::menu;
use crate::tray::{self, Tray};
use crate::tray_menu;
use crate::player;
use crate::settings::{self, Tab};
use crate::visualizer::{self, Visualizer};
use crate::theme::{self, AQUA, BLUE, DEEP, DH_DEEP, DH_MOON, INK, NAVY, RED, color, heading, icon, label};

pub const HEIGHT: f32 = 84.0;

pub fn view(monitor: &Monitor) -> LayerWindow {
    let clock = Clock::read();
    let dark = clock.dark_hour();
    let top = if dark { DH_DEEP } else { NAVY };

    LayerWindow::new()
        .width(Full)
        .height(HEIGHT)
        .anchor_vertical(Vertical::Top)
        .layer(Layer::Top)
        .space(Zone::Reserve)
        .namespace("p3r-hud")
        .child(
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .fill(Gradient::linear(
                    180.0,
                    [(0.0, theme::with_alpha(top, 0xf0)), (0.75, theme::with_alpha(top, 0x90)), (1.0, theme::with_alpha(top, 0x00))],
                ))
                .padding(Padding { top: 4.0, right: 22.0, bottom: 4.0, left: 22.0 })
                .child(
                    Row::new(children![
                        workspaces(monitor, dark),
                        now_playing(dark),
                        Row::new(children![status(dark), calendar(&clock)]).gap(26.0).align(Center),
                    ])
                    .width(Parent)
                    .height(Parent)
                    .justify(SpaceBetween)
                    .align(Center),
                ),
        )
}

fn workspaces(monitor: &Monitor, dark: bool) -> Row {
    let workspaces = Workspaces::read();
    let mut tiles: Vec<Box<dyn Widget>> = Vec::new();

    for workspace in workspaces.list() {
        if workspace.output().is_some_and(|output| output != monitor.name) {
            continue;
        }
        tiles.push(Box::new(workspace_tile(workspace, dark)));
    }

    Row::new(tiles).gap(12.0).align(Center)
}

fn workspace_tile(workspace: &Workspace, dark: bool) -> Rectangle {
    let accent = theme::accent(dark);
    let id = workspace.id();
    let name = workspace.name().map(String::from);
    let number = format!("{:02}", workspace.index());

    let tile = Rectangle::new()
        .width(50.0)
        .height(46.0)
        .rotate(-10.0)
        .align_child(Center, Center)
        .cursor(Pointer)
        .on_click(move |_| hyprland::focus_workspace(id, name.as_deref()));

    if workspace.focused() {
        tile.fill(Color::WHITE)
            .shadow(Shadow::drop(color(accent)).blur(0.0).offset(5.0, 5.0))
            .scale(1.12)
            .child(heading(number, 22.0, INK))
    } else if workspace.urgent() {
        tile.fill(RED).child(heading(number, 20.0, "#ffffff"))
    } else {
        let text = if workspace.windows() > 0 { "#ffffff" } else { "#6f86c6" };
        tile.fill(theme::with_alpha(DEEP, 0xcc))
            .border(2.0, theme::with_alpha(accent, 0x99))
            .child(heading(number, 18.0, text))
    }
}

fn now_playing(dark: bool) -> Rectangle {
    let media = Media::read();
    // fills the space between the workspaces and the status chips, packed to the left
    let plate = Rectangle::new()
        .width(Parent)
        .height(Parent)
        .padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 26.0 })
        .align_child(amane::Start, Center);

    if media.title().is_empty() {
        return plate;
    }

    let accent = theme::accent(dark);
    let playing = media.playing();
    let glyph = if playing { "\u{f001}" } else { "\u{f04c}" };

    // only a playing track subscribes to cava, so a paused bar never redraws at its frame rate
    let levels = if playing { Visualizer::read().levels } else { [0; visualizer::BARS] };

    let mut bars: Vec<Box<dyn Widget>> = Vec::new();
    for (index, level) in levels.iter().enumerate() {
        let height = 3.0 + *level as f32 / 100.0 * 26.0;
        // brighter toward the middle, like a sound wave swelling
        let middle = 1.0 - (index as f32 - (visualizer::BARS as f32 - 1.0) / 2.0).abs() / (visualizer::BARS as f32 / 2.0);
        let alpha = if playing { 0x88 + (middle * 119.0) as u8 } else { 0x55 };
        bars.push(Box::new(
            Rectangle::new()
                .width(5.0)
                .height(height)
                .radius(Full)
                .fill(Gradient::linear(180.0, [(0.0, theme::with_alpha(AQUA, alpha)), (0.5, theme::with_alpha(accent, alpha)), (1.0, theme::with_alpha(AQUA, alpha))])),
        ));
    }

    plate.child(
        Rectangle::new()
            .width(270.0)
            .height(40.0)
            .rotate(-2.0)
            .fill(theme::with_alpha(INK, 0xcc))
            .border(1.5, theme::with_alpha(accent, 0x88))
            .padding(Padding { top: 0.0, right: 14.0, bottom: 0.0, left: 14.0 })
            .align_child(amane::Start, Center)
            .cursor(Pointer)
            .on_click(|button| match button {
                Button::Right => Media::next(),
                Button::Middle => Media::play_pause(),
                Button::Left => player::toggle(),
            })
            .child(
                Row::new(children![icon(glyph, 14.0, accent), Row::new(bars).gap(3.0).height(34.0).align(Center)])
                    .gap(12.0)
                    .align(Center),
            ),
    )
}

fn status(dark: bool) -> Row {
    let accent = theme::accent(dark);
    let battery = Battery::read();
    let audio = Audio::read();
    let network = Network::read();

    let mut chips: Vec<Box<dyn Widget>> = Vec::new();

    if let Some(plate) = tray_plate(accent) {
        chips.push(Box::new(plate));
    }

    if battery.present() {
        let glyph = if battery.charging() { "\u{f0e7}" } else { "\u{f240}" };
        let hue = if battery.percent() <= 15 && !battery.charging() { RED } else { accent };
        chips.push(Box::new(
            chip(glyph, format!("{}", battery.percent()), hue)
                .cursor(Pointer)
                .on_click(|_| settings::show(Tab::Battery)),
        ));
    }

    let (glyph, level) = if audio.muted() { ("\u{f026}", String::from("MUTE")) } else { ("\u{f028}", audio.volume().to_string()) };
    chips.push(Box::new(
        chip(glyph, level, accent)
            .cursor(Pointer)
            .on_click(|button| match button {
                Button::Middle => Audio::toggle_mute(),
                _ => settings::show(Tab::Audio),
            })
            .on_scroll(|scroll| {
                let volume = Audio::read().volume() as i32;
                Audio::set_volume((volume - scroll.y.round() as i32 * 5).clamp(0, 100) as u8);
            }),
    ));

    let net = if network.connected() { "ONLINE" } else { "OFFLINE" };
    chips.push(Box::new(
        chip("\u{f1eb}", net.to_string(), if network.connected() { accent } else { RED })
            .cursor(Pointer)
            .on_click(|_| settings::show(Tab::Network)),
    ));

    Row::new(chips).gap(8.0).align(Center)
}

// app tray icons, in the same slanted plate as the status chips; passive items stay hidden
fn tray_plate(accent: &str) -> Option<Rectangle> {
    let tray = Tray::read();
    let items: Vec<&tray::Item> = tray.items.iter().filter(|item| item.status != "Passive").collect();
    if items.is_empty() {
        return None;
    }

    let mut icons: Vec<Box<dyn Widget>> = Vec::new();
    for item in &items {
        let glyph: Box<dyn Widget> = match &item.icon {
            Some(path) => Box::new(Rectangle::new().width(18.0).height(18.0).fill(Image::contain(path.clone()).thumbnail(36, 36))),
            None => Box::new(heading(item.title.chars().next().unwrap_or('?').to_uppercase().to_string(), 13.0, "#ffffff")),
        };
        let clicked = (*item).clone();
        let cell = Rectangle::new()
            .width(26.0)
            .height(26.0)
            .rotate(8.0)
            .align_child(Center, Center)
            .cursor(Pointer)
            .on_click(move |button| match button {
                Button::Left if clicked.item_is_menu && clicked.menu.is_some() => tray_menu::open(&clicked),
                Button::Left => tray::activate(&clicked, "Activate"),
                Button::Right if clicked.menu.is_some() => tray_menu::open(&clicked),
                Button::Right => tray::activate(&clicked, "ContextMenu"),
                Button::Middle => tray::activate(&clicked, "SecondaryActivate"),
            })
            .child(Column::new(vec![glyph]));
        let cell = if item.status == "NeedsAttention" { cell.fill(theme::with_alpha(RED, 0x99)).radius(6.0) } else { cell };
        icons.push(Box::new(cell));
    }

    Some(
        Rectangle::new()
            .width(items.len() as f32 * 30.0 + 14.0)
            .height(30.0)
            .rotate(-8.0)
            .fill(theme::with_alpha(INK, 0xee))
            .border(1.5, theme::with_alpha(accent, 0xcc))
            .radius_bottom_right(10.0)
            .align_child(Center, Center)
            .child(Row::new(icons).gap(4.0).align(Center)),
    )
}

fn chip(glyph: &str, value: String, hue: &str) -> Rectangle {
    Rectangle::new()
        .width(if value.len() > 4 { 104.0 } else { 74.0 })
        .height(28.0)
        .rotate(-8.0)
        .fill(theme::with_alpha(INK, 0xee))
        .border(1.5, theme::with_alpha(hue, 0xcc))
        .radius_bottom_right(10.0)
        .align_child(Center, Center)
        .child(Row::new(children![icon(glyph, 13.0, hue), label(value, 13.0, "#ffffff")]).gap(7.0).align(Center))
}

pub fn calendar(clock: &Clock) -> Rectangle {
    let dark = clock.dark_hour();
    let accent = theme::accent(dark);
    let date = format!("{}/{:02}", clock.month, clock.day);
    let weekday_fill = match clock.weekday {
        7 => RED,
        6 => BLUE,
        _ => accent,
    };
    let weekday_text = if clock.weekday >= 6 { "#ffffff" } else { INK };
    let (lit, shade) = if dark { (DH_MOON, DH_DEEP) } else { (AQUA, INK) };

    let block = Stack::new(children![
        // offset copy behind the date for the layered look
        Rectangle::new().width(150.0).height(54.0).translate(4.0, 3.0).child(heading(date.clone(), 46.0, if dark { "#0d5a2c" } else { BLUE })),
        Rectangle::new().width(150.0).height(54.0).child(heading(date, 46.0, "#ffffff")),
        Rectangle::new()
            .width(62.0)
            .height(26.0)
            .translate(140.0, 8.0)
            .rotate(-12.0)
            .fill(weekday_fill)
            .align_child(Center, Center)
            .child(heading(clock.weekday_name(), 16.0, weekday_text)),
        moon(40.0, clock.moon_phase(), lit, shade).translate(212.0, 2.0),
        Rectangle::new()
            .width(250.0)
            .height(22.0)
            .translate(6.0, 52.0)
            .rotate(-2.5)
            .fill(Gradient::linear(90.0, [(0.0, color(if dark { "#0d5a2c" } else { BLUE })), (1.0, theme::with_alpha(DEEP, 0x00))]))
            .padding(Padding { top: 0.0, right: 10.0, bottom: 0.0, left: 10.0 })
            .align_child(amane::Start, Center)
            .child(
                Row::new(children![
                    heading(clock.period(), 13.0, "#ffffff"),
                    label(format!("{:02}:{:02}", clock.hour, clock.minute), 13.0, accent),
                ])
                .gap(12.0)
                .align(Center),
            ),
    ])
    .width(258.0)
    .height(76.0);

    Rectangle::new()
        .width(258.0)
        .height(76.0)
        .cursor(Pointer)
        .on_click(|_| {
            menu::toggle();
        })
        .child(block)
}

/*
 * the dark disc is the rectangle itself and only the lit part is drawn on top, bounded by
 * the limb on one side and the terminator (a half ellipse) on the other; nothing lit sits
 * under the dark side, so no bright fringe can show through its edge
 */
pub fn moon(diameter: f32, phase: f32, lit: &str, shade: &str) -> Rectangle {
    const STEPS: usize = 48;
    let radius = diameter / 2.0;
    // waxing moons are lit on the right, waning ones on the left
    let side = if phase <= 0.5 { 1.0 } else { -1.0 };
    let terminator = (phase * std::f32::consts::TAU).cos();

    let point = |angle: f32, width: f32| (radius + side * width * radius * angle.sin(), radius - radius * angle.cos());

    let (x, y) = point(0.0, 1.0);
    let mut path = Path::new().move_to(x, y);
    for step in 1..=STEPS {
        let (x, y) = point(std::f32::consts::PI * step as f32 / STEPS as f32, 1.0);
        path = path.line_to(x, y);
    }
    for step in (0..STEPS).rev() {
        let (x, y) = point(std::f32::consts::PI * step as f32 / STEPS as f32, terminator);
        path = path.line_to(x, y);
    }

    Rectangle::new()
        .width(diameter)
        .height(diameter)
        .radius(Full)
        .fill(shade)
        .shadow(Shadow::drop(color(lit)).blur(diameter * 0.35).opacity(0.55))
        .child(Canvas::new().width(diameter).height(diameter).shapes(shapes![path.close().fill(lit)]))
}
