#include "../Components.hpp"
#include "../LibraryModel.hpp"
#include <iostream>
#include <algorithm>

namespace MusicUI {

NowPlayingPill::NowPlayingPill(const Rect& b) : Widget(b) {}

void NowPlayingPill::render_self(Canvas& canvas) {
    canvas.draw_drop_shadow(bounds, 16.0f, 0.0f, 1.0f, 2.0f, Color(0, 0, 0, 0.25f));
    canvas.fill_rounded_rect(bounds, 16.0f, Color::White);

    // Artwork: (376, 15, 32, 32)
    Rect art_r(bounds.x + 8.0f, bounds.y + 8.0f, 32.0f, 32.0f);
    if(state.cover_path.empty()) canvas.draw_svg("assets/icons/sketch/noCOVER.svg",art_r);
    else canvas.draw_image(state.cover_path, art_r, 4.0f);

    auto fit_info=[&](std::string text,float size,FontWeight weight){
        float available=std::max(20.f,bounds.w-185);
        if(canvas.measure_text(text,size,weight).w>available){
            while(!text.empty() && canvas.measure_text(text+"…",size,weight).w>available)erase_character(text);
            text+="…";
        }
        return text;
    };
    // Title: (412, 14 in window -> bounds.x + 44.0f, bounds.y + 7.0f)
    canvas.draw_text(fit_info(state.title,12,FontWeight::ExtraBold), Point(bounds.x + 44.0f, bounds.y + 7.0f), 12.0f, Color::Black, FontWeight::ExtraBold);

    // Artist: (412, 29 in window -> bounds.x + 44.0f, bounds.y + 22.0f)
    canvas.draw_text(fit_info(state.artist,8,FontWeight::Medium), Point(bounds.x + 44.0f, bounds.y + 22.0f), 8.0f, Color::from_hex(0x707070), FontWeight::Medium);

    // Controls are anchored to the right edge:
    // When bounds.w == 334.0f, right_base = bounds.x + 201.0f
    float right_base = bounds.x + bounds.w - 133.0f;

    // Progress bar stretches flexibly between info text and right controls
    float bar_w = std::max(40.0f, bounds.w - 189.0f);
    Rect bar_track(bounds.x + 44.0f, bounds.y + 34.0f, bar_w, 4.0f);
    canvas.fill_rounded_rect(bar_track, 2.0f, Color::from_hex(0xD6EFFD));
    float prog_w = std::max(0.0f, std::min(bar_w, state.progress * bar_w));
    canvas.fill_rounded_rect(Rect(bar_track.x, bar_track.y, prog_w, 4.0f), 2.0f, Color::from_hex(0x00A6FF));

    // Tactile draggable scrubber thumb knob
    float thumb_r = is_scrubbing ? 6.0f : 4.5f;
    Point thumb_pt(bar_track.x + prog_w, bar_track.y + 2.0f);
    canvas.fill_circle(thumb_pt.x, thumb_pt.y, thumb_r, Color::from_hex(0x0088FF));
    canvas.stroke_circle(thumb_pt.x, thumb_pt.y, thumb_r, Color::White, 1.5f);

    // Queue icon
    canvas.draw_svg("assets/icons/sketch/queue.svg", Rect(right_base, bounds.y + 8.0f, 16.0f, 16.0f));

    // Waveform / Music Note (0.4s vector morph)
    canvas.draw_morph_note_waveform(Rect(right_base + 16.0f, bounds.y + 8.0f, 16.0f, 16.0f), note_morph.val, Color::from_hex(0x0088FF), wave_phase);

    // Repeat 0.4s vector morph
    Color rep_col = state.repeat_on ? Color::from_hex(0x0070FF) : Color::from_hex(0x707070);
    canvas.draw_morph_repeat(Rect(right_base, bounds.y + 24.0f, 16.0f, 16.0f), repeat_morph.val, rep_col);

    // Shuffle 0.4s vector morph
    Color shuf_col = state.shuffle_on ? Color::from_hex(0x0070FF) : Color::from_hex(0x707070);
    canvas.draw_morph_shuffle(Rect(right_base + 16.0f, bounds.y + 24.0f, 16.0f, 16.0f), shuffle_morph.val, shuf_col);

    // Transport buttons:
    // Prev: 32x32
    float prev_cx = right_base + 48.0f;
    float prev_cy = bounds.y + 24.0f;
    if (prev_pop.val != 1.0f) {
        canvas.save();
        canvas.translate(prev_cx, prev_cy);
        canvas.scale(prev_pop.val, prev_pop.val);
        canvas.translate(-prev_cx, -prev_cy);
        canvas.draw_svg("assets/icons/sketch/prev.svg", Rect(right_base + 32.0f, bounds.y + 8.0f, 32.0f, 32.0f));
        canvas.restore();
    } else {
        canvas.draw_svg("assets/icons/sketch/prev.svg", Rect(right_base + 32.0f, bounds.y + 8.0f, 32.0f, 32.0f));
    }

    // Play/Pause: 32x32 (0.4s vector morphing between play and pause)
    float play_cx = right_base + 80.0f;
    float play_cy = bounds.y + 24.0f;
    Rect play_rect(right_base + 64.0f, bounds.y + 8.0f, 32.0f, 32.0f);
    if (play_pop.val != 1.0f) {
        canvas.save();
        canvas.translate(play_cx, play_cy);
        canvas.scale(play_pop.val, play_pop.val);
        canvas.translate(-play_cx, -play_cy);
        canvas.draw_morph_play_pause(play_rect, play_morph.val, Color::Black);
        canvas.restore();
    } else {
        canvas.draw_morph_play_pause(play_rect, play_morph.val, Color::Black);
    }

    // Next: 32x32
    float next_cx = right_base + 112.0f;
    float next_cy = bounds.y + 24.0f;
    if (next_pop.val != 1.0f) {
        canvas.save();
        canvas.translate(next_cx, next_cy);
        canvas.scale(next_pop.val, next_pop.val);
        canvas.translate(-next_cx, -next_cy);
        canvas.draw_svg("assets/icons/sketch/next.svg", Rect(right_base + 96.0f, bounds.y + 8.0f, 32.0f, 32.0f));
        canvas.restore();
    } else {
        canvas.draw_svg("assets/icons/sketch/next.svg", Rect(right_base + 96.0f, bounds.y + 8.0f, 32.0f, 32.0f));
    }
}

bool NowPlayingPill::handle_mouse_down(float x, float y, int /*button*/) {
    if (!bounds.contains(x, y)) return false;

    float right_base = bounds.x + bounds.w - 133.0f;

    // Repeat (triggers 0.4s vector morph)
    if (x >= right_base && x <= right_base + 16.0f && y >= bounds.y + 24.0f && y <= bounds.y + 40.0f) {
        state.repeat_on = !state.repeat_on;
        repeat_morph.set_target(state.repeat_on ? 1.0f : 0.0f);
        if (on_repeat_toggle) on_repeat_toggle();
        return true;
    }
    // Shuffle (triggers 0.4s vector morph)
    if (x >= right_base + 16.0f && x <= right_base + 32.0f && y >= bounds.y + 24.0f && y <= bounds.y + 40.0f) {
        state.shuffle_on = !state.shuffle_on;
        shuffle_morph.set_target(state.shuffle_on ? 1.0f : 0.0f);
        if (on_shuffle_toggle) on_shuffle_toggle();
        return true;
    }
    // Prev
    if (x >= right_base + 32.0f && x <= right_base + 64.0f && y >= bounds.y + 8.0f && y <= bounds.y + 40.0f) {
        prev_pop.snap(0.95f);
        prev_pop.set_target(1.0f);
        if (on_prev) on_prev();
        return true;
    }
    // Play/Pause (triggers 0.4s vector morph)
    if (x >= right_base + 64.0f && x <= right_base + 96.0f && y >= bounds.y + 8.0f && y <= bounds.y + 40.0f) {
        play_pop.snap(0.95f);
        play_pop.set_target(1.0f);
        state.is_playing = !state.is_playing;
        play_morph.set_target(state.is_playing ? 1.0f : 0.0f);
        note_morph.set_target(state.is_playing ? 1.0f : 0.0f);
        if (on_play_pause) on_play_pause();
        return true;
    }
    // Next
    if (x >= right_base + 96.0f && x <= right_base + 128.0f && y >= bounds.y + 8.0f && y <= bounds.y + 40.0f) {
        next_pop.snap(0.95f);
        next_pop.set_target(1.0f);
        if (on_next) on_next();
        return true;
    }

    // Seek bar: generous 24px vertical hit area
    float bar_w = std::max(40.0f, bounds.w - 189.0f);
    if (x >= bounds.x + 38.0f && x <= bounds.x + 44.0f + bar_w + 6.0f && y >= bounds.y + 24.0f && y <= bounds.y + 48.0f) {
        is_scrubbing = true;
        state.progress = std::max(0.0f, std::min(1.0f, (x - (bounds.x + 44.0f)) / bar_w));
        if (on_seek) on_seek(state.progress);
        return true;
    }

    return false;
}

bool NowPlayingPill::handle_mouse_move(float x, float /*y*/) {
    if (is_scrubbing) {
        float bar_w = std::max(40.0f, bounds.w - 189.0f);
        state.progress = std::max(0.0f, std::min(1.0f, (x - (bounds.x + 44.0f)) / bar_w));
        if (on_seek) on_seek(state.progress);
        return true;
    }
    return false;
}

bool NowPlayingPill::handle_mouse_up(float x, float /*y*/, int /*button*/) {
    if (is_scrubbing) {
        is_scrubbing = false;
        float bar_w = std::max(40.0f, bounds.w - 189.0f);
        state.progress = std::max(0.0f, std::min(1.0f, (x - (bounds.x + 44.0f)) / bar_w));
        if (on_seek) on_seek(state.progress);
        return true;
    }
    return false;
}

} // namespace MusicUI
