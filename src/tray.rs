use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::thread;

use amane::{Argument, Bus, Service, Value, arguments};

const WATCHER: &str = "org.kde.StatusNotifierWatcher";
const WATCHER_PATH: &str = "/StatusNotifierWatcher";
const ITEM: &str = "org.kde.StatusNotifierItem";
const PROPERTIES: &str = "org.freedesktop.DBus.Properties";
const MENU: &str = "com.canonical.dbusmenu";

const INTROSPECTION: &str = r#"<node>
  <interface name="org.kde.StatusNotifierWatcher">
    <method name="RegisterStatusNotifierItem"><arg name="service" type="s" direction="in"/></method>
    <method name="RegisterStatusNotifierHost"><arg name="service" type="s" direction="in"/></method>
    <property name="RegisteredStatusNotifierItems" type="as" access="read"/>
    <property name="IsStatusNotifierHostRegistered" type="b" access="read"/>
    <property name="ProtocolVersion" type="i" access="read"/>
    <signal name="StatusNotifierItemRegistered"><arg type="s"/></signal>
    <signal name="StatusNotifierItemUnregistered"><arg type="s"/></signal>
    <signal name="StatusNotifierHostRegistered"/>
  </interface>
</node>"#;

#[derive(Clone, PartialEq)]
pub struct Item {
    pub bus: String,
    pub path: String,
    // the unique name (:1.42) that signals come from, even when `bus` is a well-known name
    owner: String,
    pub title: String,
    pub status: String,
    pub icon: Option<String>,
    pub menu: Option<String>,
    pub item_is_menu: bool,
}

impl Item {
    fn service(&self) -> String {
        format!("{}{}", self.bus, self.path)
    }
}

pub struct Tray {
    pub items: Vec<Item>,
}

/*
 * the bar is the tray host and also the watcher apps register with; amane's dbus
 * wrapper has no sender on method calls, so items that register with only a path
 * are matched to their owner by asking every connection for that object
 */
impl Service for Tray {
    fn new() -> Self {
        Self { items: Vec::new() }
    }

    fn listen() {
        let bus = Bus::session();
        let calls = bus.methods(WATCHER_PATH, WATCHER);
        let properties = bus.methods(WATCHER_PATH, PROPERTIES);
        let introspect = bus.methods(WATCHER_PATH, "org.freedesktop.DBus.Introspectable");

        if !bus.own(WATCHER) {
            eprintln!("amane tray: another program already runs the tray watcher");
            return;
        }

        thread::spawn(move || {
            for method in properties {
                let name = method.arguments().get(1).map(|value| value.text().to_string()).unwrap_or_default();
                match method.name() {
                    "Get" => method.reply(&[Argument::Variant(Box::new(watcher_property(&name)))]),
                    "GetAll" => {
                        let mut map = std::collections::BTreeMap::new();
                        for key in ["RegisteredStatusNotifierItems", "IsStatusNotifierHostRegistered", "ProtocolVersion"] {
                            map.insert(key.to_string(), watcher_property(key));
                        }
                        method.reply(&[Argument::Map(map)]);
                    }
                    _ => method.reply(&[]),
                }
            }
        });

        thread::spawn(move || {
            for method in introspect {
                method.reply(&arguments![INTROSPECTION]);
            }
        });

        for signal in ["NewIcon", "NewAttentionIcon", "NewStatus", "NewTitle", "NewMenu"] {
            thread::spawn(move || {
                for event in Bus::session().signals(ITEM, signal) {
                    let (sender, path) = (event.sender().to_string(), event.path().to_string());
                    let known = Self::read().items.iter().any(|item| item.owner == sender && item.path == path);
                    if known {
                        refresh(&sender, &path);
                    }
                }
            });
        }

        // an app that quits takes its item with it
        thread::spawn(|| {
            for event in Bus::session().signals("org.freedesktop.DBus", "NameOwnerChanged") {
                let [name, old, new] = event.arguments() else { continue };
                if !new.text().is_empty() {
                    continue;
                }
                let gone: Vec<String> = Self::read()
                    .items
                    .iter()
                    .filter(|item| item.owner == old.text() || item.bus == name.text())
                    .map(Item::service)
                    .collect();
                if gone.is_empty() {
                    continue;
                }
                Self::write().items.retain(|item| !gone.contains(&item.service()));
                for service in gone {
                    Bus::session().emit(WATCHER_PATH, WATCHER, "StatusNotifierItemUnregistered", &arguments![service]);
                }
            }
        });

        bus.emit(WATCHER_PATH, WATCHER, "StatusNotifierHostRegistered", &[]);

        for method in calls {
            let service = method.arguments().first().map(|value| value.text().to_string()).unwrap_or_default();
            // answer first: some apps wait on the reply before they finish setting the item up
            method.reply(&[]);
            if method.name() == "RegisterStatusNotifierItem" && !service.is_empty() {
                thread::spawn(move || register(&service));
            }
        }
    }
}

fn watcher_property(name: &str) -> Argument {
    match name {
        "RegisteredStatusNotifierItems" => Argument::TextList(Tray::read().items.iter().map(Item::service).collect()),
        "IsStatusNotifierHostRegistered" => Argument::Bool(true),
        _ => Argument::Int(0),
    }
}

fn register(service: &str) {
    let (bus, path) = if service.starts_with('/') {
        match owner_of_path(service) {
            Some(owner) => (owner, service.to_string()),
            None => return,
        }
    } else {
        (service.to_string(), String::from("/StatusNotifierItem"))
    };

    refresh(&bus, &path);
    Bus::session().emit(WATCHER_PATH, WATCHER, "StatusNotifierItemRegistered", &arguments![format!("{bus}{path}")]);
}

fn owner_of_path(path: &str) -> Option<String> {
    let bus = Bus::session();
    let names = bus.call("org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus", "ListNames", &[]);
    names
        .list()
        .iter()
        .map(Value::text)
        .filter(|name| name.starts_with(':'))
        .find(|name| !matches!(bus.call(name, path, PROPERTIES, "Get", &arguments![ITEM, "Id"]), Value::Nothing))
        .map(String::from)
}

// reads every property of an item again and replaces (or adds) it
fn refresh(bus_name: &str, path: &str) {
    let bus = Bus::session();
    let properties = bus.call(bus_name, path, PROPERTIES, "GetAll", &arguments![ITEM]);
    if matches!(properties, Value::Nothing) {
        return;
    }

    let owner = if bus_name.starts_with(':') {
        bus_name.to_string()
    } else {
        bus.call("org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus", "GetNameOwner", &arguments![bus_name]).text().to_string()
    };

    let text = |key: &str| properties.get(key).text().to_string();
    let status = text("Status");
    let theme_path = text("IconThemePath");
    let attention = status == "NeedsAttention";

    let icon = if attention {
        resolve_icon(&text("AttentionIconName"), &theme_path, properties.get("AttentionIconPixmap"))
    } else {
        None
    }
    .or_else(|| resolve_icon(&text("IconName"), &theme_path, properties.get("IconPixmap")));

    let title = [text("Title"), text("Id")].into_iter().find(|name| !name.is_empty()).unwrap_or_default();
    let menu = Some(text("Menu")).filter(|menu| menu.starts_with('/') && menu != "/");

    let item = Item {
        bus: bus_name.to_string(),
        path: path.to_string(),
        owner,
        title,
        status,
        icon,
        menu,
        item_is_menu: properties.get("ItemIsMenu").bool(),
    };

    let mut tray = Tray::write();
    match tray.items.iter_mut().find(|known| known.service() == item.service() || (known.owner == item.owner && known.path == item.path)) {
        Some(known) if *known == item => {}
        Some(known) => *known = item,
        None => tray.items.push(item),
    }
}

// clicks talk to the app off the draw thread
pub fn activate(item: &Item, method: &'static str) {
    let (bus, path) = (item.bus.clone(), item.path.clone());
    thread::spawn(move || {
        Bus::session().call(&bus, &path, ITEM, method, &arguments![0, 0]);
    });
}

/* ---------- icons ---------- */

fn resolve_icon(name: &str, theme_path: &str, pixmap: &Value) -> Option<String> {
    if !name.is_empty() {
        if name.starts_with('/') && Path::new(name).exists() {
            return Some(name.to_string());
        }
        if !theme_path.is_empty() {
            if let Some(path) = find_in(Path::new(theme_path), name, 0) {
                return Some(path.to_string_lossy().into_owned());
            }
        }
        if let Some(path) = ICONS.get(name) {
            return Some(path.to_string_lossy().into_owned());
        }
    }
    write_pixmap(pixmap)
}

fn find_in(folder: &Path, name: &str, depth: u32) -> Option<PathBuf> {
    if depth > 4 {
        return None;
    }
    for entry in fs::read_dir(folder).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_in(&path, name, depth + 1) {
                return Some(found);
            }
        } else if path.file_stem().is_some_and(|stem| stem == name)
            && path.extension().is_some_and(|extension| extension == "png" || extension == "svg")
        {
            return Some(path);
        }
    }
    None
}

// every icon of the user's theme chain by name, best size first; built once, on the tray's thread
static ICONS: LazyLock<HashMap<String, PathBuf>> = LazyLock::new(|| {
    let home = std::env::var("HOME").unwrap_or_default();
    let roots = [format!("{home}/.local/share/icons"), String::from("/usr/share/icons")];

    let user_theme = amane::output("gsettings get org.gnome.desktop.interface icon-theme").trim().trim_matches('\'').to_string();
    let mut themes = Vec::new();
    for theme in [user_theme.as_str(), "Papirus", "Adwaita", "hicolor"] {
        add_theme(theme, &roots, &mut themes);
    }

    let mut icons: HashMap<String, (usize, u32, PathBuf)> = HashMap::new();
    for (rank, theme) in themes.iter().enumerate() {
        for root in &roots {
            collect(Path::new(&format!("{root}/{theme}")), rank, 0, 0, &mut icons);
        }
    }
    collect(Path::new("/usr/share/pixmaps"), themes.len(), 0, 0, &mut icons);

    icons.into_iter().map(|(name, (_, _, path))| (name, path)).collect()
});

// a theme, then the themes it inherits from, depth first like the icon theme spec
fn add_theme(theme: &str, roots: &[String], themes: &mut Vec<String>) {
    if theme.is_empty() || themes.iter().any(|known| known == theme) || themes.len() >= 8 {
        return;
    }
    themes.push(theme.to_string());
    let inherits = roots
        .iter()
        .find_map(|root| fs::read_to_string(format!("{root}/{theme}/index.theme")).ok())
        .and_then(|index| index.lines().find_map(|line| line.strip_prefix("Inherits=").map(String::from)))
        .unwrap_or_default();
    for parent in inherits.split(',') {
        add_theme(parent.trim(), roots, themes);
    }
}

fn collect(folder: &Path, rank: usize, size: u32, depth: u32, icons: &mut HashMap<String, (usize, u32, PathBuf)>) {
    if depth > 4 {
        return;
    }
    let Ok(entries) = fs::read_dir(folder) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            // folders like 48x48 or 48x48@2x carry the size, scalable beats every size
            let folder_size = if file_name == "scalable" {
                1000
            } else {
                file_name.split('x').next().and_then(|number| number.parse().ok()).unwrap_or(size)
            };
            collect(&path, rank, folder_size, depth + 1, icons);
            continue;
        }
        let Some((stem, extension)) = file_name.rsplit_once('.') else { continue };
        if extension != "png" && extension != "svg" {
            continue;
        }
        let score = if extension == "svg" { size.max(500) } else { size };
        let better = icons.get(stem).is_none_or(|(known_rank, known_score, _)| rank < *known_rank || (rank == *known_rank && score > *known_score));
        if better {
            icons.insert(stem.to_string(), (rank, score, path));
        }
    }
}

// IconPixmap is a list of (width, height, ARGB bytes); the largest one becomes a png on disk
fn write_pixmap(pixmap: &Value) -> Option<String> {
    let best = pixmap.list().iter().max_by_key(|entry| entry.list().first().map(|width| width.number() as u32).unwrap_or(0))?;
    let [width, height, data] = best.list() else { return None };
    let (width, height) = (width.number() as u32, height.number() as u32);
    let argb: Vec<u8> = data.list().iter().map(|byte| byte.number() as u8).collect();
    if width == 0 || height == 0 || argb.len() < (width * height * 4) as usize {
        return None;
    }

    let mut rgba = Vec::with_capacity(argb.len());
    for pixel in argb.chunks_exact(4) {
        rgba.extend_from_slice(&[pixel[1], pixel[2], pixel[3], pixel[0]]);
    }

    // named by content, because amane keeps images by path and a new icon needs a new path
    let hash = rgba.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3));
    let folder = format!("{}/amane-tray", std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| String::from("/tmp")));
    let path = format!("{folder}/{hash:016x}.png");
    if !Path::new(&path).exists() {
        fs::create_dir_all(&folder).ok()?;
        fs::write(&path, png(width, height, &rgba)).ok()?;
    }
    Some(path)
}

// an uncompressed png: stored deflate blocks are valid zlib, and tray icons are tiny
fn png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity(rgba.len() + height as usize);
    for row in rgba.chunks_exact(width as usize * 4) {
        raw.push(0);
        raw.extend_from_slice(row);
    }

    let mut zlib = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65_535).collect();
    for (index, block) in blocks.iter().enumerate() {
        zlib.push(u8::from(index + 1 == blocks.len()));
        let length = block.len() as u16;
        zlib.extend_from_slice(&length.to_le_bytes());
        zlib.extend_from_slice(&(!length).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for byte in &raw {
        a = (a + u32::from(*byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    zlib.extend_from_slice(&((b << 16) | a).to_be_bytes());

    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);

    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    for (kind, data) in [(b"IHDR", header), (b"IDAT", zlib), (b"IEND", Vec::new())] {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let start = out.len();
        out.extend_from_slice(kind);
        out.extend_from_slice(&data);
        let crc = crc32(&out[start..]);
        out.extend_from_slice(&crc.to_be_bytes());
    }
    out
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 { (crc >> 1) ^ 0xedb8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/* ---------- menus (com.canonical.dbusmenu) ---------- */

#[derive(Clone)]
pub struct Entry {
    pub id: i32,
    pub label: String,
    pub enabled: bool,
    pub separator: bool,
    pub checked: Option<bool>,
    pub children: Vec<Entry>,
}

fn parse(node: &Value) -> Option<Entry> {
    let [id, properties, children] = node.list() else { return None };
    let flag = |key: &str| match properties.get(key) {
        Value::Nothing => true,
        value => value.bool(),
    };
    if !flag("visible") {
        return None;
    }
    let checked = match properties.get("toggle-type").text() {
        "checkmark" | "radio" => Some(properties.get("toggle-state").number() as i32 == 1),
        _ => None,
    };
    Some(Entry {
        id: id.number() as i32,
        // underscores mark keyboard mnemonics, a doubled one is a literal underscore
        label: properties.get("label").text().replace("__", "\u{0}").replace('_', "").replace('\u{0}', "_"),
        enabled: flag("enabled"),
        separator: properties.get("type").text() == "separator",
        checked,
        children: children.list().iter().filter_map(parse).collect(),
    })
}

// waits on the app, so call it off the draw thread
pub fn load_menu(bus: &str, path: &str) -> Vec<Entry> {
    let session = Bus::session();
    session.call(bus, path, MENU, "AboutToShow", &arguments![0]);
    let reply = session.call(bus, path, MENU, "GetLayout", &[Argument::Int(0), Argument::Int(-1), Argument::TextList(Vec::new())]);
    reply.list().get(1).and_then(parse).map(|root| root.children).unwrap_or_default()
}

pub fn click(bus: &str, path: &str, id: i32) {
    let (bus, path) = (bus.to_string(), path.to_string());
    thread::spawn(move || {
        Bus::session().call(
            &bus,
            &path,
            MENU,
            "Event",
            &[Argument::Int(id), Argument::Text(String::from("clicked")), Argument::Variant(Box::new(Argument::Int(0))), Argument::Unsigned(0)],
        );
    });
}
