#include <sstream>
#include "Canvas.hpp"
#ifdef __APPLE__
#include <cairo/cairo-quartz.h>
#include <CoreText/CoreText.h>
#endif
#include <cairo-ft.h>
#include <SDL3/SDL.h>
#include <SDL3_image/SDL_image.h>
#include <cmath>
#include <iostream>

namespace MusicUI {

Canvas::Canvas(cairo_surface_t* surface, float scale_factor)
    : m_surface(surface), m_scale(scale_factor) {
    m_cr = cairo_create(m_surface);
    cairo_scale(m_cr, m_scale, m_scale);
    cairo_set_antialias(m_cr, CAIRO_ANTIALIAS_BEST);
    
    cairo_font_options_t* fo = cairo_font_options_create();
    cairo_font_options_set_hint_style(fo, CAIRO_HINT_STYLE_NONE);
    cairo_font_options_set_hint_metrics(fo, CAIRO_HINT_METRICS_OFF);
    cairo_font_options_set_antialias(fo, CAIRO_ANTIALIAS_GRAY);
    cairo_set_font_options(m_cr, fo);
    cairo_font_options_destroy(fo);

    init_fonts();
}

void Canvas::set_surface(cairo_surface_t* surface, float scale_factor) {
    if (m_cr) {
        cairo_destroy(m_cr);
        m_cr = nullptr;
    }
    m_surface = surface;
    m_scale = scale_factor;
    if (m_surface) {
        m_cr = cairo_create(m_surface);
        cairo_scale(m_cr, m_scale, m_scale);
        cairo_set_antialias(m_cr, CAIRO_ANTIALIAS_BEST);

        cairo_font_options_t* fo = cairo_font_options_create();
        cairo_font_options_set_hint_style(fo, CAIRO_HINT_STYLE_NONE);
        cairo_font_options_set_hint_metrics(fo, CAIRO_HINT_METRICS_OFF);
        cairo_font_options_set_antialias(fo, CAIRO_ANTIALIAS_GRAY);
        cairo_set_font_options(m_cr, fo);
        cairo_font_options_destroy(fo);
    }
}

Canvas::~Canvas() {
#ifdef __APPLE__
    for(auto& item:m_native_fonts)CFRelease(item.second);
    for(auto& item:m_native_lines)CFRelease(item.second);
#endif
    for (auto& pair : m_svg_cache) {
        if (pair.second) g_object_unref(pair.second);
    }
    m_svg_cache.clear();

    for (auto& pair : m_image_cache) {
        if (pair.second) cairo_surface_destroy(pair.second);
    }
    m_image_cache.clear();

    for (auto& pair : m_font_faces) {
        if (pair.second) cairo_font_face_destroy(pair.second);
    }
    m_font_faces.clear();

#ifndef __APPLE__
    for (auto& pair : m_ft_faces) {
        if (pair.second) FT_Done_Face(pair.second);
    }
    m_ft_faces.clear();

    if (m_ft) {
        FT_Done_FreeType(m_ft);
        m_ft = nullptr;
    }
#endif

    if (m_cr) {
        cairo_destroy(m_cr);
        m_cr = nullptr;
    }
}

void Canvas::init_fonts() {
#ifdef __APPLE__
    auto load_weight = [this](FontWeight weight, const std::string& path) {
        CGDataProviderRef provider = CGDataProviderCreateWithFilename(path.c_str());
        if (provider) {
            CGFontRef cgfont = CGFontCreateWithDataProvider(provider);
            if (cgfont) {
                m_font_faces[weight] = cairo_quartz_font_face_create_for_cgfont(cgfont);
                m_native_fonts[weight]=CTFontCreateWithGraphicsFont(cgfont,12,nullptr,nullptr);
                CGFontRelease(cgfont);
            }
            CGDataProviderRelease(provider);
        }
    };
#else
    if (FT_Init_FreeType(&m_ft) != 0) {
        std::cerr << "Failed to init FreeType in Canvas\n";
        return;
    }

    auto load_weight = [this](FontWeight weight, const std::string& path) {
        FT_Face face = nullptr;
        if (FT_New_Face(m_ft, path.c_str(), 0, &face) == 0) {
            m_ft_faces[weight] = face;
            m_font_faces[weight] = cairo_ft_font_face_create_for_ft_face(face, 0);
        }
    };
#endif

    load_weight(FontWeight::Light, "assets/fonts/rodin/RodinNTLGPro-L.otf");
    load_weight(FontWeight::Medium, "assets/fonts/rodin/RodinNTLGPro-M.otf");
    load_weight(FontWeight::DemiBold, "assets/fonts/rodin/RodinNTLGPro-DB.otf");
    load_weight(FontWeight::ExtraBold, "assets/fonts/rodin/RodinNTLGPro-EB.otf");
    load_weight(FontWeight::UltraBold, "assets/fonts/rodin/RodinNTLGPro-UB.otf");
}

cairo_font_face_t* Canvas::get_font_face(FontWeight weight) {
    auto it = m_font_faces.find(weight);
    if (it != m_font_faces.end() && it->second) {
        return it->second;
    }
    // Fallback to DemiBold or whatever is loaded
    auto fallback = m_font_faces.find(FontWeight::DemiBold);
    if (fallback != m_font_faces.end() && fallback->second) {
        return fallback->second;
    }
    return nullptr;
}

void Canvas::save() {
    cairo_save(m_cr);
}

void Canvas::restore() {
    cairo_restore(m_cr);
}

void Canvas::translate(float tx, float ty) {
    cairo_translate(m_cr, tx, ty);
}

void Canvas::scale(float sx, float sy) {
    cairo_scale(m_cr, sx, sy);
}

void Canvas::push_group() {
    cairo_push_group(m_cr);
}

void Canvas::pop_group_to_source() {
    cairo_pop_group_to_source(m_cr);
}

void Canvas::paint_with_alpha(float alpha) {
    cairo_paint_with_alpha(m_cr, alpha);
}

void Canvas::add_rounded_rect_path(const Rect& rect, float radius, float smoothing) {
    float x = rect.x;
    float y = rect.y;
    float w = rect.w;
    float h = rect.h;
    float budget = std::min(w, h) * 0.5f;
    if (radius > budget) radius = budget;
    if (radius <= 0.0f) {
        cairo_rectangle(m_cr, x, y, w, h);
        return;
    }

    smoothing = std::max(0.0f, std::min(1.0f, smoothing));
    if (smoothing <= 0.0f) {
        cairo_new_sub_path(m_cr);
        cairo_arc(m_cr, x + w - radius, y + radius, radius, -M_PI / 2.0, 0.0);
        cairo_arc(m_cr, x + w - radius, y + h - radius, radius, 0.0, M_PI / 2.0);
        cairo_arc(m_cr, x + radius, y + h - radius, radius, M_PI / 2.0, M_PI);
        cairo_arc(m_cr, x + radius, y + radius, radius, M_PI, 3.0 * M_PI / 2.0);
        cairo_close_path(m_cr);
        return;
    }

    float p = (1.0f + smoothing) * radius;
    if (p > budget) {
        float max_s = (budget / radius) - 1.0f;
        smoothing = std::max(0.0f, max_s);
        p = budget;
    }

    float arc_measure = 90.0f * (1.0f - smoothing);
    float arc_measure_rad = arc_measure * (float)(M_PI / 180.0);
    float arc_section_length = std::sin(arc_measure_rad * 0.5f) * radius * std::sqrt(2.0f);

    float angle_alpha = (90.0f - arc_measure) * 0.5f;
    float angle_alpha_rad = angle_alpha * (float)(M_PI / 180.0);
    float p3_to_p4 = radius * std::tan(angle_alpha_rad * 0.5f);

    float angle_beta = 45.0f * smoothing;
    float angle_beta_rad = angle_beta * (float)(M_PI / 180.0);
    float c_val = p3_to_p4 * std::cos(angle_beta_rad);
    float d_val = c_val * std::tan(angle_beta_rad);

    float b_val = (p - arc_section_length - c_val - d_val) / 3.0f;
    float a_val = 2.0f * b_val;

    cairo_new_sub_path(m_cr);
    cairo_move_to(m_cr, x + w - p, y);

    // Top-Right corner
    cairo_curve_to(m_cr, x + w - p + a_val, y,
                          x + w - p + a_val + b_val, y,
                          x + w - p + a_val + b_val + c_val, y + d_val);
    cairo_arc(m_cr, x + w - radius, y + radius, radius, -M_PI * 0.5 + angle_alpha_rad, 0.0 - angle_alpha_rad);
    cairo_curve_to(m_cr, x + w, y + p - (a_val + b_val),
                          x + w, y + p - a_val,
                          x + w, y + p);

    // Right edge
    cairo_line_to(m_cr, x + w, y + h - p);

    // Bottom-Right corner
    cairo_curve_to(m_cr, x + w, y + h - p + a_val,
                          x + w, y + h - p + a_val + b_val,
                          x + w - d_val, y + h - p + a_val + b_val + c_val);
    cairo_arc(m_cr, x + w - radius, y + h - radius, radius, 0.0 + angle_alpha_rad, M_PI * 0.5 - angle_alpha_rad);
    cairo_curve_to(m_cr, x + w - p + (a_val + b_val), y + h,
                          x + w - p + a_val, y + h,
                          x + w - p, y + h);

    // Bottom edge
    cairo_line_to(m_cr, x + p, y + h);

    // Bottom-Left corner
    cairo_curve_to(m_cr, x + p - a_val, y + h,
                          x + p - (a_val + b_val), y + h,
                          x + p - (a_val + b_val + c_val), y + h - d_val);
    cairo_arc(m_cr, x + radius, y + h - radius, radius, M_PI * 0.5 + angle_alpha_rad, M_PI - angle_alpha_rad);
    cairo_curve_to(m_cr, x, y + h - p + (a_val + b_val),
                          x, y + h - p + a_val,
                          x, y + h - p);

    // Left edge
    cairo_line_to(m_cr, x, y + p);

    // Top-Left corner
    cairo_curve_to(m_cr, x, y + p - a_val,
                          x, y + p - (a_val + b_val),
                          x + d_val, y + p - (a_val + b_val + c_val));
    cairo_arc(m_cr, x + radius, y + radius, radius, M_PI + angle_alpha_rad, 1.5 * M_PI - angle_alpha_rad);
    cairo_curve_to(m_cr, x + p - (a_val + b_val), y,
                          x + p - a_val, y,
                          x + p, y);

    cairo_close_path(m_cr);
}

void Canvas::push_clip_rect(const Rect& rect) {
    save();
    cairo_rectangle(m_cr, rect.x, rect.y, rect.w, rect.h);
    cairo_clip(m_cr);
}

void Canvas::push_clip_rounded_rect(const Rect& rect, float radius, float smoothing) {
    save();
    add_rounded_rect_path(rect, radius, smoothing);
    cairo_clip(m_cr);
}

void Canvas::pop_clip() {
    restore();
}

void Canvas::mask_rounded_rect(const Rect& rect, float radius, float smoothing) {
    save();
    cairo_set_operator(m_cr, CAIRO_OPERATOR_DEST_IN);
    cairo_new_path(m_cr);
    add_rounded_rect_path(rect, radius, smoothing);
    cairo_set_source_rgba(m_cr, 1.0, 1.0, 1.0, 1.0);
    cairo_fill(m_cr);
    restore();
}


void Canvas::source_color(const Color& original, float alpha) {
    Color color = color_mapper ? color_mapper(original) : original;
    cairo_set_source_rgba(m_cr, color.r, color.g, color.b, color.a * alpha);
}

void Canvas::fill_rect(const Rect& rect, const Color& color) {
    source_color(color);
    cairo_rectangle(m_cr, rect.x, rect.y, rect.w, rect.h);
    cairo_fill(m_cr);
}

void Canvas::stroke_rect(const Rect& rect, const Color& color, float line_width) {
    source_color(color);
    cairo_set_line_width(m_cr, line_width);
    cairo_rectangle(m_cr, rect.x + line_width * 0.5f, rect.y + line_width * 0.5f, rect.w - line_width, rect.h - line_width);
    cairo_stroke(m_cr);
}

void Canvas::fill_rounded_rect(const Rect& rect, float radius, const Color& color, float smoothing) {
    source_color(color);
    add_rounded_rect_path(rect, radius, smoothing);
    cairo_fill(m_cr);
}

void Canvas::stroke_rounded_rect(const Rect& rect, float radius, const Color& color, float line_width, float smoothing) {
    source_color(color);
    cairo_set_line_width(m_cr, line_width);
    Rect inset_r = rect.inset(line_width * 0.5f, line_width * 0.5f);
    add_rounded_rect_path(inset_r, std::max(0.0f, radius - line_width * 0.5f), smoothing);
    cairo_stroke(m_cr);
}

void Canvas::fill_pill(const Rect& rect, const Color& color) {
    fill_rounded_rect(rect, rect.h * 0.5f, color);
}

void Canvas::stroke_pill(const Rect& rect, const Color& color, float line_width) {
    stroke_rounded_rect(rect, rect.h * 0.5f, color, line_width);
}

void Canvas::fill_circle(float cx, float cy, float radius, const Color& color) {
    source_color(color);
    cairo_new_sub_path(m_cr);
    cairo_arc(m_cr, cx, cy, radius, 0.0, 2.0 * M_PI);
    cairo_fill(m_cr);
}

void Canvas::stroke_circle(float cx, float cy, float radius, const Color& color, float line_width) {
    source_color(color);
    cairo_set_line_width(m_cr, line_width);
    cairo_new_sub_path(m_cr);
    cairo_arc(m_cr, cx, cy, radius - line_width * 0.5f, 0.0, 2.0 * M_PI);
    cairo_stroke(m_cr);
}

void Canvas::draw_line(Point p1, Point p2, const Color& color, float line_width) {
    source_color(color);
    cairo_set_line_width(m_cr, line_width);
    cairo_move_to(m_cr, p1.x, p1.y);
    cairo_line_to(m_cr, p2.x, p2.y);
    cairo_stroke(m_cr);
}

void Canvas::fill_linear_gradient(const Rect& rect, const LinearGradient& gradient, float corner_radius, float smoothing) {
    cairo_pattern_t* pat = cairo_pattern_create_linear(
        gradient.start.x, gradient.start.y,
        gradient.end.x, gradient.end.y
    );
    for (const auto& stop : gradient.stops) {
        const Color color = color_mapper ? color_mapper(stop.color) : stop.color;
        cairo_pattern_add_color_stop_rgba(pat, stop.position, color.r, color.g, color.b, color.a);
    }

    if (corner_radius > 0.0f) {
        add_rounded_rect_path(rect, corner_radius, smoothing);
    } else {
        cairo_rectangle(m_cr, rect.x, rect.y, rect.w, rect.h);
    }

    cairo_set_source(m_cr, pat);
    cairo_fill(m_cr);
    cairo_pattern_destroy(pat);
}

void Canvas::stroke_linear_gradient(const Rect& rect, const LinearGradient& gradient, float corner_radius, float line_width, float smoothing) {
    cairo_pattern_t* pat = cairo_pattern_create_linear(
        gradient.start.x, gradient.start.y,
        gradient.end.x, gradient.end.y
    );
    for (const auto& stop : gradient.stops) {
        const Color color = color_mapper ? color_mapper(stop.color) : stop.color;
        cairo_pattern_add_color_stop_rgba(pat, stop.position, color.r, color.g, color.b, color.a);
    }

    if (corner_radius > 0.0f) {
        add_rounded_rect_path(rect, corner_radius, smoothing);
    } else {
        cairo_rectangle(m_cr, rect.x, rect.y, rect.w, rect.h);
    }

    cairo_set_source(m_cr, pat);
    cairo_set_line_width(m_cr, line_width);
    cairo_stroke(m_cr);
    cairo_pattern_destroy(pat);
}

void Canvas::draw_drop_shadow(const Rect& rect, float corner_radius, float offset_x, float offset_y, float blur_radius, const Color& color) {
    int passes = 6;
    for (int i = passes; i >= 1; --i) {
        float factor = (float)i / (float)passes;
        float spread = blur_radius * factor;
        float alpha = (color.a / (float)passes) * 0.85f;
        Rect shadow_r(
            rect.x + offset_x - spread * 0.5f,
            rect.y + offset_y - spread * 0.2f,
            rect.w + spread,
            rect.h + spread
        );
        fill_rounded_rect(shadow_r, corner_radius + spread * 0.5f, Color(color.r, color.g, color.b, alpha));
    }
}

#ifdef __APPLE__
static CTLineRef shaped_line(const std::string& text,CTFontRef base,float size) {
    auto font=CTFontCreateCopyWithAttributes(base,size,nullptr,nullptr);
    auto string=CFStringCreateWithBytes(nullptr,reinterpret_cast<const UInt8*>(text.data()),text.size(),kCFStringEncodingUTF8,false);
    if(!string){CFRelease(font);return nullptr;}
    const void* keys[]={kCTFontAttributeName};const void* values[]={font};
    auto attrs=CFDictionaryCreate(nullptr,keys,values,1,&kCFTypeDictionaryKeyCallBacks,&kCFTypeDictionaryValueCallBacks);
    auto attributed=CFAttributedStringCreate(nullptr,string,attrs);
    auto line=CTLineCreateWithAttributedString(attributed);
    CFRelease(attributed);CFRelease(attrs);CFRelease(string);CFRelease(font);return line;
}
CTLineRef Canvas::native_line(const std::string& text,float size,FontWeight weight) {
    auto key=std::to_string(int(weight))+":"+std::to_string(size)+":"+text;
    if(auto i=m_native_lines.find(key);i!=m_native_lines.end())return i->second;
    if(m_native_lines.size()>=4096){for(auto& item:m_native_lines)CFRelease(item.second);m_native_lines.clear();}
    auto line=shaped_line(text,m_native_fonts.at(weight),size);if(line)m_native_lines[key]=line;return line;
}
void Canvas::draw_text(const std::string& text,const Point& pos,float size,const Color& color,FontWeight weight,TextAlign align) {
    if(text.empty() || !m_native_fonts.count(weight))return;
    source_color(color);std::stringstream stream(text);std::string row;float y=pos.y;
    while(std::getline(stream,row,'\n')){
        auto line=native_line(row,size,weight);if(!line)continue;
        CGFloat ascent,descent,leading;double width=CTLineGetTypographicBounds(line,&ascent,&descent,&leading);
        float x=pos.x-(align==TextAlign::Center?width*.5:align==TextAlign::Right?width:0);
        auto runs=CTLineGetGlyphRuns(line);
        for(CFIndex i=0;i<CFArrayGetCount(runs);++i){
            auto run=static_cast<CTRunRef>(CFArrayGetValueAtIndex(runs,i));CFIndex count=CTRunGetGlyphCount(run);
            std::vector<CGGlyph> glyphs(count);std::vector<CGPoint> positions(count);
            CTRunGetGlyphs(run,CFRangeMake(0,0),glyphs.data());CTRunGetPositions(run,CFRangeMake(0,0),positions.data());
            auto run_font=static_cast<CTFontRef>(CFDictionaryGetValue(CTRunGetAttributes(run),kCTFontAttributeName));
            auto cgfont=CTFontCopyGraphicsFont(run_font,nullptr);auto face=cairo_quartz_font_face_create_for_cgfont(cgfont);
            cairo_set_font_face(m_cr,face);cairo_set_font_size(m_cr,size);
            std::vector<cairo_glyph_t> out(count);
            for(CFIndex n=0;n<count;++n)out[n]={glyphs[n],x+positions[n].x,y+ascent-positions[n].y};
            cairo_show_glyphs(m_cr,out.data(),int(count));cairo_font_face_destroy(face);CGFontRelease(cgfont);
        }
        y+=size*1.2f;
    }
}
Size Canvas::measure_text(const std::string& text,float size,FontWeight weight) {
    if(text.empty() || !m_native_fonts.count(weight))return {};
    auto line=native_line(text,size,weight);if(!line)return {};
    CGFloat ascent,descent,leading;double width=CTLineGetTypographicBounds(line,&ascent,&descent,&leading);
    return {float(width),float(ascent+descent+leading)};
}
#else
void Canvas::draw_text(const std::string& text, const Point& pos, float font_size, const Color& color, FontWeight weight, TextAlign align) {
    if (text.empty()) return;
    cairo_font_face_t* face = get_font_face(weight);
    if (face) {
        cairo_set_font_face(m_cr, face);
    } else {
        cairo_select_font_face(m_cr, "Helvetica", CAIRO_FONT_SLANT_NORMAL, CAIRO_FONT_WEIGHT_BOLD);
    }
    cairo_set_font_size(m_cr, font_size);
    source_color(color);

    cairo_font_extents_t font_extents;
    cairo_font_extents(m_cr, &font_extents);
    float line_height = font_size * 1.15f;

    std::stringstream ss(text);
    std::string line;
    float cur_y = pos.y + font_extents.ascent;

    while (std::getline(ss, line, '\n')) {
        cairo_text_extents_t extents;
        cairo_text_extents(m_cr, line.c_str(), &extents);

        float draw_x = pos.x;
        if (align == TextAlign::Center) {
            draw_x -= (extents.width * 0.5f + extents.x_bearing);
        } else if (align == TextAlign::Right) {
            draw_x -= (extents.width + extents.x_bearing);
        }

        cairo_move_to(m_cr, draw_x, cur_y);
        cairo_show_text(m_cr, line.c_str());
        cur_y += line_height;
    }
}

Size Canvas::measure_text(const std::string& text, float font_size, FontWeight weight) {
    if (text.empty()) return Size(0, 0);
    cairo_font_face_t* face = get_font_face(weight);
    if (face) {
        cairo_set_font_face(m_cr, face);
    } else {
        cairo_select_font_face(m_cr, "Helvetica", CAIRO_FONT_SLANT_NORMAL, CAIRO_FONT_WEIGHT_BOLD);
    }
    cairo_set_font_size(m_cr, font_size);

    cairo_text_extents_t extents;
    cairo_text_extents(m_cr, text.c_str(), &extents);
    cairo_font_extents_t font_extents;
    cairo_font_extents(m_cr, &font_extents);
    return Size(extents.x_advance, font_extents.height);
}

#endif

RsvgHandle* Canvas::get_or_load_svg(const std::string& path) {
    auto it = m_svg_cache.find(path);
    if (it != m_svg_cache.end()) {
        return it->second;
    }
    GError* error = nullptr;
    RsvgHandle* handle = rsvg_handle_new_from_file(path.c_str(), &error);
    if (!handle) {
        std::cerr << "Failed to load SVG: " << path << " (" << (error ? error->message : "unknown") << ")\n";
        m_svg_cache[path] = nullptr;
        return nullptr;
    }
    m_svg_cache[path] = handle;
    return handle;
}

void Canvas::draw_svg(const std::string& path, const Rect& bounds, float alpha) {
    RsvgHandle* handle = get_or_load_svg(path);
    if (!handle) return;

    save();
    RsvgRectangle viewport = { bounds.x, bounds.y, bounds.w, bounds.h };
    if (alpha < 1.0f) {
        cairo_push_group(m_cr);
        rsvg_handle_render_document(handle, m_cr, &viewport, nullptr);
        cairo_pop_group_to_source(m_cr);
        cairo_paint_with_alpha(m_cr, alpha);
    } else {
        rsvg_handle_render_document(handle, m_cr, &viewport, nullptr);
    }
    restore();
}

cairo_surface_t* Canvas::get_or_load_image(const std::string& path) {
    auto it = m_image_cache.find(path);
    if (it != m_image_cache.end()) {
        return it->second;
    }

    SDL_Surface* sdl_surf = IMG_Load(path.c_str());
    if (!sdl_surf) {
        m_image_cache[path] = nullptr;
        return nullptr;
    }

    // Convert to ARGB8888 for Cairo
    SDL_Surface* converted = SDL_ConvertSurface(sdl_surf, SDL_PIXELFORMAT_ARGB8888);
    SDL_DestroySurface(sdl_surf);
    if (!converted) {
        m_image_cache[path] = nullptr;
        return nullptr;
    }

    cairo_surface_t* c_surf = cairo_image_surface_create_for_data(
        (unsigned char*)converted->pixels,
        CAIRO_FORMAT_ARGB32,
        converted->w,
        converted->h,
        converted->pitch
    );

    // Create a standalone copy so we can free converted
    cairo_surface_t* standalone = cairo_image_surface_create(CAIRO_FORMAT_ARGB32, converted->w, converted->h);
    cairo_t* tmp_cr = cairo_create(standalone);
    cairo_set_source_surface(tmp_cr, c_surf, 0, 0);
    cairo_paint(tmp_cr);
    cairo_destroy(tmp_cr);
    cairo_surface_destroy(c_surf);
    SDL_DestroySurface(converted);

    m_image_cache[path] = standalone;
    return standalone;
}

void Canvas::draw_image(const std::string& path, const Rect& bounds, float corner_radius, float alpha) {
    cairo_surface_t* img = get_or_load_image(path);
    if (!img) return;

    int img_w = cairo_image_surface_get_width(img);
    int img_h = cairo_image_surface_get_height(img);
    if (img_w <= 0 || img_h <= 0) return;

    save();
    if (corner_radius > 0.0f) {
        add_rounded_rect_path(bounds, corner_radius);
        cairo_clip(m_cr);
    }

    cairo_translate(m_cr, bounds.x, bounds.y);
    cairo_scale(m_cr, bounds.w / (float)img_w, bounds.h / (float)img_h);
    cairo_set_source_surface(m_cr, img, 0, 0);
    if (alpha < 1.0f) {
        cairo_paint_with_alpha(m_cr, alpha);
    } else {
        cairo_paint(m_cr);
    }
    restore();
}

void Canvas::draw_morph_play_pause(const Rect& bounds, float progress, const Color& color) {
    float t = std::max(0.0f, std::min(1.0f, progress));

    save();
    source_color(color);
    cairo_set_line_cap(m_cr, CAIRO_LINE_CAP_ROUND);
    cairo_set_line_join(m_cr, CAIRO_LINE_JOIN_ROUND);

    float bx = bounds.x;
    float by = bounds.y;
    float bw = bounds.w;
    float bh = bounds.h;

    // Contour 1: Left Bar / Left Triangle wedge
    Point p1_0(bx + bw * 0.24f, by + bh * 0.16f);
    Point p1_1(bx + bw * (0.52f - 0.08f * t), by + bh * (0.33f - 0.17f * t));
    Point p1_2(bx + bw * (0.52f - 0.08f * t), by + bh * (0.67f + 0.17f * t));
    Point p1_3(bx + bw * 0.24f, by + bh * 0.84f);

    cairo_new_sub_path(m_cr);
    cairo_move_to(m_cr, p1_0.x, p1_0.y);
    cairo_line_to(m_cr, p1_1.x, p1_1.y);
    cairo_line_to(m_cr, p1_2.x, p1_2.y);
    cairo_line_to(m_cr, p1_3.x, p1_3.y);
    cairo_close_path(m_cr);
    cairo_fill(m_cr);

    // Contour 2: Right Bar / Triangle Apex (emerges / collapses cleanly at apex)
    Point p2_0(bx + bw * (0.52f + 0.04f * t), by + bh * (0.33f - 0.17f * t));
    Point p2_1(bx + bw * (0.80f - 0.04f * t), by + bh * (0.50f - 0.34f * t));
    Point p2_2(bx + bw * (0.80f - 0.04f * t), by + bh * (0.50f + 0.34f * t));
    Point p2_3(bx + bw * (0.52f + 0.04f * t), by + bh * (0.67f + 0.17f * t));

    cairo_new_sub_path(m_cr);
    cairo_move_to(m_cr, p2_0.x, p2_0.y);
    cairo_line_to(m_cr, p2_1.x, p2_1.y);
    cairo_line_to(m_cr, p2_2.x, p2_2.y);
    cairo_line_to(m_cr, p2_3.x, p2_3.y);
    cairo_close_path(m_cr);
    cairo_fill(m_cr);

    restore();
}

void Canvas::draw_morph_sparkle(const Rect& bounds, float progress, const Color& color) {
    float t = std::max(0.0f, std::min(1.0f, progress));
    save();
    source_color(color);
    cairo_set_line_cap(m_cr, CAIRO_LINE_CAP_ROUND);

    float cx = bounds.x + bounds.w * 0.5f;
    float cy = bounds.y + bounds.h * 0.5f;
    float r = bounds.w * 0.42f;

    if (t < 0.05f) {
        cairo_new_sub_path(m_cr);
        cairo_move_to(m_cr, cx, cy - r);
        cairo_curve_to(m_cr, cx, cy - r * 0.2f, cx + r * 0.2f, cy, cx + r, cy);
        cairo_curve_to(m_cr, cx + r * 0.2f, cy, cx, cy + r * 0.2f, cx, cy + r);
        cairo_curve_to(m_cr, cx, cy + r * 0.2f, cx - r * 0.2f, cy, cx - r, cy);
        cairo_curve_to(m_cr, cx - r * 0.2f, cy, cx, cy - r * 0.2f, cx, cy - r);
        cairo_close_path(m_cr);
        cairo_fill(m_cr);
    } else {
        float h_left = (1.0f - t) * (r * 0.5f) + t * (r * 0.65f);
        float h_mid = r * (1.0f + 0.1f * t);
        float h_right = (1.0f - t) * (r * 0.5f) + t * (r * 0.50f);

        float spacing = r * 0.6f * t;
        float bar_w = 2.5f + 1.0f * t;

        cairo_set_line_width(m_cr, bar_w);

        cairo_move_to(m_cr, cx, cy - h_mid);
        cairo_line_to(m_cr, cx, cy + h_mid);
        cairo_stroke(m_cr);

        if (spacing > 1.0f) {
            float alpha = std::min(1.0f, t / 0.4f);
            source_color(color, alpha);
            cairo_move_to(m_cr, cx - spacing, cy - h_left);
            cairo_line_to(m_cr, cx - spacing, cy + h_left);
            cairo_stroke(m_cr);

            cairo_move_to(m_cr, cx + spacing, cy - h_right);
            cairo_line_to(m_cr, cx + spacing, cy + h_right);
            cairo_stroke(m_cr);
        }
    }
    restore();
}

void Canvas::draw_morph_note_waveform(const Rect& bounds, float progress, const Color& color, float wave_phase) {
    float t = std::max(0.0f, std::min(1.0f, progress));
    save();
    source_color(color);
    cairo_set_line_cap(m_cr, CAIRO_LINE_CAP_ROUND);
    cairo_set_line_join(m_cr, CAIRO_LINE_JOIN_ROUND);

    float bx = bounds.x;
    float by = bounds.y;
    float bw = bounds.w;
    float bh = bounds.h;

    if (t < 0.99f) {
        // Musical notehead + stem
        float note_alpha = 1.0f - t;
        source_color(color, note_alpha);

        // Notehead tilted oval at bottom-left
        float head_x = bx + bw * 0.35f;
        float head_y = by + bh * 0.72f;
        float head_r = bw * 0.18f;
        cairo_save(m_cr);
        cairo_translate(m_cr, head_x, head_y);
        cairo_rotate(m_cr, -0.35f);
        cairo_scale(m_cr, 1.25f, 0.85f);
        cairo_arc(m_cr, 0, 0, head_r, 0, 2.0 * M_PI);
        cairo_fill(m_cr);
        cairo_restore(m_cr);

        // Stem
        cairo_set_line_width(m_cr, 2.0f);
        cairo_move_to(m_cr, head_x + head_r * 0.8f, head_y);
        cairo_line_to(m_cr, head_x + head_r * 0.8f, by + bh * 0.22f);
        cairo_stroke(m_cr);

        // Flag
        cairo_move_to(m_cr, head_x + head_r * 0.8f, by + bh * 0.22f);
        cairo_curve_to(m_cr, bx + bw * 0.70f, by + bh * 0.26f, bx + bw * 0.78f, by + bh * 0.42f, bx + bw * 0.70f, by + bh * 0.52f);
        cairo_stroke(m_cr);
    }

    if (t > 0.01f) {
        // 4 Dynamic Equalizer Waveform bars
        source_color(color, t);
        cairo_set_line_width(m_cr, 2.2f);

        float bar_x[4] = { bx + bw * 0.22f, bx + bw * 0.40f, bx + bw * 0.60f, bx + bw * 0.78f };
        float bar_min_h = bh * 0.15f;
        float bar_max_h = bh * 0.65f;

        for (int i = 0; i < 4; ++i) {
            float phase_offset = wave_phase * 5.0f + i * 1.35f;
            float osc = 0.5f + 0.5f * std::sin(phase_offset);
            float current_h = (bar_min_h + osc * (bar_max_h - bar_min_h)) * t;
            float cy = by + bh * 0.5f;

            cairo_move_to(m_cr, bar_x[i], cy - current_h * 0.5f);
            cairo_line_to(m_cr, bar_x[i], cy + current_h * 0.5f);
            cairo_stroke(m_cr);
        }
    }

    restore();
}

void Canvas::draw_morph_repeat(const Rect& bounds, float progress, const Color& color) {
    float t = std::max(0.0f, std::min(1.0f, progress));
    save();
    source_color(color);
    cairo_set_line_width(m_cr, 1.75f);
    cairo_set_line_cap(m_cr, CAIRO_LINE_CAP_ROUND);
    cairo_set_line_join(m_cr, CAIRO_LINE_JOIN_ROUND);

    float cx = bounds.x + bounds.w * 0.5f;
    float cy = bounds.y + bounds.h * 0.5f;
    float r = bounds.w * 0.35f;

    // Top arc / straight line morph
    cairo_new_sub_path(m_cr);
    if (t < 0.5f) {
        float f = t * 2.0f;
        float y0 = cy - r * (0.6f + 0.4f * f);
        cairo_move_to(m_cr, cx - r, y0);
        cairo_curve_to(m_cr, cx - r * 0.5f, y0 - r * 0.3f * f, cx + r * 0.5f, y0 - r * 0.3f * f, cx + r, y0);
        // Arrowhead
        cairo_move_to(m_cr, cx + r - 3.0f, y0 - 3.0f);
        cairo_line_to(m_cr, cx + r, y0);
        cairo_line_to(m_cr, cx + r - 3.0f, y0 + 3.0f);
    } else {
        float f = (t - 0.5f) * 2.0f;
        float start_angle = -M_PI * (0.8f + 0.2f * f);
        float end_angle = 0.0f;
        cairo_arc(m_cr, cx, cy, r, start_angle, end_angle);
        cairo_move_to(m_cr, cx + r - 3.0f, cy - 3.0f);
        cairo_line_to(m_cr, cx + r, cy);
        cairo_line_to(m_cr, cx + r + 3.0f, cy - 3.0f);
    }
    cairo_stroke(m_cr);

    // Bottom arc / straight line morph
    cairo_new_sub_path(m_cr);
    if (t < 0.5f) {
        float f = t * 2.0f;
        float y1 = cy + r * (0.6f + 0.4f * f);
        cairo_move_to(m_cr, cx + r, y1);
        cairo_curve_to(m_cr, cx + r * 0.5f, y1 + r * 0.3f * f, cx - r * 0.5f, y1 + r * 0.3f * f, cx - r, y1);
        cairo_move_to(m_cr, cx - r + 3.0f, y1 - 3.0f);
        cairo_line_to(m_cr, cx - r, y1);
        cairo_line_to(m_cr, cx - r + 3.0f, y1 + 3.0f);
    } else {
        float f = (t - 0.5f) * 2.0f;
        float start_angle = 0.2f * (1.0f - f);
        float end_angle = M_PI * (0.9f + 0.1f * f);
        cairo_arc(m_cr, cx, cy, r, start_angle, end_angle);
        cairo_move_to(m_cr, cx - r - 3.0f, cy + 3.0f);
        cairo_line_to(m_cr, cx - r, cy);
        cairo_line_to(m_cr, cx - r + 3.0f, cy + 3.0f);
    }
    cairo_stroke(m_cr);

    restore();
}

void Canvas::draw_morph_shuffle(const Rect& bounds, float progress, const Color& color) {
    float t = std::max(0.0f, std::min(1.0f, progress));
    save();
    source_color(color);
    cairo_set_line_width(m_cr, 1.75f);
    cairo_set_line_cap(m_cr, CAIRO_LINE_CAP_ROUND);
    cairo_set_line_join(m_cr, CAIRO_LINE_JOIN_ROUND);

    float bx = bounds.x + 2.0f;
    float by = bounds.y + 2.0f;
    float bw = bounds.w - 4.0f;
    float bh = bounds.h - 4.0f;

    float y_top = by + bh * 0.30f;
    float y_bot = by + bh * 0.70f;

    // Track 1: Starts top-left, ends top-right (t=0) or bottom-right (t=1)
    float end1_y = (1.0f - t) * y_top + t * y_bot;
    cairo_new_sub_path(m_cr);
    cairo_move_to(m_cr, bx, y_top);
    cairo_curve_to(m_cr, bx + bw * 0.35f, y_top, bx + bw * 0.65f, end1_y, bx + bw, end1_y);
    cairo_stroke(m_cr);

    // Arrowhead 1
    cairo_new_sub_path(m_cr);
    cairo_move_to(m_cr, bx + bw - 3.5f, end1_y - 3.0f);
    cairo_line_to(m_cr, bx + bw, end1_y);
    cairo_line_to(m_cr, bx + bw - 3.5f, end1_y + 3.0f);
    cairo_stroke(m_cr);

    // Track 2: Starts bottom-left, ends bottom-right (t=0) or top-right (t=1)
    float end2_y = (1.0f - t) * y_bot + t * y_top;
    cairo_new_sub_path(m_cr);
    cairo_move_to(m_cr, bx, y_bot);
    cairo_curve_to(m_cr, bx + bw * 0.35f, y_bot, bx + bw * 0.65f, end2_y, bx + bw, end2_y);
    cairo_stroke(m_cr);

    // Arrowhead 2
    cairo_new_sub_path(m_cr);
    cairo_move_to(m_cr, bx + bw - 3.5f, end2_y - 3.0f);
    cairo_line_to(m_cr, bx + bw, end2_y);
    cairo_line_to(m_cr, bx + bw - 3.5f, end2_y + 3.0f);
    cairo_stroke(m_cr);

    restore();
}

void Canvas::draw_morph_scope(const Rect& bounds, float progress, const Color& color) {
    float t = std::max(0.0f, std::min(1.0f, progress));
    save();
    source_color(color);
    cairo_set_line_cap(m_cr, CAIRO_LINE_CAP_ROUND);
    cairo_set_line_join(m_cr, CAIRO_LINE_JOIN_ROUND);

    float cx = bounds.x + bounds.w * 0.5f;
    float cy = bounds.y + bounds.h * 0.5f;

    // At t=0: 3 dots stacked in a column
    // At t=1: iPod/phone device capsule outline with screen + clickwheel
    if (t < 0.99f) {
        float dot_r = 3.0f * (1.0f - t);
        float dot_spacing = 8.0f * (1.0f - t);
        float alpha = 1.0f - t;
        source_color(color, alpha);
        cairo_arc(m_cr, cx, cy - dot_spacing, dot_r, 0, 2.0 * M_PI);
        cairo_fill(m_cr);
        cairo_arc(m_cr, cx, cy, dot_r, 0, 2.0 * M_PI);
        cairo_fill(m_cr);
        cairo_arc(m_cr, cx, cy + dot_spacing, dot_r, 0, 2.0 * M_PI);
        cairo_fill(m_cr);
    }

    if (t > 0.01f) {
        float dev_w = 16.0f * t;
        float dev_h = 24.0f * t;
        Rect dev_rect(cx - dev_w * 0.5f, cy - dev_h * 0.5f, dev_w, dev_h);
        source_color(color, t);
        cairo_set_line_width(m_cr, 1.6f);
        add_rounded_rect_path(dev_rect, 4.0f * t);
        cairo_stroke(m_cr);

        // Screen mini-window
        Rect screen_rect(cx - dev_w * 0.35f, cy - dev_h * 0.38f, dev_w * 0.70f, dev_h * 0.38f);
        add_rounded_rect_path(screen_rect, 1.5f * t);
        cairo_stroke(m_cr);

        // Clickwheel circle
        cairo_arc(m_cr, cx, cy + dev_h * 0.22f, dev_w * 0.20f, 0, 2.0 * M_PI);
        cairo_stroke(m_cr);
    }

    restore();
}

void Canvas::draw_morph_battery_lightning(const Rect& bounds, float progress, const Color& color) {
    float t = std::max(0.0f, std::min(1.0f, progress));
    save();
    source_color(color);
    cairo_set_line_cap(m_cr, CAIRO_LINE_CAP_ROUND);
    cairo_set_line_join(m_cr, CAIRO_LINE_JOIN_ROUND);

    float cx = bounds.x + bounds.w * 0.5f;
    float cy = bounds.y + bounds.h * 0.5f;
    float sz = std::min(bounds.w, bounds.h) * 0.8f;

    // Morph from horizontal Battery outline to Lightning Bolt
    if (t < 0.99f) {
        float alpha = 1.0f - t;
        source_color(color, alpha);
        cairo_set_line_width(m_cr, 2.0f);
        Rect bat_r(cx - sz * 0.45f, cy - sz * 0.28f, sz * 0.80f, sz * 0.56f);
        add_rounded_rect_path(bat_r, 4.0f);
        cairo_stroke(m_cr);
        // Positive terminal bump
        Rect bump_r(bat_r.right(), cy - sz * 0.12f, 3.0f, sz * 0.24f);
        add_rounded_rect_path(bump_r, 1.5f);
        cairo_fill(m_cr);
    }

    if (t > 0.01f) {
        source_color(color, t);
        cairo_new_sub_path(m_cr);
        // Sharp energetic lightning bolt
        cairo_move_to(m_cr, cx + sz * 0.08f, cy - sz * 0.48f);
        cairo_line_to(m_cr, cx - sz * 0.32f, cy + sz * 0.02f);
        cairo_line_to(m_cr, cx - sz * 0.02f, cy + sz * 0.02f);
        cairo_line_to(m_cr, cx - sz * 0.14f, cy + sz * 0.48f);
        cairo_line_to(m_cr, cx + sz * 0.35f, cy - sz * 0.08f);
        cairo_line_to(m_cr, cx + sz * 0.05f, cy - sz * 0.08f);
        cairo_close_path(m_cr);
        cairo_fill(m_cr);
    }

    restore();
}

} // namespace MusicUI
