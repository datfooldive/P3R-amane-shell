use amane::{
    Center, Color, Column, Full, Gradient, LayerWindow, Lock, Monitor, Padding, Parent, Rectangle,
    Row, Service, Shadow, Stack, Start, TextInput, Widget, children,
};

use crate::clock::Clock;
use crate::theme::{self, BLOOD, DH_DEEP, DH_GREEN, DH_MOON, INK, color, heading, icon, label};

pub fn view(monitor: &Monitor) -> LayerWindow {
    let lock = Lock::read();
    let clock = Clock::read();
    let (width, height) = (monitor.width as f32, monitor.height as f32);

    let status = if lock.checking() {
        "CHECKING..."
    } else if lock.failed() {
        "WRONG PASSWORD"
    } else {
        "THE CLOCK HAS STOPPED"
    };
    let status_hue = if lock.failed() { "#ff4466" } else { DH_GREEN };

    let mut layers: Vec<Box<dyn Widget>> = vec![
        Box::new(
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .fill(Gradient::linear(180.0, [(0.0, color("#0d3a1b")), (0.55, color(DH_DEEP)), (1.0, color("#010603"))])),
        ),
        Box::new(
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .fill(Gradient::radial([(0.0, theme::with_alpha(DH_GREEN, 0x30)), (1.0, theme::with_alpha(DH_GREEN, 0x00))]))
                .translate(width * 0.25, -height * 0.35),
        ),
        Box::new(
            Rectangle::new()
                .width(height * 0.42)
                .height(height * 0.42)
                .radius(Full)
                .translate(width * 0.62, height * 0.08)
                .fill(Gradient::radial([(0.0, color("#ffffe8")), (0.7, color(DH_MOON)), (1.0, color("#c9e07a"))]))
                .shadow(Shadow::drop(color(DH_MOON)).blur(140.0).opacity(0.55)),
        ),
    ];

    for (index, (x, w)) in [(0.06, 0.22), (0.38, 0.14), (0.71, 0.26)].into_iter().enumerate() {
        layers.push(Box::new(
            Rectangle::new()
                .width(width * w)
                .height(26.0 + index as f32 * 8.0)
                .radius(Full)
                .translate(width * x, height * (0.9 + index as f32 * 0.02))
                .fill(theme::with_alpha(BLOOD, 0xaa))
                .shadow(Shadow::drop(color(BLOOD)).blur(30.0).opacity(0.6)),
        ));
    }

    layers.push(Box::new(
        Rectangle::new()
            .width(width)
            .height(200.0)
            .translate(-40.0, height * 0.16)
            .rotate(-8.0)
            .child(heading("THE DARK HOUR", 150.0, "#ffffff").color(theme::with_alpha("#ffffff", 0x22))),
    ));

    layers.push(Box::new(
        Rectangle::new()
            .width(Parent)
            .height(Parent)
            .padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: width * 0.08 })
            .align_child(Start, Center)
            .child(
                Column::new(children![
                    heading(format!("{:02}:{:02}", clock.hour, clock.minute), 120.0, DH_MOON),
                    heading(format!("{}/{:02}  {}", clock.month, clock.day, clock.weekday_name()), 34.0, "#ffffff"),
                    Rectangle::new()
                        .width(520.0)
                        .height(58.0)
                        .rotate(-4.0)
                        .fill(Color::WHITE)
                        .shadow(Shadow::drop(color(DH_GREEN)).blur(0.0).offset(9.0, 9.0))
                        .padding(Padding { top: 0.0, right: 18.0, bottom: 0.0, left: 18.0 })
                        .align_child(Start, Center)
                        .child(
                            Row::new(children![
                                icon("\u{f023}", 20.0, INK),
                                TextInput::new("p3r-password")
                                    .placeholder("password")
                                    .password()
                                    .focused()
                                    .size(22.0)
                                    .on_submit(|password| Lock::unlock(&password)),
                            ])
                            .gap(14.0)
                            .align(Center),
                        ),
                    label(status, 16.0, status_hue),
                ])
                .gap(18.0),
            ),
    ));

    // the compositor sizes lock screens to the monitor, so this size is never used
    LayerWindow::new().width(1.0).height(1.0).child(
        Rectangle::new()
            .width(Parent)
            .height(Parent)
            .fill(color(DH_DEEP))
            .align_child(Center, Center)
            .child(Stack::new(layers).width(Parent).height(Parent)),
    )
}
