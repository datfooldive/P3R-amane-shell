use std::time::Duration;

use amane::{
    Animation, Button, Center, Color, Column, Easing, Gradient, Horizontal, Image, Key,
    Keyboard, Layer, LayerWindow, Margin, Media, Padding, Parent, Pointer, Rectangle, Row,
    Service, Shadow, Vertical, Widget, children,
};

use crate::clock::Clock;
use crate::theme::{self, AQUA, DEEP, INK, NAVY, color, heading, icon, label};

const WIDTH: f32 = 470.0;
const HEIGHT: f32 = 216.0;
const ART: f32 = 150.0;

pub struct Player {
    open: bool,
    slide: Animation,
}

impl Service for Player {
    fn new() -> Self {
        Self { open: false, slide: Animation::new(0.0).duration(Duration::from_millis(260)).easing(Easing::Out) }
    }

    fn interval() -> Duration {
        Duration::from_secs(3600)
    }
}

// only one popup at a time: opening the player closes the settings panel
pub fn toggle() {
    if !Player::read().open {
        crate::settings::close();
        crate::tray_menu::close();
    }
    let mut player = Player::write();
    player.open = !player.open;
    let target = if player.open { 1.0 } else { 0.0 };
    player.slide.to(target);
}

pub fn close() {
    if !Player::read().open {
        return;
    }
    let mut player = Player::write();
    player.open = false;
    player.slide.to(0.0);
}

fn key(key: Key) {
    match key {
        Key::Escape => close(),
        Key::Space => Media::play_pause(),
        Key::Left => Media::previous(),
        Key::Right => Media::next(),
        _ => {}
    }
}

pub fn view() -> LayerWindow {
    let player = Player::read();
    let slide = player.slide.value();

    // fixed in place and never hidden, see settings::view
    let window = LayerWindow::new()
        .width(WIDTH + 20.0)
        .height(HEIGHT + 26.0)
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Left)
        .margin(Margin { top: 0, right: 0, bottom: 0, left: 16 })
        .layer(Layer::Overlay)
        .namespace("p3r-player")
        .keyboard(if player.open { Keyboard::OnDemand } else { Keyboard::None })
        .on_key(key);

    if !player.open && slide <= 0.01 {
        return window.click_through().child(Rectangle::new().width(Parent).height(Parent));
    }

    let media = Media::read();
    let accent = theme::accent(Clock::read().dark_hour());

    window
        .child(
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .padding(Padding { top: 6.0, right: 20.0, bottom: 20.0, left: 0.0 })
                .child(
                    Rectangle::new()
                        .width(WIDTH)
                        .height(HEIGHT)
                        // drops down from behind the bar
                        .translate(0.0, -(1.0 - slide) * (HEIGHT + 30.0))
                        .opacity(slide)
                        .fill(Gradient::linear(160.0, [(0.0, theme::with_alpha(NAVY, 0xf4)), (1.0, theme::with_alpha(DEEP, 0xf0))]))
                        .border(2.0, color(accent))
                        .shadow(Shadow::drop(color(accent)).blur(0.0).offset(10.0, 10.0).opacity(0.45))
                        .padding(Padding { top: 22.0, right: 22.0, bottom: 22.0, left: 26.0 })
                        .child(content(&media, accent)),
                ),
        )
}

fn content(media: &Media, accent: &str) -> Row {
    if media.title().is_empty() {
        return Row::new(children![Column::new(children![
            heading("NO TRACK", 30.0, "#ffffff"),
            label("Nothing is playing right now.", 14.0, "#8ea4dd"),
        ])
        .gap(8.0)]);
    }

    let source = media.active().map(|player| player.identity().to_uppercase()).unwrap_or_default();
    let position = media.position().as_secs();
    let length = media.length().as_secs();
    let progress = if length > 0 { (position as f32 / length as f32).clamp(0.0, 1.0) } else { 0.0 };
    let track_width = WIDTH - 48.0 - ART - 26.0;

    let time = if length > 0 {
        format!("{}:{:02} / {}:{:02}", position / 60, position % 60, length / 60, length % 60)
    } else {
        format!("{}:{:02}", position / 60, position % 60)
    };

    Row::new(children![
        art(media, accent),
        Column::new(children![
            label(source, 11.0, accent),
            heading(media.title(), 22.0, "#ffffff").elide(),
            label(media.artist(), 14.0, AQUA).elide(),
            Rectangle::new()
                .width(track_width)
                .height(6.0)
                .rotate(-1.0)
                .fill(theme::with_alpha(INK, 0xff))
                .clip()
                .child(Rectangle::new().width((track_width * progress).max(1.0)).height(6.0).fill(accent)),
            label(time, 11.0, "#8ea4dd"),
            controls(media.playing(), accent),
        ])
        .gap(6.0)
        .width(Parent),
    ])
    .gap(26.0)
    .align(Center)
}

fn art(media: &Media, accent: &str) -> Rectangle {
    let frame = Rectangle::new()
        .width(ART)
        .height(ART)
        .rotate(-4.0)
        .border(3.0, Color::WHITE)
        .shadow(Shadow::drop(color(accent)).blur(0.0).offset(7.0, 7.0));

    // players hand out file:// links; anything else can't be shown
    match media.art_url().strip_prefix("file://") {
        Some(path) if Image::loaded(path) => frame.fill(Image::cover(path.to_string()).thumbnail(ART as u32 * 2, ART as u32 * 2)),
        _ => frame.fill(theme::with_alpha(INK, 0xff)).align_child(Center, Center).child(icon("\u{f001}", 52.0, accent)),
    }
}

fn controls(playing: bool, accent: &str) -> Row {
    let button = |glyph: &'static str, big: bool, action: fn()| {
        let size = if big { 40.0 } else { 32.0 };
        let plate = Rectangle::new()
            .width(size + 8.0)
            .height(size)
            .rotate(-8.0)
            .align_child(Center, Center)
            .cursor(Pointer)
            .on_click(move |_: Button| action());
        if big {
            plate.fill(Color::WHITE).shadow(Shadow::drop(color(accent)).blur(0.0).offset(4.0, 4.0)).child(icon(glyph, 18.0, INK))
        } else {
            plate.fill(theme::with_alpha(INK, 0xee)).border(1.5, theme::with_alpha(accent, 0xaa)).child(icon(glyph, 14.0, accent))
        }
    };

    let mut row: Vec<Box<dyn Widget>> = Vec::new();
    row.push(Box::new(button("\u{f048}", false, Media::previous)));
    row.push(Box::new(button(if playing { "\u{f04c}" } else { "\u{f04b}" }, true, Media::play_pause)));
    row.push(Box::new(button("\u{f051}", false, Media::next)));

    Row::new(row).gap(14.0).align(Center)
}
