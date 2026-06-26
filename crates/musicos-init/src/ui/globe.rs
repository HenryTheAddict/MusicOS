use crate::renderer::{Canvas, Color};

pub fn draw_real_globe(
    canvas: &mut Canvas<'_>,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    thick: f32,
    color: Color,
) {
    let map_x = |x_svg: f32| -> f32 { cx + (x_svg - 69.0) * (rx / 64.0) };
    let map_y = |y_svg: f32| -> f32 { cy + (y_svg - 49.5) * (ry / 44.5) };

    // 1. C 37 94, 5 74.0767, 5 49.5
    canvas.bezier_curve(
        map_x(69.0),
        map_y(94.0),
        map_x(37.0),
        map_y(94.0),
        map_x(5.0),
        map_y(74.0767),
        map_x(5.0),
        map_y(49.5),
        thick,
        color,
    );

    // 2. C 101 94, 133 74.0767, 133 49.5
    canvas.bezier_curve(
        map_x(69.0),
        map_y(94.0),
        map_x(101.0),
        map_y(94.0),
        map_x(133.0),
        map_y(74.0767),
        map_x(133.0),
        map_y(49.5),
        thick,
        color,
    );

    // 3. C 69 59.2433, 69 39.7567, 69 5
    canvas.bezier_curve(
        map_x(69.0),
        map_y(94.0),
        map_x(69.0),
        map_y(59.2433),
        map_x(69.0),
        map_y(39.7567),
        map_x(69.0),
        map_y(5.0),
        thick,
        color,
    );

    // 4a. C 81.8 87.325, 101 71.75, 101 49.5
    canvas.bezier_curve(
        map_x(69.0),
        map_y(94.0),
        map_x(81.8),
        map_y(87.325),
        map_x(101.0),
        map_y(71.75),
        map_x(101.0),
        map_y(49.5),
        thick,
        color,
    );
    // 4b. C 101 27.25, 81.4968 22.3783, 69 5
    canvas.bezier_curve(
        map_x(101.0),
        map_y(49.5),
        map_x(101.0),
        map_y(27.25),
        map_x(81.4968),
        map_y(22.3783),
        map_x(69.0),
        map_y(5.0),
        thick,
        color,
    );

    // 5a. C 56.5032 76.6217, 37 71.75, 37 49.5
    canvas.bezier_curve(
        map_x(69.0),
        map_y(94.0),
        map_x(56.5032),
        map_y(76.6217),
        map_x(37.0),
        map_y(71.75),
        map_x(37.0),
        map_y(49.5),
        thick,
        color,
    );
    // 5b. C 37 27.25, 56.5032 22.3783, 69 5
    canvas.bezier_curve(
        map_x(37.0),
        map_y(49.5),
        map_x(37.0),
        map_y(27.25),
        map_x(56.5032),
        map_y(22.3783),
        map_x(69.0),
        map_y(5.0),
        thick,
        color,
    );

    // 6. C 5 24.9233, 37 5, 69 5
    canvas.bezier_curve(
        map_x(5.0),
        map_y(49.5),
        map_x(5.0),
        map_y(24.9233),
        map_x(37.0),
        map_y(5.0),
        map_x(69.0),
        map_y(5.0),
        thick,
        color,
    );

    // 7. H 133
    canvas.line(
        map_x(5.0),
        map_y(49.5),
        map_x(133.0),
        map_y(49.5),
        thick,
        color,
    );

    // 8. C 109 5, 133 24.9233, 133 49.5
    canvas.bezier_curve(
        map_x(69.0),
        map_y(5.0),
        map_x(109.0),
        map_y(5.0),
        map_x(133.0),
        map_y(24.9233),
        map_x(133.0),
        map_y(49.5),
        thick,
        color,
    );
}
