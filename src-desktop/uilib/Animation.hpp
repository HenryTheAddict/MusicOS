#pragma once
#include <cmath>
#include <algorithm>

namespace MusicUI {

class SpringAnim {
public:
    float val = 0.0f;
    float target = 0.0f;
    float vel = 0.0f;
    float tension = 580.0f;   // Snappy frequency
    float damping = 38.0f;   // Poppy bounciness

    SpringAnim(float init_val = 0.0f, float t = 580.0f, float d = 38.0f)
        : val(init_val), target(init_val), vel(0.0f), tension(t), damping(d) {}

    void set_target(float t) {
        target = t;
    }

    void snap(float v) {
        val = v;
        target = v;
        vel = 0.0f;
    }

    void update(float dt) {
        if (dt <= 0 || !std::isfinite(dt)) return;
        // Exact damped-spring solution: independent of frame rate and stalls.
        double x = val - target, v = vel, half = damping * .5;
        double discriminant = half * half - tension;
        double next_x, next_v;
        if (std::abs(discriminant) < .0001) {
            double e = std::exp(-half * dt), b = v + half * x;
            next_x = (x + b * dt) * e;
            next_v = (v - half * b * dt) * e;
        } else if (discriminant < 0) {
            double w = std::sqrt(-discriminant), e = std::exp(-half * dt);
            double a = x, b = (v + half * x) / w;
            double co = std::cos(w * dt), si = std::sin(w * dt);
            next_x = e * (a * co + b * si);
            next_v = e * ((b * w - half * a) * co - (a * w + half * b) * si);
        } else {
            double root = std::sqrt(discriminant), r1 = -half + root, r2 = -half - root;
            double a = (v - r2 * x) / (r1 - r2), b = x - a;
            next_x = a * std::exp(r1 * dt) + b * std::exp(r2 * dt);
            next_v = a * r1 * std::exp(r1 * dt) + b * r2 * std::exp(r2 * dt);
        }
        val = float(target + next_x); vel = float(next_v);
        if (std::abs(vel) < 0.0002f && std::abs(val - target) < 0.0002f) {
            val = target;
            vel = 0.0f;
        }
    }

    bool is_animating() const {
        return val != target || std::abs(vel) > 0.0002f;
    }
};

class TimedAnim {
public:
    float val = 0.0f;
    float start_val = 0.0f;
    float target_val = 0.0f;
    float elapsed = 0.4f;
    float duration = 0.4f; // 0.4s standard
    bool animating = false;

    TimedAnim(float init_val = 0.0f, float dur = 0.18f)
        : val(init_val), start_val(init_val), target_val(init_val), elapsed(dur), duration(dur), animating(false) {}

    void set_target(float t) {
        if (std::abs(target_val - t) < 0.0001f ) return;
        start_val = val;
        target_val = t;
        elapsed = 0.0f;
        animating = true;
    }

    void snap(float v) {
        val = v;
        start_val = v;
        target_val = v;
        elapsed = duration;
        animating = false;
    }

    void update(float dt) {
        if (!animating) return;
        elapsed += dt;
        float progress = elapsed / duration;
        if (progress >= 1.0f) {
            val = target_val;
            animating = false;
        } else {
            float p = progress;
            float eased = (p < 0.5f) ? (4.0f * p * p * p) : (1.0f - std::pow(-2.0f * p + 2.0f, 3.0f) * 0.5f);
            val = start_val + (target_val - start_val) * eased;
        }
    }

    bool is_animating() const {
        return animating;
    }
};

} // namespace MusicUI
