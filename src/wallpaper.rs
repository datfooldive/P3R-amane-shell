use amane::{
    Canvas, Circle, Full, Gradient, Layer, LayerWindow, Monitor, Parent, Rectangle, Shape, Stack,
    Widget, Zone, window_size,
};

use crate::theme::{self, AQUA, BLUE, CYAN, DEEP, color};

const STARS: usize = 70;

// cheap stable noise in 0..1, so the stars land in the same places on every redraw
fn hash(seed: usize, salt: f32) -> f32 {
    ((seed as f32 * 12.9898 + salt * 78.233).sin() * 43_758.547).fract().abs()
}

// kept faint and away from the top edge, so the bar and windows stay readable over it
pub fn view(monitor: &Monitor) -> LayerWindow {
    let (mut width, mut height) = window_size();
    if width < 1.0 || height < 1.0 {
        (width, height) = (monitor.width as f32, monitor.height as f32);
    }

    let layers: Vec<Box<dyn Widget>> = vec![
        Box::new(
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .fill(Gradient::linear(180.0, [(0.0, color("#050b1f")), (1.0, color("#010205"))])),
        ),
        Box::new(
            Rectangle::new()
                .width(width * 1.6)
                .height(height * 0.42)
                .translate(-width * 0.3, height * 0.52)
                .rotate(-16.0)
                .fill(Gradient::linear(90.0, [(0.0, theme::with_alpha(DEEP, 0x00)), (0.5, theme::with_alpha(DEEP, 0x66)), (1.0, theme::with_alpha(DEEP, 0x10))])),
        ),
        Box::new(
            Rectangle::new()
                .width(width * 1.6)
                .height(3.0)
                .translate(-width * 0.3, height * 0.5)
                .rotate(-16.0)
                .fill(Gradient::linear(90.0, [(0.0, theme::with_alpha(CYAN, 0x00)), (0.6, theme::with_alpha(CYAN, 0x55)), (1.0, theme::with_alpha(CYAN, 0x00))])),
        ),
        Box::new(
            Rectangle::new()
                .width(width * 1.6)
                .height(height * 0.08)
                .translate(-width * 0.3, height * 0.98)
                .rotate(-16.0)
                .fill(theme::with_alpha(BLUE, 0x18)),
        ),
        Box::new(sky(width, height)),
    ];

    LayerWindow::new()
        .width(Full)
        .height(Full)
        .layer(Layer::Background)
        .space(Zone::Ignore)
        .namespace("p3r-wallpaper")
        .click_through()
        .child(Stack::new(layers).width(Parent).height(Parent))
}

// a hollow moon ring low on the right and a sprinkle of faint stars
fn sky(width: f32, height: f32) -> Canvas {
    let ring = height * 0.3;
    let (cx, cy) = (width * 0.8, height * 0.72);

    let mut shapes: Vec<Box<dyn Shape>> = vec![
        Box::new(Circle::new().center(cx, cy).radius(ring).fill(theme::with_alpha(DEEP, 0x30))),
        Box::new(Circle::new().center(cx, cy).radius(ring).stroke(2.0, AQUA).opacity(0.14)),
        Box::new(Circle::new().center(cx, cy).radius(ring * 1.12).stroke(1.0, AQUA).opacity(0.07)),
    ];

    for index in 0..STARS {
        let x = hash(index, 1.0) * width;
        // nothing in the strip under the bar
        let y = height * (0.15 + hash(index, 2.0) * 0.8);
        let size = 0.6 + hash(index, 3.0) * 1.4;
        let glow = 0.08 + hash(index, 4.0) * 0.22;
        shapes.push(Box::new(Circle::new().center(x, y).radius(size).fill(AQUA).opacity(glow)));
    }

    Canvas::new().width(width).height(height).shapes(shapes)
}
