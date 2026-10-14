#include "../Components.hpp"
#include <iostream>

namespace MusicUI {

// 1. Button
Button::Button(const Rect& b, const std::string& txt) : Widget(b), text(txt) {}

void Button::render_self(Canvas& canvas) {
    if (has_gradient) {
        canvas.fill_linear_gradient(bounds, gradient, corner_radius);
    } else {
        Color fill = (hovered && enabled) ? hover_bg : bg_color;
        if (corner_radius >= bounds.h * 0.5f) {
            canvas.fill_pill(bounds, fill);
        } else {
            canvas.fill_rounded_rect(bounds, corner_radius, fill);
        }
    }

    if (border_width > 0.0f && border_color.a > 0.0f) {
        if (corner_radius >= bounds.h * 0.5f) {
            canvas.stroke_pill(bounds, border_color, border_width);
        } else {
            canvas.stroke_rounded_rect(bounds, corner_radius, border_color, border_width);
        }
    }

    if (!icon_path.empty()) {
        float icon_sz = bounds.h * 0.5f;
        Rect icon_r(bounds.x + 8.0f, bounds.y + (bounds.h - icon_sz) * 0.5f, icon_sz, icon_sz);
        canvas.draw_svg(icon_path, icon_r);
    }

    if (!text.empty()) {
        Point text_pos(bounds.x + bounds.w * 0.5f, bounds.y + (bounds.h - font_size) * 0.5f - 1.0f);
        canvas.draw_text(text, text_pos, font_size, text_color, font_weight, TextAlign::Center);
    }
}

bool Button::handle_mouse_down(float x, float y, int /*button*/) {
    if (bounds.contains(x, y)) {
        if (on_click) on_click();
        return true;
    }
    return false;
}

bool Button::handle_mouse_move(float x, float y) {
    bool is_in = bounds.contains(x, y);
    if (is_in != hovered) {
        hovered = is_in;
        return true;
    }
    return false;
}

// 2. Traffic Lights
TrafficLights::TrafficLights(const Rect& b) : Widget(b) {}

void TrafficLights::render_self(Canvas& canvas) {
    // Red / Close: (18, 19, 24, 24) -> center (30, 31), r=12
    canvas.fill_circle(bounds.x + 12.0f, bounds.y + 12.0f, 12.0f, Color::from_hex(0xFF5F57));
    canvas.stroke_circle(bounds.x + 12.0f, bounds.y + 12.0f, 12.0f, Color::from_hex(0xE0443E), 1.0f);
    if(hovered) canvas.draw_svg("assets/icons/sketch/tinyX.svg", Rect(bounds.x + 8.0f, bounds.y + 8.0f, 8.0f, 8.0f));

    // Yellow / Minimize: (54, 19, 24, 24) -> center (66, 31), r=12
    canvas.fill_circle(bounds.x + 48.0f, bounds.y + 12.0f, 12.0f, Color::from_hex(0xFEBC2E));
    canvas.stroke_circle(bounds.x + 48.0f, bounds.y + 12.0f, 12.0f, Color::from_hex(0xDEA123), 1.0f);
    if(hovered) canvas.draw_svg("assets/icons/sketch/tinyMinimize.svg", Rect(bounds.x + 44.0f, bounds.y + 8.0f, 8.0f, 8.0f));

    // Green / Maximize: (90, 19, 24, 24) -> center (102, 31), r=12
    canvas.fill_circle(bounds.x + 84.0f, bounds.y + 12.0f, 12.0f, Color::from_hex(0x28C840));
    canvas.stroke_circle(bounds.x + 84.0f, bounds.y + 12.0f, 12.0f, Color::from_hex(0x1AAB29), 1.0f);
    if(hovered) canvas.draw_svg("assets/icons/sketch/tinyMaximize.svg", Rect(bounds.x + 80.0f, bounds.y + 8.0f, 8.0f, 8.0f));
}

bool TrafficLights::handle_mouse_down(float x, float y, int button) {
    if(button!=SDL_BUTTON_LEFT)return false;
    if (!bounds.contains(x, y)) return false;

    if (x >= bounds.x && x <= bounds.x + 24.0f) {
        if (on_close) on_close();
        return true;
    }
    if (x >= bounds.x + 36.0f && x <= bounds.x + 60.0f) {
        if (on_minimize) on_minimize();
        return true;
    }
    if (x >= bounds.x + 72.0f && x <= bounds.x + 96.0f) {
        if (on_maximize) on_maximize();
        return true;
    }
    return false;
}

bool TrafficLights::handle_mouse_move(float x, float y) {
    bool is_in = bounds.contains(x, y);
    if (is_in != hovered) {
        hovered = is_in;
        return true;
    }
    return false;
}

// 3. Scope Selector
ScopeSelector::ScopeSelector(const Rect& b) : Widget(b) {}

void ScopeSelector::render_self(Canvas& canvas) {
    canvas.draw_drop_shadow(bounds,16,0,1,2,Color(0,.12f,.35f,.18f));
    canvas.fill_rounded_rect(bounds,16,Color::from_hex(hovered || menu_open?0x0873EC:0x0064DF));
    canvas.draw_morph_scope({bounds.x+10,bounds.y+8,32,32},scope_morph.val,Color::White);
    const char* labels[]={"Library","Devices","Settings"};
    canvas.draw_text(labels[int(mode)],{bounds.x+64,bounds.y+12},24,Color::White,FontWeight::Light);
    float cx=bounds.right()-22,cy=bounds.y+24;
    float direction=1-2*menu_reveal.val;
    canvas.draw_line({cx-8,cy-3*direction},{cx,cy+3*direction},Color::White,1.5f);
    canvas.draw_line({cx,cy+3*direction},{cx+8,cy-3*direction},Color::White,1.5f);
}

void ScopeSelector::render_menu(Canvas& canvas) {
    if(menu_reveal.val<=.001f)return;
    auto menu=menu_bounds();
    auto cr=canvas.cr();cairo_save(cr);cairo_push_group(cr);
    cairo_translate(cr,0,-6*(1-menu_reveal.val));
    canvas.draw_drop_shadow(menu,16,0,4,14,Color(0,.12f,.3f,.22f));
    canvas.fill_rounded_rect(menu,16,Color::from_hex(0xF2FAFF));
    canvas.stroke_rounded_rect(menu,16,Color::from_hex(0xB3D6F1),1);
    const char* labels[]={"Library","Devices","Settings"};
    const char* icons[]={"assets/icons/sketch/Album.svg","assets/icons/sketch/device.svg","assets/icons/desktop/settings.svg"};
    for(int i=0;i<3;++i){
        Rect row(menu.x+6,menu.y+6+i*42,menu.w-12,40);
        if(i==int(mode) || i==menu_hover || i==menu_focus)
            canvas.fill_rounded_rect(row,10,Color::from_hex(i==menu_hover || i==menu_focus?0xD6EDFD:0xE5F3FD));
        canvas.draw_svg(icons[i],{row.x+10,row.y+8,24,24});
        canvas.draw_text(labels[i],{row.x+44,row.y+12},15,Color::Black,i==int(mode)?FontWeight::DemiBold:FontWeight::Medium);
        if(i==int(mode)){
            float x=row.right()-20,y=row.y+20;
            canvas.draw_line({x-5,y},{x-1,y+4},Color::from_hex(0x0064DF),1.8f);
            canvas.draw_line({x-1,y+4},{x+6,y-4},Color::from_hex(0x0064DF),1.8f);
        }
    }
    cairo_pop_group_to_source(cr);cairo_paint_with_alpha(cr,menu_reveal.val);cairo_restore(cr);
}
bool ScopeSelector::handle_mouse_down(float x,float y,int button) {
    if(button!=SDL_BUTTON_LEFT)return false;
    if(menu_open){
        auto menu=menu_bounds();
        if(menu.contains(x,y)){
            int i=int((y-menu.y-6)/42);
            if(i>=0 && i<3){if(on_mode_change)on_mode_change(static_cast<Mode>(i));}
        }
        close_menu();return true;
    }
    if(!bounds.contains(x,y))return false;
    menu_open=true;menu_focus=int(mode);menu_hover=-1;menu_reveal.set_target(1);
    return true;
}
bool ScopeSelector::handle_mouse_move(float x,float y) {
    hovered=bounds.contains(x,y);menu_hover=-1;
    auto menu=menu_bounds();
    if(menu_open && menu.contains(x,y)){
        int i=int((y-menu.y-6)/42);
        if(i>=0 && i<3){menu_hover=i;menu_focus=i;}
    }
    return hovered || menu_hover>=0;
}
bool ScopeSelector::handle_key_down(SDL_Keycode key) {
    if(!menu_open)return false;
    if(key==SDLK_ESCAPE){close_menu();return true;}
    if(key==SDLK_DOWN)menu_focus=(menu_focus+1)%3;
    else if(key==SDLK_UP)menu_focus=(menu_focus+2)%3;
    else if(key==SDLK_RETURN || key==SDLK_SPACE){
        if(on_mode_change)on_mode_change(static_cast<Mode>(menu_focus));
        close_menu();
    }
    return true;
}

// 4. View Mode Selector
ViewModeSelector::ViewModeSelector(const Rect& b) : Widget(b) {}

void ViewModeSelector::render_self(Canvas& canvas) {
    canvas.fill_rounded_rect(bounds, 8.0f, Color::from_hex(0x869FFF));
    float half_w = bounds.w * 0.5f;
    Rect active_r = (view == View::Icons) ? Rect(bounds.x, bounds.y, half_w, bounds.h) : Rect(bounds.x + half_w, bounds.y, half_w, bounds.h);
    canvas.fill_rounded_rect(active_r, 8.0f, Color::from_hex(0x0048FF));

    canvas.draw_text("Icons", Point(bounds.x + half_w * 0.5f, bounds.y + 4.0f), 14.0f, Color::White, FontWeight::DemiBold, TextAlign::Center);
    canvas.draw_text("List", Point(bounds.x + half_w + half_w * 0.5f, bounds.y + 4.0f), 14.0f, Color::White, FontWeight::DemiBold, TextAlign::Center);
}

bool ViewModeSelector::handle_mouse_down(float x, float y, int /*button*/) {
    if (!bounds.contains(x, y)) return false;
    float half_w = bounds.w * 0.5f;
    if (x < bounds.x + half_w) {
        view = View::Icons;
    } else {
        view = View::List;
    }
    if (on_view_change) on_view_change(view);
    return true;
}

} // namespace MusicUI
