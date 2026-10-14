#include "../Components.hpp"
#include <iostream>
#include <cmath>

namespace MusicUI {

// 9. Sparklist Modal
SparklistModal::SparklistModal(const Rect& b) : Widget(b) {
    candidates = {
        { "Plastic Beach", "Gorillaz", "assets/covers/Beach.png", 1 },
        { "Instant Crush", "Daft Punk", "assets/covers/Apart.png", 2 },
        { "The Less I Know", "Tame Impala", "assets/covers/Hills.png", 0 }
    };
}

void SparklistModal::render_self(Canvas& canvas) {
    // Gradient background: #0064FF -> #0034FF
    LinearGradient bg_grad(
        Point(bounds.x, bounds.y),
        Point(bounds.x, bounds.bottom()),
        { GradientStop(0.0f, Color::from_hex(0x0064FF)), GradientStop(1.0f, Color::from_hex(0x0034FF)) }
    );
    canvas.fill_linear_gradient(bounds, bg_grad, 32.0f);

    // Gradient stroke 3px from #AA00FF to #00DCFF
    LinearGradient stroke_grad(
        Point(bounds.right(), bounds.y),
        Point(bounds.x, bounds.bottom()),
        { GradientStop(0.0f, Color::from_hex(0xAA00FF)), GradientStop(1.0f, Color::from_hex(0x00DCFF)) }
    );
    canvas.stroke_linear_gradient(bounds, stroke_grad, 32.0f, 3.0f);

    // Header Sparklist icon: (112, 19, 32, 32) - 0.4s vector morphing
    Rect icon_badge(bounds.x + 112.0f, bounds.y + 19.0f, 32.0f, 32.0f);
    canvas.draw_morph_sparkle(icon_badge, sparkle_morph.val, Color::White);

    // Greeting 4 lines: "Hello!\nChoose a song\nyou like the\nvibe of!"
    const char* lines[4] = { "Hello!", "Choose a song", "you like the", "vibe of!" };
    const float line_cx[4] = { 127.0f, 127.75f, 128.25f, 126.0f };
    const float line_cy[4] = { 56.5f, 84.0f, 112.0f, 140.5f };
    for (int i = 0; i < 4; ++i) {
        canvas.draw_text(lines[i], Point(bounds.x + line_cx[i], bounds.y + line_cy[i]), 28.0f, Color::White, FontWeight::Light, TextAlign::Center);
    }

    // Song picker box: (16, 182, 225, 145), radius 24px, fill #D6DEFD
    Rect picker_r(bounds.x + 16.0f, bounds.y + 182.0f, 225.0f, 145.0f);
    canvas.fill_rounded_rect(picker_r, 24.0f, Color::from_hex(0xD6DEFD));
    LinearGradient picker_stroke_grad(
        Point(picker_r.x + picker_r.w * 0.5f, picker_r.y),
        Point(picker_r.x + picker_r.w * 0.5f, picker_r.bottom()),
        { GradientStop(0.0f, Color::from_hex(0x00ECFF)), GradientStop(1.0f, Color::from_hex(0xFF0000)) }
    );
    canvas.stroke_linear_gradient(picker_r, picker_stroke_grad, 24.0f, 1.0f);

    // Search row: (13, 9, 200, 32), fill #91A8FF
    Rect search_row(picker_r.x + 13.0f, picker_r.y + 9.0f, 200.0f, 32.0f);
    canvas.fill_rounded_rect(search_row, 8.0f, Color::from_hex(0x91A8FF));
    canvas.draw_svg("assets/icons/sketch/search.svg", Rect(search_row.x + 44.0f, search_row.y, 32.0f, 32.0f));
    canvas.draw_text("Search", Point(search_row.x + 76.0f, search_row.y + 5.5f), 24.0f, Color::Black, FontWeight::Light);

    // Candidate rows: (13, 45), (13, 79), (13, 113), fill #C1CEFF
    float crow_y[3] = { 45.0f, 79.0f, 113.0f };
    for (int i = 0; i < 3 && i < (int)candidates.size(); ++i) {
        Rect crow(picker_r.x + 13.0f, picker_r.y + crow_y[i], 200.0f, 32.0f);
        canvas.fill_rounded_rect(crow, 8.0f, Color::from_hex(0xC1CEFF));
        // Cover image: (4, 4, 24, 24)
        canvas.draw_image(candidates[i].cover_path, Rect(crow.x + 4.0f, crow.y + 4.0f, 24.0f, 24.0f), 4.0f);
        // Title: (32, 4.5) to align baseline with Quartz rendering
        canvas.draw_text(candidates[i].title, Point(crow.x + 32.0f, crow.y + 4.5f), 10.0f, Color::Black, FontWeight::DemiBold);
        // Divider: (32, 17, 164, 2)
        canvas.fill_rect(Rect(crow.x + 32.0f, crow.y + 17.0f, 164.0f, 2.0f), Color(0.290f, 0.376f, 0.698f, 0.5f));
        // Artist: (32.5, 20)
        canvas.draw_text(candidates[i].artist, Point(crow.x + 32.5f, crow.y + 20.0f), 8.0f, Color::Black, FontWeight::DemiBold);
    }

    // Button 1: "Nevermind." at (18, 335, 220, 32)
    Rect btn1_r(bounds.x + 18.0f, bounds.y + 335.0f, 220.0f, 32.0f);
    canvas.fill_rounded_rect(btn1_r, 16.0f, Color::from_hex(0x9BB0FF));
    canvas.stroke_rounded_rect(btn1_r, 16.0f, Color::from_hex(0xD100FF, 0.5f), 1.0f);
    canvas.draw_text("Nevermind.", Point(bounds.x + 128.5f, btn1_r.y + 7.5f), 16.0f, Color(0.0f, 0.0f, 0.0f, 0.75f), FontWeight::DemiBold, TextAlign::Center);

    // Button 2: "Choose For Me!" at (18, 371, 220, 32)
    Rect btn2_r(bounds.x + 18.0f, bounds.y + 371.0f, 220.0f, 32.0f);
    canvas.fill_rounded_rect(btn2_r, 16.0f, Color::from_hex(0x0098FF));
    LinearGradient btn2_stroke_grad(
        Point(btn2_r.x + btn2_r.w * 0.5f, btn2_r.y),
        Point(btn2_r.x + btn2_r.w * 0.5f, btn2_r.bottom()),
        { GradientStop(0.0f, Color::from_hex(0x00FDFF)), GradientStop(1.0f, Color::from_hex(0xD300FF)) }
    );
    canvas.stroke_linear_gradient(btn2_r, btn2_stroke_grad, 16.0f, 1.0f);
    canvas.draw_text("Choose For Me!", Point(bounds.x + 128.5f, btn2_r.y + 7.5f), 16.0f, Color::Black, FontWeight::DemiBold, TextAlign::Center);
}

bool SparklistModal::handle_mouse_down(float x, float y, int /*button*/) {
    if (!bounds.contains(x, y)) return false;

    // Check candidate rows
    Rect picker_r(bounds.x + 16.0f, bounds.y + 182.0f, 225.0f, 145.0f);
    float crow_y[3] = { 45.0f, 79.0f, 113.0f };
    for (int i = 0; i < 3 && i < (int)candidates.size(); ++i) {
        Rect crow(picker_r.x + 13.0f, picker_r.y + crow_y[i], 200.0f, 32.0f);
        if (crow.contains(x, y)) {
            sparkle_morph.snap(0.0f);
            sparkle_morph.set_target(1.0f);
            if (on_song_chosen) on_song_chosen(candidates[i]);
            return true;
        }
    }

    // Nevermind button: (18, 335, 220, 32)
    Rect btn1_r(bounds.x + 18.0f, bounds.y + 335.0f, 220.0f, 32.0f);
    if (btn1_r.contains(x, y)) {
        if (on_close) on_close();
        return true;
    }

    // Choose For Me button: (18, 371, 220, 32)
    Rect btn2_r(bounds.x + 18.0f, bounds.y + 371.0f, 220.0f, 32.0f);
    if (btn2_r.contains(x, y)) {
        if (on_choose_for_me) on_choose_for_me();
        return true;
    }

    return true; // Eat clicks on modal
}

} // namespace MusicUI
