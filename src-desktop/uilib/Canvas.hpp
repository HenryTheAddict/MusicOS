#pragma once
#include "Types.hpp"
#include <cairo.h>
#include <librsvg/rsvg.h>
#ifdef __APPLE__
#include <ApplicationServices/ApplicationServices.h>
#include <CoreText/CoreText.h>
#endif
#include <ft2build.h>
#include FT_FREETYPE_H
#include <string>
#include <unordered_map>
#include <memory>
#include <functional>

namespace MusicUI {

enum class FontWeight {
    Light,
    Medium,
    DemiBold,
    ExtraBold,
    UltraBold
};

enum class TextAlign {
    Left,
    Center,
    Right
};

class Canvas {
public:
    Canvas(cairo_surface_t* surface, float scale_factor = 2.0f);
    ~Canvas();

    void set_surface(cairo_surface_t* surface, float scale_factor);

    std::function<Color(const Color&)> color_mapper;
    void source_color(const Color& color, float alpha = 1);
    float scale() const { return m_scale; }
    cairo_t* cr() { return m_cr; }

    // State management
    void save();
    void restore();
    void translate(float tx, float ty);
    void scale(float sx, float sy);
    void push_group();
    void pop_group_to_source();
    void paint_with_alpha(float alpha);

    // Clipping
    void push_clip_rect(const Rect& rect);
    void push_clip_rounded_rect(const Rect& rect, float radius, float smoothing = 0.6f);
    void pop_clip();
    void mask_rounded_rect(const Rect& rect, float radius, float smoothing = 0.6f);

    // Solid shapes
    void fill_rect(const Rect& rect, const Color& color);
    void stroke_rect(const Rect& rect, const Color& color, float line_width = 1.0f);
    void fill_rounded_rect(const Rect& rect, float radius, const Color& color, float smoothing = 0.6f);
    void stroke_rounded_rect(const Rect& rect, float radius, const Color& color, float line_width = 1.0f, float smoothing = 0.6f);
    void fill_pill(const Rect& rect, const Color& color);
    void stroke_pill(const Rect& rect, const Color& color, float line_width = 1.0f);
    void fill_circle(float cx, float cy, float radius, const Color& color);
    void stroke_circle(float cx, float cy, float radius, const Color& color, float line_width = 1.0f);
    void draw_line(Point p1, Point p2, const Color& color, float line_width = 1.0f);

    // Gradients
    void fill_linear_gradient(const Rect& rect, const LinearGradient& gradient, float corner_radius = 0.0f, float smoothing = 0.6f);
    void stroke_linear_gradient(const Rect& rect, const LinearGradient& gradient, float corner_radius = 0.0f, float line_width = 1.0f, float smoothing = 0.6f);

    // Drop shadow
    void draw_drop_shadow(const Rect& rect, float corner_radius, float offset_x, float offset_y, float blur_radius, const Color& color);

    // Text
    void draw_text(const std::string& text, const Point& pos, float font_size, const Color& color, FontWeight weight = FontWeight::DemiBold, TextAlign align = TextAlign::Left);
    Size measure_text(const std::string& text, float font_size, FontWeight weight = FontWeight::DemiBold);

    // SVG Rendering
    void draw_svg(const std::string& path, const Rect& bounds, float alpha = 1.0f);

    // Image Rendering
    void draw_image(const std::string& path, const Rect& bounds, float corner_radius = 0.0f, float alpha = 1.0f);

    // Vector Morphing (0.4s multi-contour with smart line disappearance)
    void draw_morph_play_pause(const Rect& bounds, float progress, const Color& color);
    void draw_morph_sparkle(const Rect& bounds, float progress, const Color& color);
    void draw_morph_note_waveform(const Rect& bounds, float progress, const Color& color, float wave_phase = 0.0f);
    void draw_morph_repeat(const Rect& bounds, float progress, const Color& color);
    void draw_morph_shuffle(const Rect& bounds, float progress, const Color& color);
    void draw_morph_scope(const Rect& bounds, float progress, const Color& color);
    void draw_morph_battery_lightning(const Rect& bounds, float progress, const Color& color);

private:
    void init_fonts();
    void add_rounded_rect_path(const Rect& rect, float radius, float smoothing = 0.6f);
    cairo_font_face_t* get_font_face(FontWeight weight);
    RsvgHandle* get_or_load_svg(const std::string& path);
    cairo_surface_t* get_or_load_image(const std::string& path);

    cairo_surface_t* m_surface = nullptr;
    cairo_t* m_cr = nullptr;
    float m_scale = 2.0f;

#ifndef __APPLE__
    FT_Library m_ft = nullptr;
    std::unordered_map<FontWeight, FT_Face> m_ft_faces;
#endif
    std::unordered_map<FontWeight, cairo_font_face_t*> m_font_faces;
#ifdef __APPLE__
    std::unordered_map<FontWeight,CTFontRef> m_native_fonts;
    std::unordered_map<std::string,CTLineRef> m_native_lines;
    CTLineRef native_line(const std::string& text,float size,FontWeight weight);
#endif
    std::unordered_map<std::string, RsvgHandle*> m_svg_cache;
    std::unordered_map<std::string, cairo_surface_t*> m_image_cache;
};

} // namespace MusicUI
