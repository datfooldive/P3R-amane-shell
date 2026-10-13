use std::thread;
use std::time::Duration;

use amane::{
    Animation, Center, Color, Column, Easing, Gradient, Horizontal, InputArea, Key, Keyboard, Layer,
    LayerWindow, Margin, Padding, Parent, Pointer, Rectangle, Row, ScrollArea, Service, Shadow,
    SpaceBetween, Start, Vertical, Widget, children,
};

use crate::clock::Clock;
use crate::theme::{self, DEEP, INK, NAVY, color, heading, icon, label};
use crate::tray::{self, Entry, Item};

const WIDTH: f32 = 300.0;
const MAX_HEIGHT: f32 = 460.0;
const HEADER: f32 = 46.0;
const ROW: f32 = 34.0;
const SEPARATOR: f32 = 11.0;
// lines the popup up under the tray icons, which sit left of the status chips and the calendar
const RIGHT: i32 = 560;

pub struct TrayMenu {
    open: bool,
    slide: Animation,
    bus: String,
    path: String,
    title: String,
    loading: bool,
    entries: Vec<Entry>,
    // the menus above the one shown, for going back out of a submenu
    trail: Vec<(String, Vec<Entry>)>,
}

impl Service for TrayMenu {
    fn new() -> Self {
        Self {
            open: false,
            slide: Animation::new(0.0).duration(Duration::from_millis(220)).easing(Easing::Out),
            bus: String::new(),
            path: String::new(),
            title: String::new(),
            loading: false,
            entries: Vec::new(),
            trail: Vec::new(),
        }
    }

    fn interval() -> Duration {
        Duration::from_secs(3600)
    }
}

pub fn open(item: &Item) {
    let Some(path) = item.menu.clone() else { return };

    // the same item again closes its menu, like the other popups
    {
        let menu = TrayMenu::read();
        if menu.open && menu.bus == item.bus && menu.path == path {
            drop(menu);
            close();
            return;
        }
    }

    crate::settings::close();
    crate::player::close();

    {
        let mut menu = TrayMenu::write();
        menu.open = true;
        menu.slide.to(1.0);
        menu.bus = item.bus.clone();
        menu.path = path.clone();
        menu.title = item.title.to_uppercase();
        menu.loading = true;
        menu.entries.clear();
        menu.trail.clear();
    }

    let bus = item.bus.clone();
    thread::spawn(move || {
        let entries = tray::load_menu(&bus, &path);
        let mut menu = TrayMenu::write();
        // only if this menu is still the one asked for
        if menu.bus == bus && menu.path == path {
            menu.entries = entries;
            menu.loading = false;
        }
    });
}

pub fn close() {
    if !TrayMenu::read().open {
        return;
    }
    let mut menu = TrayMenu::write();
    menu.open = false;
    menu.slide.to(0.0);
}

fn back() {
    let mut menu = TrayMenu::write();
    if let Some((title, entries)) = menu.trail.pop() {
        menu.title = title;
        menu.entries = entries;
    }
}

fn key(key: Key) {
    if key == Key::Escape || key == Key::Left {
        if TrayMenu::read().trail.is_empty() {
            close();
        } else {
            back();
        }
    }
}

fn rows_height(entries: &[Entry], has_back: bool) -> f32 {
    let rows: f32 = entries.iter().map(|entry| if entry.separator { SEPARATOR } else { ROW }).sum();
    rows + if has_back { ROW } else { 0.0 } + if entries.is_empty() { ROW } else { 0.0 }
}

pub fn view() -> LayerWindow {
    let menu = TrayMenu::read();
    let slide = menu.slide.value();
    let shown = menu.open || slide > 0.01;
    let accent = theme::accent(Clock::read().dark_hour());

    let list_height = rows_height(&menu.entries, !menu.trail.is_empty()).min(MAX_HEIGHT - HEADER - 16.0);
    let panel_height = HEADER + list_height + 16.0;

    // fixed size and never hidden, see settings::view; only the panel itself takes the mouse
    let window = LayerWindow::new()
        .width(WIDTH + 14.0)
        .height(MAX_HEIGHT + 20.0)
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Right)
        .layer(Layer::Overlay)
        .margin(Margin { top: 6, right: RIGHT, bottom: 0, left: 0 })
        .namespace("p3r-tray-menu")
        .keyboard(if menu.open { Keyboard::OnDemand } else { Keyboard::None })
        .on_key(key);

    if !shown {
        return window.click_through().child(Rectangle::new().width(Parent).height(Parent));
    }

    let mut rows: Vec<Box<dyn Widget>> = Vec::new();

    if !menu.trail.is_empty() {
        rows.push(Box::new(
            row()
                .on_click(|_| back())
                .child(Row::new(children![icon("\u{f053}", 11.0, accent), label("BACK", 13.0, accent)]).gap(10.0).align(Center)),
        ));
    }

    if menu.entries.is_empty() {
        let note = if menu.loading { "LOADING..." } else { "NO MENU" };
        rows.push(Box::new(Rectangle::new().width(Parent).height(ROW).padding(12.0).align_child(Start, Center).child(label(note, 13.0, "#6f86c6"))));
    }

    for entry in &menu.entries {
        rows.push(Box::new(entry_row(entry, &menu, accent)));
    }

    window.input_region(vec![InputArea { x: 0, y: 0, width: (WIDTH + 14.0) as i32, height: (panel_height + 14.0) as i32 }]).child(
        Rectangle::new()
            .width(Parent)
            .height(Parent)
            .padding(Padding { top: 0.0, right: 14.0, bottom: 0.0, left: 0.0 })
            .child(
                Rectangle::new()
                    .width(WIDTH)
                    .height(panel_height)
                    .translate(0.0, -(1.0 - slide) * 30.0)
                    .opacity(slide)
                    .fill(Gradient::linear(160.0, [(0.0, theme::with_alpha(NAVY, 0xf6)), (1.0, theme::with_alpha(DEEP, 0xf2))]))
                    .border(2.0, color(accent))
                    .shadow(Shadow::drop(color(accent)).blur(0.0).offset(8.0, 8.0).opacity(0.45))
                    .clip()
                    .child(
                        Column::new(children![
                            Rectangle::new()
                                .width(Parent)
                                .height(HEADER)
                                .fill(Gradient::linear(90.0, [(0.0, Color::WHITE), (0.75, Color::WHITE), (1.0, theme::with_alpha("#ffffff", 0x00))]))
                                .padding(Padding { top: 0.0, right: 14.0, bottom: 0.0, left: 16.0 })
                                .align_child(Start, Center)
                                .child(heading(menu.title.clone(), 18.0, INK).elide()),
                            Rectangle::new()
                                .width(Parent)
                                .height(list_height + 16.0)
                                .padding(8.0)
                                .child(ScrollArea::new("p3r-tray-menu", Column::new(rows).width(Parent)).height(list_height)),
                        ])
                        .width(Parent),
                    ),
            ),
    )
}

fn row() -> Rectangle {
    Rectangle::new()
        .width(Parent)
        .height(ROW)
        .padding(Padding { top: 0.0, right: 12.0, bottom: 0.0, left: 12.0 })
        .align_child(Start, Center)
        .cursor(Pointer)
}

fn entry_row(entry: &Entry, menu: &TrayMenu, accent: &str) -> Rectangle {
    if entry.separator {
        return Rectangle::new()
            .width(Parent)
            .height(SEPARATOR)
            .align_child(Center, Center)
            .child(Rectangle::new().width(Parent).height(1.0).fill(theme::with_alpha(accent, 0x44)));
    }

    let ink = if entry.enabled { "#ffffff" } else { "#4a5f99" };
    let mark: Box<dyn Widget> = match entry.checked {
        Some(true) => Box::new(icon("\u{f00c}", 11.0, accent)),
        Some(false) => Box::new(Rectangle::new().width(11.0).height(11.0).border(1.0, theme::with_alpha(accent, 0x88))),
        None => Box::new(Rectangle::new().width(11.0).height(11.0)),
    };
    let tail: Box<dyn Widget> = if entry.children.is_empty() {
        Box::new(Rectangle::new().width(1.0).height(1.0))
    } else {
        Box::new(icon("\u{f054}", 10.0, accent))
    };

    let base = row().child(
        Row::new(vec![Box::new(Row::new(vec![mark, Box::new(label(entry.label.clone(), 13.0, ink).elide())]).gap(10.0).align(Center).width(Parent)), tail])
            .width(Parent)
            .justify(SpaceBetween)
            .align(Center),
    );

    if !entry.enabled {
        return base;
    }

    let (bus, path, id) = (menu.bus.clone(), menu.path.clone(), entry.id);
    let submenu = (!entry.children.is_empty()).then(|| (entry.label.to_uppercase(), entry.children.clone()));

    base.on_click(move |_| match &submenu {
        Some((title, children)) => {
            let mut menu = TrayMenu::write();
            let previous = (std::mem::take(&mut menu.title), std::mem::take(&mut menu.entries));
            menu.trail.push(previous);
            menu.title = title.clone();
            menu.entries = children.clone();
        }
        None => {
            tray::click(&bus, &path, id);
            close();
        }
    })
}
