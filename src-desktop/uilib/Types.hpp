#pragma once
#include <cstdint>
#include <vector>
#include <string>
#include <algorithm>

namespace MusicUI {

struct Point {
    float x = 0.0f;
    float y = 0.0f;
    Point() = default;
    Point(float x_, float y_) : x(x_), y(y_) {}
};

struct Size {
    float w = 0.0f;
    float h = 0.0f;
    Size() = default;
    Size(float w_, float h_) : w(w_), h(h_) {}
};

struct Rect {
    float x = 0.0f;
    float y = 0.0f;
    float w = 0.0f;
    float h = 0.0f;

    Rect() = default;
    Rect(float x_, float y_, float w_, float h_) : x(x_), y(y_), w(w_), h(h_) {}

    bool contains(float px, float py) const {
        return px >= x && px <= (x + w) && py >= y && py <= (y + h);
    }

    Rect inset(float dx, float dy) const {
        return Rect(x + dx, y + dy, std::max(0.0f, w - 2 * dx), std::max(0.0f, h - 2 * dy));
    }

    float right() const { return x + w; }
    float bottom() const { return y + h; }
};

struct Color {
    float r = 0.0f;
    float g = 0.0f;
    float b = 0.0f;
    float a = 1.0f;

    Color() = default;
    Color(float r_, float g_, float b_, float a_ = 1.0f) : r(r_), g(g_), b(b_), a(a_) {}

    static Color from_rgb(uint8_t red, uint8_t green, uint8_t blue, float alpha = 1.0f) {
        return Color(red / 255.0f, green / 255.0f, blue / 255.0f, alpha);
    }

    static Color from_hex(uint32_t hex, float alpha = 1.0f) {
        float red = ((hex >> 16) & 0xFF) / 255.0f;
        float green = ((hex >> 8) & 0xFF) / 255.0f;
        float blue = (hex & 0xFF) / 255.0f;
        return Color(red, green, blue, alpha);
    }

    static Color from_hex_alpha(uint32_t hex) {
        float red = ((hex >> 24) & 0xFF) / 255.0f;
        float green = ((hex >> 16) & 0xFF) / 255.0f;
        float blue = ((hex >> 8) & 0xFF) / 255.0f;
        float alpha = (hex & 0xFF) / 255.0f;
        return Color(red, green, blue, alpha);
    }

    static const Color Transparent;
    static const Color White;
    static const Color Black;
    static const Color PrimaryBlue;
    static const Color DeepBlue;
    static const Color LightBlue;
    static const Color TextMuted;
    static const Color TrafficRed;
    static const Color TrafficYellow;
    static const Color TrafficGreen;
};

inline const Color Color::Transparent = Color(0, 0, 0, 0);
inline const Color Color::White = Color(1.0f, 1.0f, 1.0f, 1.0f);
inline const Color Color::Black = Color(0.0f, 0.0f, 0.0f, 1.0f);
inline const Color Color::PrimaryBlue = Color::from_hex(0x00BAFF);
inline const Color Color::DeepBlue = Color::from_hex(0x0064DF);
inline const Color Color::LightBlue = Color::from_hex(0xD6DEFD);
inline const Color Color::TextMuted = Color::from_hex(0x707070);
inline const Color Color::TrafficRed = Color::from_hex(0xFF0000);
inline const Color Color::TrafficYellow = Color::from_hex(0xFFE100);
inline const Color Color::TrafficGreen = Color::from_hex(0x1FC800);

struct GradientStop {
    float position = 0.0f;
    Color color;
    GradientStop() = default;
    GradientStop(float pos, Color c) : position(pos), color(c) {}
};

struct LinearGradient {
    Point start;
    Point end;
    std::vector<GradientStop> stops;

    LinearGradient() = default;
    LinearGradient(Point s, Point e, std::initializer_list<GradientStop> st) : start(s), end(e), stops(st) {}
};

} // namespace MusicUI
