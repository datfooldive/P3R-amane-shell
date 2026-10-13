use std::env;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::thread;

/*
 * Hyprland 0.56 reads `dispatch` as Lua, so amane's built-in `dispatch workspace <id>`
 * is rejected; this sends the hl.dsp form instead, off the draw thread
 */
fn dispatch(lua: String) {
    thread::spawn(move || {
        let (Some(runtime), Some(instance)) = (env::var_os("XDG_RUNTIME_DIR"), env::var_os("HYPRLAND_INSTANCE_SIGNATURE")) else {
            return;
        };
        let path = PathBuf::from(runtime).join("hypr").join(instance).join(".socket.sock");
        let Ok(mut stream) = UnixStream::connect(path) else {
            return;
        };
        if stream.write_all(format!("dispatch {lua}").as_bytes()).is_ok() {
            let mut reply = String::new();
            let _ = stream.read_to_string(&mut reply);
        }
    });
}

// named and special workspaces have negative ids, so those go by name
pub fn focus_workspace(id: i64, name: Option<&str>) {
    let target = match name {
        Some(name) if id < 0 => format!("\"name:{}\"", name.replace(['"', '\\'], "")),
        _ => id.to_string(),
    };
    dispatch(format!("hl.dsp.focus({{ workspace = {target} }})"));
}

pub fn exit() {
    dispatch(String::from("hl.dsp.exit()"));
}
