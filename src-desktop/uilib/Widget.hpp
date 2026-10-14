#pragma once
#include "Types.hpp"
#include "Canvas.hpp"
#include <vector>
#include <memory>
#include <functional>

namespace MusicUI {

class Widget : public std::enable_shared_from_this<Widget> {
public:
    Rect bounds{0, 0, 0, 0};
    bool visible = true;
    bool enabled = true;
    bool hovered = false;
    bool pressed = false;

    Widget* parent = nullptr;
    std::vector<std::shared_ptr<Widget>> children;

    std::function<void()> on_click;

    Widget() = default;
    Widget(const Rect& b) : bounds(b) {}
    virtual ~Widget() = default;

    void add_child(std::shared_ptr<Widget> child) {
        if (!child) return;
        child->parent = this;
        children.push_back(child);
    }

    virtual void render(Canvas& canvas) {
        if (!visible) return;
        render_self(canvas);
        for (auto& child : children) {
            if (child->visible) {
                child->render(canvas);
            }
        }
    }

    virtual void render_self(Canvas& /*canvas*/) {}

    virtual bool handle_mouse_down(float x, float y, int button) {
        if (!visible || !enabled || !bounds.contains(x, y)) return false;

        // Give children first chance
        for (auto it = children.rbegin(); it != children.rend(); ++it) {
            if ((*it)->handle_mouse_down(x, y, button)) {
                return true;
            }
        }

        pressed = true;
        return true;
    }

    virtual bool handle_mouse_up(float x, float y, int button) {
        if (!visible || !enabled) return false;

        bool handled = false;
        for (auto it = children.rbegin(); it != children.rend(); ++it) {
            if ((*it)->handle_mouse_up(x, y, button)) {
                handled = true;
                break;
            }
        }

        if (pressed && bounds.contains(x, y)) {
            pressed = false;
            if (on_click) {
                on_click();
            }
            return true;
        }
        pressed = false;
        return handled;
    }

    virtual bool handle_mouse_move(float x, float y) {
        if (!visible || !enabled) {
            hovered = false;
            return false;
        }

        bool was_hovered = hovered;
        hovered = bounds.contains(x, y);

        bool handled = false;
        for (auto it = children.rbegin(); it != children.rend(); ++it) {
            if ((*it)->handle_mouse_move(x, y)) {
                handled = true;
            }
        }

        return handled || (hovered != was_hovered);
    }
};

} // namespace MusicUI
