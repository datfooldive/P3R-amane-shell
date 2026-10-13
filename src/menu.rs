use std::sync::LazyLock;
use std::time::{Duration, Instant};

use amane::{
    Animation, Apps, Audio, Battery, Blend, Canvas, Cap, Center, Color, Column, Cpu, Easing, Full,
    Gradient, Image, Key, Keyboard, Layer, LayerWindow, Media, Memory, Network, Padding, Parent,
    Path, Pointer, Rectangle, Row, Service, Shadow, Shape, Stack, Start, Text, TextInput, Weight,
    Widget, Zone, children, window_size,
};

use crate::clock::{Clock, short_weekday};
use crate::hud;
use crate::theme::{self, AQUA, BLUE, DEEP, DH_DEEP, DH_MOON, DISPLAY, INK, NAVY, RED, color, heading, icon, label};

static START: LazyLock<Instant> = LazyLock::new(Instant::now);

const SEARCH: &str = "p3r-search";
// the panel's title strip plus the body's top and bottom padding
const PANEL_CHROME: f32 = 58.0 + 18.0 + 22.0;
const APP_ROW: f32 = 40.0;
const ROW_GAP: f32 = 8.0;

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Apps,
    Stats,
    Music,
    Calendar,
    System,
}

const ITEMS: [(Page, &str, &str); 5] = [
    (Page::Apps, "APPS", "LAUNCH A PROGRAM"),
    (Page::Stats, "STATS", "SOCIAL STATS"),
    (Page::Music, "MUSIC", "NOW PLAYING"),
    (Page::Calendar, "CALENDAR", "SCHEDULE"),
    (Page::System, "SYSTEM", "SESSION"),
];

const SYSTEM_ACTIONS: [(&str, &str); 4] = [
    ("\u{f023}", "LOCK"),
    ("\u{f2f5}", "LOG OUT"),
    ("\u{f021}", "REBOOT"),
    ("\u{f011}", "SHUT DOWN"),
];

pub struct Menu {
    open: bool,
    item: usize,
    inside: bool,
    sub: usize,
    query: String,
    armed: Option<usize>,
    reveal: Animation,
    cursor: Animation,
    page: Animation,
}

impl Service for Menu {
    fn new() -> Self {
        Self {
            open: false,
            item: 0,
            inside: false,
            sub: 0,
            query: String::new(),
            armed: None,
            reveal: Animation::new(0.0).duration(Duration::from_millis(420)).easing(Easing::Out),
            cursor: Animation::new(0.0).duration(Duration::from_millis(160)),
            page: Animation::new(1.0),
        }
    }

    fn interval() -> Duration {
        Duration::from_secs(3600)
    }
}

impl Menu {
    fn select(&mut self, item: usize) {
        if item == self.item {
            return;
        }
        self.item = item;
        self.sub = 0;
        self.armed = None;
        self.cursor.to(item as f32);
        self.page = Animation::new(0.0).duration(Duration::from_millis(260));
        self.page.to(1.0);
    }

    fn enter(&mut self) {
        self.inside = true;
        self.sub = 0;
        self.armed = None;
        self.query.clear();
        TextInput::set_text(SEARCH, "");
    }

    fn leave(&mut self) {
        self.inside = false;
        self.armed = None;
    }

    fn page(&self) -> Page {
        ITEMS[self.item].0
    }
}

pub fn toggle() -> String {
    if !Menu::read().open {
        crate::settings::close();
        crate::player::close();
        crate::tray_menu::close();
    }
    let mut menu = Menu::write();
    menu.open = !menu.open;
    if menu.open {
        menu.reveal.to(1.0);
        menu.leave();
        String::from("opened")
    } else {
        menu.reveal.to(0.0);
        String::from("closed")
    }
}

fn close() {
    let mut menu = Menu::write();
    menu.open = false;
    menu.reveal.to(0.0);
}

fn matching_apps(query: &str) -> Vec<usize> {
    let apps = Apps::read();
    let query = query.to_lowercase();
    apps.list()
        .iter()
        .enumerate()
        .filter(|(_, app)| query.is_empty() || app.name().to_lowercase().contains(&query))
        .map(|(index, _)| index)
        .collect()
}

fn sub_count(menu: &Menu) -> usize {
    match menu.page() {
        Page::Apps => matching_apps(&menu.query).len(),
        Page::Music => 3,
        Page::System => SYSTEM_ACTIONS.len(),
        Page::Stats | Page::Calendar => 0,
    }
}

fn activate() {
    let (page, sub, query, armed) = {
        let menu = Menu::read();
        (menu.page(), menu.sub, menu.query.clone(), menu.armed)
    };

    match page {
        Page::Apps => {
            if let Some(&index) = matching_apps(&query).get(sub) {
                let app = Apps::read().list()[index].clone();
                app.launch();
                close();
            }
        }
        Page::Music => match sub {
            0 => Media::previous(),
            1 => Media::play_pause(),
            _ => Media::next(),
        },
        Page::System => {
            // everything but lock needs a second Enter to confirm
            if sub != 0 && armed != Some(sub) {
                Menu::write().armed = Some(sub);
                return;
            }
            close();
            match sub {
                0 => amane::Lock::start(),
                1 => crate::hyprland::exit(),
                2 => amane::spawn("systemctl reboot"),
                _ => amane::spawn("systemctl poweroff"),
            }
        }
        Page::Stats | Page::Calendar => {}
    }
}

fn key(key: Key) {
    let inside = Menu::read().inside;

    if !inside {
        match key {
            Key::Up | Key::Character('w') | Key::Character('k') => {
                let mut menu = Menu::write();
                let item = (menu.item + ITEMS.len() - 1) % ITEMS.len();
                menu.select(item);
            }
            Key::Down | Key::Tab | Key::Character('s') | Key::Character('j') => {
                let mut menu = Menu::write();
                let item = (menu.item + 1) % ITEMS.len();
                menu.select(item);
            }
            Key::Enter | Key::Space | Key::Right => Menu::write().enter(),
            Key::Escape | Key::Character('q') => close(),
            _ => {}
        }
        return;
    }

    match key {
        Key::Up | Key::Down | Key::Tab => {
            let mut menu = Menu::write();
            let count = sub_count(&menu);
            if count > 0 {
                menu.sub = if key == Key::Up { (menu.sub + count - 1) % count } else { (menu.sub + 1) % count };
                menu.armed = None;
            }
        }
        Key::Enter | Key::Space => activate(),
        Key::Escape | Key::Left => Menu::write().leave(),
        _ => {}
    }
}

pub fn view() -> LayerWindow {
    let menu = Menu::read();
    let clock = Clock::read();
    let reveal = menu.reveal.value();
    let shown = menu.open || reveal > 0.01;

    let (mut width, mut height) = window_size();
    if width < 1.0 || height < 1.0 {
        (width, height) = (1920.0, 1080.0);
    }

    // full screen, never moved or hidden, see settings::view
    let window = LayerWindow::new()
        .width(Full)
        .height(Full)
        .layer(Layer::Overlay)
        .space(Zone::Ignore)
        .namespace("p3r-menu")
        .keyboard(if menu.open { Keyboard::Exclusive } else { Keyboard::None })
        .on_key(key);

    if !shown {
        return window.click_through().child(Rectangle::new().width(Parent).height(Parent));
    }

    // the waves animate for as long as the menu is up
    amane::request_frame();

    let dark = clock.dark_hour();
    let accent = theme::accent(dark);
    let (base, mid) = if dark { (DH_DEEP, "#0b3b1d") } else { (NAVY, DEEP) };
    let slide = 1.0 - reveal;

    let mut layers: Vec<Box<dyn Widget>> = Vec::new();

    layers.push(Box::new(
        Rectangle::new()
            .width(Parent)
            .height(Parent)
            .fill(Gradient::linear(120.0, [(0.0, color(base)), (0.55, color(mid)), (1.0, color(if dark { "#14572b" } else { BLUE }))]))
            .opacity(reveal * 0.94)
            .on_click(|_| close()),
    ));

    layers.push(Box::new(waves(width, height, accent, reveal)));

    layers.push(Box::new(
        Rectangle::new()
            .width(width * 1.4)
            .height(260.0)
            .translate(-width * 0.12 - slide * 400.0, height * 0.58)
            .rotate(-14.0)
            .child(Text::new(clock.period()).font(DISPLAY).weight(Weight::Black).size(230.0).color(theme::with_alpha("#ffffff", (18.0 * reveal) as u8))),
    ));

    layers.push(Box::new(
        Rectangle::new()
            .width(width * 1.5)
            .height(height * 0.5)
            .translate(-width * 0.55 - slide * width * 0.6, height * 0.2)
            .rotate(-16.0)
            .fill(Gradient::linear(90.0, [(0.0, theme::with_alpha(INK, 0xee)), (0.62, theme::with_alpha(INK, 0xaa)), (1.0, theme::with_alpha(INK, 0x00))]))
            .opacity(reveal),
    ));
    layers.push(Box::new(
        Rectangle::new()
            .width(width * 1.5)
            .height(8.0)
            .translate(-width * 0.5 - slide * width, height * 0.73)
            .rotate(-16.0)
            .fill(accent)
            .opacity(reveal * 0.85),
    ));

    for (index, entry) in ITEMS.iter().enumerate() {
        layers.push(Box::new(menu_item(index, entry, &menu, width, height, reveal, accent)));
    }

    layers.push(Box::new(page_panel(&menu, &clock, width, height, reveal, accent)));

    layers.push(Box::new(
        hud::calendar(&clock)
            .translate(width - 330.0 + slide * 360.0, 40.0)
            .scale(1.25),
    ));

    layers.push(Box::new(prompts(&menu, accent).translate(width - 560.0, height - 70.0 + slide * 120.0)));

    window.child(Stack::new(layers).width(Parent).height(Parent))
}

fn mix(from: &str, to: &str, amount: f32) -> Color {
    Color::blend(color(from), color(to), amount.clamp(0.0, 1.0))
}

// the cascade of entries shrinks on narrow screens so the page panel never has to overlap it
fn item_width(width: f32) -> f32 {
    (width * 0.29).clamp(360.0, 470.0)
}

// right edge of the furthest entry: last step, the focus nudge, its 1.1 scale and the tilt's reach
fn items_right(width: f32) -> f32 {
    70.0 + (ITEMS.len() - 1) as f32 * 38.0 + 34.0 + item_width(width) * 1.05 + 12.0
}

fn menu_item(index: usize, entry: &(Page, &str, &str), menu: &Menu, width: f32, height: f32, reveal: f32, accent: &str) -> Rectangle {
    let (_, name, caption) = *entry;
    let item_width = item_width(width);
    let type_scale = item_width / 470.0;
    let focus = (1.0 - (index as f32 - menu.cursor.value()).abs()).clamp(0.0, 1.0);
    let stagger = (1.0 - reveal) * (700.0 + index as f32 * 140.0);
    let x = 70.0 + index as f32 * 38.0 + focus * 34.0 - stagger;
    let y = height * 0.2 + index as f32 * 92.0;

    let plate = mix(INK, "#ffffff", focus);
    let plate = if focus < 0.01 { Color::TRANSPARENT } else { Color::rgba(plate.red(), plate.green(), plate.blue(), (focus * 255.0) as u8) };
    let ink = mix("#ffffff", INK, focus);

    Rectangle::new()
        .width(item_width)
        .height(76.0)
        .translate(x, y)
        .rotate(-9.0)
        .scale(1.0 + focus * 0.1)
        .fill(plate)
        .shadow(Shadow::drop(color(accent)).blur(0.0).offset(10.0 * focus, 9.0 * focus).opacity(focus))
        .padding(Padding { top: 0.0, right: 18.0, bottom: 0.0, left: 22.0 })
        .align_child(Start, Center)
        .cursor(Pointer)
        .on_hover(move |inside| {
            if inside && !Menu::read().inside {
                Menu::write().select(index);
            }
        })
        .on_click(move |_| {
            let mut menu = Menu::write();
            menu.select(index);
            menu.enter();
        })
        .child(
            Row::new(children![
                Text::new(format!("{:02}", index + 1)).font(DISPLAY).weight(Weight::Black).size(18.0).color(mix(accent, BLUE, focus)),
                Text::new(name).font(DISPLAY).weight(Weight::Black).size(46.0 * type_scale).color(ink),
                Text::new(caption).font(DISPLAY).weight(Weight::Bold).size(12.0).color(theme::with_alpha(if focus > 0.5 { BLUE } else { AQUA }, (80.0 + focus * 175.0) as u8)),
            ])
            .gap(16.0)
            .align(Center),
        )
}

fn waves(width: f32, height: f32, accent: &str, reveal: f32) -> Canvas {
    let time = START.elapsed().as_secs_f32();
    let mut lines = Vec::new();

    for layer in 0..4 {
        let depth = layer as f32;
        let base = height * (0.62 + depth * 0.09);
        let amplitude = 14.0 + depth * 9.0;
        let speed = 0.5 + depth * 0.22;
        let mut path = Path::new().move_to(0.0, base);
        let mut x = 0.0;
        while x <= width + 40.0 {
            let y = base
                + (x / 210.0 + time * speed).sin() * amplitude
                + (x / 90.0 - time * speed * 1.7).sin() * amplitude * 0.3
                - x * 0.25;
            path = path.line_to(x, y);
            x += 24.0;
        }
        let hue = if layer % 2 == 0 { accent } else { AQUA };
        lines.push(path.stroke(2.0 + depth * 0.6, hue).cap(Cap::Round).opacity((0.32 - depth * 0.06) * reveal));
    }

    let mut shapes: Vec<Box<dyn Shape>> = Vec::new();
    for line in lines {
        shapes.push(Box::new(line));
    }
    Canvas::new().width(width).height(height).shapes(shapes)
}

fn page_panel(menu: &Menu, clock: &Clock, width: f32, height: f32, reveal: f32, accent: &str) -> Rectangle {
    let page = menu.page.value();
    // starts past the furthest menu entry (plus room for the -3° tilt) and stops short of the right edge
    let left = (width * 0.53).max(items_right(width) + 28.0);
    let panel_width = (width * 0.4).clamp(460.0, 720.0).min(width - left - 48.0);
    let panel_height = (height * 0.64).clamp(460.0, 720.0);
    let x = left + (1.0 - page) * 90.0 + (1.0 - reveal) * width * 0.5;
    let y = height * 0.2;
    let (_, title, caption) = ITEMS[menu.item];
    let room = panel_height - PANEL_CHROME;

    let body: Box<dyn Widget> = match menu.page() {
        Page::Apps => Box::new(apps_page(menu, accent, room)),
        Page::Stats => Box::new(stats_page(accent)),
        Page::Music => Box::new(music_page(menu, accent)),
        Page::Calendar => Box::new(calendar_page(clock, accent, room)),
        Page::System => Box::new(system_page(menu, accent)),
    };

    let border = if menu.inside { color(accent) } else { theme::with_alpha(accent, 0x55) };

    Rectangle::new()
        .width(panel_width)
        .height(panel_height)
        .translate(x, y)
        .rotate(-3.0)
        .opacity(page * reveal)
        .fill(theme::with_alpha(INK, 0xe8))
        .border(2.0, border)
        .shadow(Shadow::drop(color(accent)).blur(0.0).offset(12.0, 12.0).opacity(0.35))
        .child(
            Column::new(children![
                Rectangle::new()
                    .width(Parent)
                    .height(58.0)
                    .fill(Gradient::linear(90.0, [(0.0, Color::WHITE), (0.7, Color::WHITE), (1.0, theme::with_alpha("#ffffff", 0x00))]))
                    .padding(Padding { top: 0.0, right: 20.0, bottom: 0.0, left: 22.0 })
                    .align_child(Start, Center)
                    .child(Row::new(children![heading(title, 32.0, INK), label(caption, 13.0, BLUE)]).gap(16.0).align(Center)),
                Rectangle::new().width(Parent).height(Parent).padding(Padding { top: 18.0, right: 26.0, bottom: 22.0, left: 26.0 }).clip().child(Column::new(vec![body]).width(Parent)),
            ])
            .width(Parent)
            .height(Parent),
        )
}

fn selectable(selected: bool, accent: &str, height: f32) -> Rectangle {
    let plate = Rectangle::new().width(Parent).height(height).padding(Padding { top: 0.0, right: 14.0, bottom: 0.0, left: 14.0 }).align_child(Start, Center).cursor(Pointer);
    if selected {
        plate.fill(accent).shadow(Shadow::drop(Color::WHITE).blur(0.0).offset(5.0, 4.0)).translate(14.0, 0.0)
    } else {
        plate.fill(theme::with_alpha(DEEP, 0x66))
    }
}

fn apps_page(menu: &Menu, accent: &str, room: f32) -> Column {
    let apps = Apps::read();
    let matches = matching_apps(&menu.query);

    let search: Box<dyn Widget> = if menu.inside {
        Box::new(
            TextInput::new(SEARCH)
                .placeholder("type to search")
                .size(18.0)
                .color(Color::WHITE)
                .focused()
                .on_change(|text| {
                    let mut menu = Menu::write();
                    menu.query = text;
                    menu.sub = 0;
                })
                .on_submit(|_| activate()),
        )
    } else {
        Box::new(label("PRESS ENTER TO SEARCH", 15.0, "#6f86c6"))
    };

    let mut rows: Vec<Box<dyn Widget>> = vec![Box::new(
        Rectangle::new()
            .width(Parent)
            .height(42.0)
            .fill(theme::with_alpha(NAVY, 0xff))
            .border(1.5, theme::with_alpha(accent, 0x99))
            .padding(Padding { top: 0.0, right: 14.0, bottom: 0.0, left: 14.0 })
            .align_child(Start, Center)
            .child(Row::new(vec![Box::new(icon("\u{f002}", 16.0, accent)) as Box<dyn Widget>, search]).gap(12.0).align(Center)),
    )];

    if apps.list().is_empty() {
        rows.push(Box::new(label("SCANNING...", 16.0, AQUA)));
    }

    // whatever fits under the search box and above the counter, so the last row never spills out
    let visible = (((room - 42.0 - 20.0) / (APP_ROW + ROW_GAP)).floor() as usize).max(1);
    let first = menu.sub.saturating_sub(visible / 2).min(matches.len().saturating_sub(visible));
    for (offset, &index) in matches.iter().enumerate().skip(first).take(visible) {
        let app = &apps.list()[index];
        let selected = menu.inside && offset == menu.sub;
        let icon_box = Rectangle::new().width(26.0).height(26.0);
        let icon_box = match app.icon_path() {
            // shrunk once at twice the drawn size, instead of scaling a 512px icon down every frame
            Some(path) => icon_box.fill(Image::contain(path).thumbnail(52, 52)),
            None => icon_box.fill(theme::with_alpha(accent, 0x44)).radius(Full),
        };
        let launched = app.clone();
        rows.push(Box::new(
            selectable(selected, accent, APP_ROW)
                .on_click(move |_| {
                    launched.launch();
                    close();
                })
                .child(Row::new(children![icon_box, label(app.name(), 17.0, if selected { INK } else { "#ffffff" }).elide()]).gap(14.0).align(Center)),
        ));
    }

    if !matches.is_empty() {
        rows.push(Box::new(label(format!("{} / {}", (menu.sub + 1).min(matches.len()), matches.len()), 12.0, "#6f86c6")));
    }

    Column::new(rows).gap(ROW_GAP).width(Parent)
}

// system load mapped onto the game's social stat ranks
fn stats_page(accent: &str) -> Column {
    let cpu = Cpu::read();
    let memory = Memory::read();
    let battery = Battery::read();
    let audio = Audio::read();
    let network = Network::read();

    let courage = if battery.present() { battery.percent() } else { 100 };
    let gib = |kib: u64| kib as f32 / 1_048_576.0;

    let link = if network.connected() {
        if network.ssid().is_empty() { String::from("WIRED") } else { network.ssid().to_uppercase() }
    } else {
        String::from("NO SIGNAL")
    };

    Column::new(children![
        stat("ACADEMICS", format!("CPU {}%", cpu.percent()), cpu.percent(), ["SLACKER", "AVERAGE", "ABOVE AVERAGE", "SMART", "INTELLIGENT", "GENIUS"], accent),
        stat(
            "CHARM",
            format!("MEMORY {:.1} / {:.1} GIB", gib(memory.used_kib()), gib(memory.total_kib())),
            memory.percent(),
            ["PLAIN", "UNPOLISHED", "CONFIDENT", "SMOOTH", "POPULAR", "CHARISMATIC"],
            accent,
        ),
        stat(
            "COURAGE",
            if battery.present() { format!("BATTERY {}%{}", battery.percent(), if battery.charging() { " +" } else { "" }) } else { String::from("ON AC POWER") },
            courage,
            ["TIMID", "ORDINARY", "DETERMINED", "TOUGH", "FEARLESS", "BADASS"],
            accent,
        ),
        Row::new(children![
            label(format!("VOLUME {}", if audio.muted() { String::from("MUTED") } else { audio.volume().to_string() }), 14.0, AQUA),
            label(format!("LINK  {link}"), 14.0, AQUA),
        ])
        .gap(30.0),
    ])
    .gap(22.0)
    .width(Parent)
}

fn stat(name: &str, detail: String, percent: u8, ranks: [&str; 6], accent: &str) -> Column {
    let rank = ((percent as usize * 6) / 101).min(5);
    let mut blocks: Vec<Box<dyn Widget>> = Vec::new();
    for step in 0..6 {
        let lit = step <= rank;
        blocks.push(Box::new(
            Rectangle::new()
                .width(46.0)
                .height(16.0)
                .rotate(-12.0)
                .fill(if lit { color(accent) } else { theme::with_alpha(DEEP, 0xaa) })
                .border(1.0, theme::with_alpha(accent, 0x66)),
        ));
    }

    Column::new(children![
        Row::new(children![label(name, 14.0, accent), label(detail, 12.0, "#8ea4dd")]).gap(14.0).align(Center),
        Row::new(children![
            heading(format!("LV {}", rank + 1), 18.0, "#ffffff"),
            heading(ranks[rank], 30.0, "#ffffff"),
        ])
        .gap(14.0)
        .align(Center),
        Row::new(blocks).gap(6.0),
    ])
    .gap(6.0)
}

fn music_page(menu: &Menu, accent: &str) -> Column {
    let media = Media::read();

    if media.title().is_empty() {
        return Column::new(children![
            heading("NO TRACK", 34.0, "#ffffff"),
            label("Nothing is playing. Start a player and it shows up here.", 15.0, "#8ea4dd").wrap().max_lines(2),
        ])
        .gap(10.0)
        .width(Parent);
    }

    let position = media.position().as_secs();
    let length = media.length().as_secs().max(1);
    let progress = (position as f32 / length as f32).clamp(0.0, 1.0);

    let art: Box<dyn Widget> = match media.art_url().strip_prefix("file://") {
        Some(path) => Box::new(
            Rectangle::new()
                .width(150.0)
                .height(150.0)
                .rotate(-4.0)
                .fill(Image::cover(path.to_string()))
                .border(3.0, Color::WHITE)
                .shadow(Shadow::drop(color(accent)).blur(0.0).offset(8.0, 8.0)),
        ),
        None => Box::new(
            Rectangle::new()
                .width(150.0)
                .height(150.0)
                .rotate(-4.0)
                .fill(theme::with_alpha(DEEP, 0xff))
                .border(3.0, color(accent))
                .align_child(Center, Center)
                .child(icon("\u{f001}", 56.0, accent)),
        ),
    };

    let actions = [
        ("\u{f048}", "PREVIOUS"),
        (if media.playing() { "\u{f04c}" } else { "\u{f04b}" }, if media.playing() { "PAUSE" } else { "PLAY" }),
        ("\u{f051}", "NEXT"),
    ];
    let mut options: Vec<Box<dyn Widget>> = Vec::new();
    for (index, (glyph, name)) in actions.into_iter().enumerate() {
        let selected = menu.inside && menu.sub == index;
        let ink = if selected { INK } else { "#ffffff" };
        options.push(Box::new(
            selectable(selected, accent, 38.0)
                .on_click(move |_| match index {
                    0 => Media::previous(),
                    1 => Media::play_pause(),
                    _ => Media::next(),
                })
                .child(Row::new(children![icon(glyph, 15.0, ink), heading(name, 17.0, ink)]).gap(12.0).align(Center)),
        ));
    }

    Column::new(children![
        Row::new(vec![
            art,
            Box::new(Column::new(children![
                heading(media.title(), 28.0, "#ffffff").wrap().max_lines(2).elide(),
                label(media.artist(), 16.0, accent).elide(),
                label(format!("{}:{:02} / {}:{:02}", position / 60, position % 60, length / 60, length % 60), 13.0, "#8ea4dd"),
                Rectangle::new()
                    .width(Parent)
                    .height(8.0)
                    .rotate(-1.0)
                    .fill(theme::with_alpha(DEEP, 0xff))
                    .clip()
                    .child(Rectangle::new().width(300.0 * progress).height(8.0).fill(accent)),
            ])
            .gap(8.0)
            .width(Parent)),
        ])
        .gap(26.0)
        .width(Parent),
        Column::new(options).gap(8.0).width(Parent),
    ])
    .gap(26.0)
    .width(Parent)
}

fn calendar_page(clock: &Clock, accent: &str, room: f32) -> Column {
    const MONTHS: [&str; 12] = ["JANUARY", "FEBRUARY", "MARCH", "APRIL", "MAY", "JUNE", "JULY", "AUGUST", "SEPTEMBER", "OCTOBER", "NOVEMBER", "DECEMBER"];
    let synodic = 29.530_588_853_f32;

    let mut header: Vec<Box<dyn Widget>> = Vec::new();
    for weekday in 1..=7 {
        let hue = match weekday {
            7 => RED,
            6 => "#5b8cff",
            _ => accent,
        };
        header.push(Box::new(Rectangle::new().width(62.0).height(22.0).align_child(Center, Center).child(label(short_weekday(weekday), 12.0, hue))));
    }

    // month title, weekday header, the moon line and the gaps between them take ~118px; six weeks share the rest
    let cell_height = ((room - 118.0) / 6.0 - 6.0).clamp(26.0, 46.0);

    let mut weeks: Vec<Box<dyn Widget>> = vec![Box::new(Row::new(header).gap(6.0))];
    let lead = clock.first_weekday.saturating_sub(1);
    let mut day: i32 = 1 - lead as i32;

    while day <= clock.days_in_month as i32 {
        let mut cells: Vec<Box<dyn Widget>> = Vec::new();
        for column in 0..7 {
            let cell = Rectangle::new().width(62.0).height(cell_height).align_child(Center, Center);
            if day < 1 || day > clock.days_in_month as i32 {
                cells.push(Box::new(cell));
                day += 1;
                continue;
            }

            let phase = (clock.moon_phase() + (day - clock.day as i32) as f32 / synodic).rem_euclid(1.0);
            let full = (phase - 0.5).abs() < 0.5 / synodic;
            let today = day == clock.day as i32;
            let past = day < clock.day as i32;
            let hue = if today {
                INK
            } else if column == 6 {
                RED
            } else if past {
                "#4a5f99"
            } else {
                "#ffffff"
            };

            let number = heading(day.to_string(), 20.0, hue);
            let content: Box<dyn Widget> = if full {
                Box::new(Column::new(children![number, hud::moon(10.0, 0.5, DH_MOON, INK)]).gap(2.0).align(Center))
            } else {
                Box::new(number)
            };

            let cell = if today {
                cell.fill(Color::WHITE).rotate(-8.0).shadow(Shadow::drop(color(accent)).blur(0.0).offset(4.0, 4.0))
            } else if full {
                cell.fill(theme::with_alpha(DH_MOON, 0x22)).border(1.0, theme::with_alpha(DH_MOON, 0x88))
            } else {
                cell.fill(theme::with_alpha(DEEP, 0x55))
            };
            cells.push(Box::new(cell.child(Column::new(vec![content]).align(Center))));
            day += 1;
        }
        weeks.push(Box::new(Row::new(cells).gap(6.0)));
    }

    let days = clock.days_to_full();
    let countdown = match days {
        0 => String::from("THE FULL MOON IS TONIGHT"),
        1 => String::from("1 DAY UNTIL THE FULL MOON"),
        _ => format!("{days} DAYS UNTIL THE FULL MOON"),
    };

    Column::new(children![
        Row::new(children![heading(MONTHS[(clock.month.max(1) - 1) as usize], 30.0, "#ffffff"), heading(clock.year.to_string(), 30.0, accent)]).gap(12.0),
        Column::new(weeks).gap(6.0),
        Row::new(children![hud::moon(22.0, clock.moon_phase(), DH_MOON, INK), label(format!("{}  ·  {countdown}", clock.moon_name()), 14.0, AQUA)]).gap(12.0).align(Center),
    ])
    .gap(16.0)
}

fn system_page(menu: &Menu, accent: &str) -> Column {
    let mut rows: Vec<Box<dyn Widget>> = Vec::new();

    for (index, (glyph, name)) in SYSTEM_ACTIONS.into_iter().enumerate() {
        let selected = menu.inside && menu.sub == index;
        let armed = menu.armed == Some(index);
        let hue = if armed { RED } else { accent };
        let ink = if selected { INK } else { "#ffffff" };
        let note = if armed { "PRESS AGAIN TO CONFIRM" } else { "" };

        rows.push(Box::new(
            selectable(selected, hue, 52.0)
                .on_click(move |_| {
                    let mut menu = Menu::write();
                    menu.inside = true;
                    menu.sub = index;
                    drop(menu);
                    activate();
                })
                .child(Row::new(children![icon(glyph, 20.0, ink), heading(name, 24.0, ink), label(note, 12.0, if selected { INK } else { RED })]).gap(16.0).align(Center)),
        ));
    }

    let user = std::env::var("USER").unwrap_or_default().to_uppercase();
    rows.push(Box::new(label(format!("SIGNED IN AS {user}"), 12.0, "#6f86c6")));

    Column::new(rows).gap(12.0).width(Parent)
}

fn prompts(menu: &Menu, accent: &str) -> Rectangle {
    let pairs: [(&str, &str); 3] = if menu.inside {
        [("↑↓", "SELECT"), ("ENTER", "CONFIRM"), ("ESC", "BACK")]
    } else {
        [("↑↓", "SELECT"), ("ENTER", "OPEN"), ("ESC", "CLOSE")]
    };

    let mut items: Vec<Box<dyn Widget>> = Vec::new();
    for (keycap, action) in pairs {
        items.push(Box::new(
            Row::new(children![
                Rectangle::new()
                    .width(if keycap.len() > 3 { 64.0 } else { 44.0 })
                    .height(26.0)
                    .radius(Full)
                    .fill(Color::WHITE)
                    .align_child(Center, Center)
                    .child(heading(keycap, 12.0, INK)),
                heading(action, 15.0, "#ffffff"),
            ])
            .gap(8.0)
            .align(Center),
        ));
    }

    Rectangle::new()
        .width(520.0)
        .height(44.0)
        .rotate(-3.0)
        .fill(theme::with_alpha(INK, 0xcc))
        .border(1.5, theme::with_alpha(accent, 0x88))
        .align_child(Center, Center)
        .child(Row::new(items).gap(22.0).align(Center))
}
