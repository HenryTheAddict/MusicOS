#pragma once
#include <SDL3/SDL.h>
#include <iostream>
#include <vector>
#include <cmath>
#include <atomic>
#include <algorithm>

namespace MusicUI {

class AudioEngine {
public:
    static AudioEngine& instance() {
        static AudioEngine s_instance;
        return s_instance;
    }

    bool init() {
        if (m_initialized) return true;
        if (!SDL_InitSubSystem(SDL_INIT_AUDIO)) {
            std::cerr << "[AudioEngine] SDL_InitSubSystem failed: " << SDL_GetError() << "\n";
            return false;
        }
        SDL_AudioSpec spec;
        SDL_zero(spec);
        spec.channels = 2;
        spec.format = SDL_AUDIO_F32;
        spec.freq = 44100;

        m_stream = SDL_OpenAudioDeviceStream(SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK, &spec, audio_callback, this);
        if (!m_stream) {
            std::cerr << "[AudioEngine] SDL_OpenAudioDeviceStream failed: " << SDL_GetError() << "\n";
            return false;
        }
        SDL_ResumeAudioStreamDevice(m_stream);
        m_initialized = true;
        std::cout << "[AudioEngine] SDL3 Audio device stream initialized successfully (44.1kHz Stereo F32).\n";
        return true;
    }

    void shutdown() {
        if (m_stream) {
            SDL_DestroyAudioStream(m_stream);
            m_stream = nullptr;
        }
        m_initialized = false;
    }

    void play() {
        m_playing = true;
        if (m_stream) {
            SDL_ResumeAudioStreamDevice(m_stream);
        }
    }

    void pause() {
        m_playing = false;
    }

    void seek(float progress) {
        // Smoothly reposition chord sequence without audio clicks
        float prog = std::max(0.0f, std::min(1.0f, progress));
        m_song_time.store(prog * 180.0);
    }

    void set_playing(bool p) {
        if (p) play();
        else pause();
    }

    bool is_playing() const {
        return m_playing.load();
    }

    void set_volume(float vol) {
        m_volume.store(std::max(0.0f, std::min(1.0f, vol)));
    }

    void set_vibe(int vibe_idx) {
        m_vibe.store(vibe_idx);
    }

private:
    AudioEngine() = default;
    ~AudioEngine() { shutdown(); }

    static void SDLCALL audio_callback(void* userdata, SDL_AudioStream* stream, int additional_amount, int /*total_amount*/) {
        auto* self = static_cast<AudioEngine*>(userdata);
        if (!self) return;

        int samples_needed = additional_amount / (int)sizeof(float);
        if (samples_needed <= 0) samples_needed = 1024;

        std::vector<float> buffer(samples_needed, 0.0f);

        if (self->m_playing.load()) {
            float master_vol = self->m_volume.load() * 0.65f;
            int vibe = self->m_vibe.load();

            // 4 Distinct Warm Harmonized Chord Progressions
            // Each chord has 4 notes (Root, 3rd, 5th, 7th/9th)
            static const float PROGRESSIONS[4][4][4] = {
                // Vibe 0: Cmaj7 -> Am7 -> Fmaj7 -> Gsus4 (Dreamy Synthwave / Chill)
                {
                    { 261.63f, 329.63f, 392.00f, 493.88f },
                    { 220.00f, 261.63f, 329.63f, 392.00f },
                    { 174.61f, 220.00f, 261.63f, 349.23f },
                    { 196.00f, 261.63f, 293.66f, 392.00f }
                },
                // Vibe 1: Dm7 -> G7 -> Cmaj7 -> A7 (Neo-Soul / Daft Punk Funk)
                {
                    { 293.66f, 349.23f, 440.00f, 523.25f },
                    { 196.00f, 246.94f, 293.66f, 349.23f },
                    { 261.63f, 329.63f, 392.00f, 493.88f },
                    { 220.00f, 277.18f, 329.63f, 392.00f }
                },
                // Vibe 2: Fmaj7 -> Em7 -> Dm7 -> Cmaj7 (Melancholy Hill / Gorillaz Vibe)
                {
                    { 174.61f, 220.00f, 261.63f, 349.23f },
                    { 164.81f, 196.00f, 246.94f, 329.63f },
                    { 146.83f, 174.61f, 220.00f, 261.63f },
                    { 130.81f, 164.81f, 196.00f, 246.94f }
                },
                // Vibe 3: Bbmaj7 -> Gm7 -> Ebmaj7 -> F7 (Tame Impala Psychedelic Pop)
                {
                    { 233.08f, 293.66f, 349.23f, 440.00f },
                    { 196.00f, 233.08f, 293.66f, 349.23f },
                    { 155.56f, 196.00f, 233.08f, 293.66f },
                    { 174.61f, 220.00f, 261.63f, 311.13f }
                }
            };

            const auto& cur_prog = PROGRESSIONS[vibe % 4];
            double cur_time = self->m_song_time.load();

            for (int i = 0; i < samples_needed; i += 2) {
                cur_time += 1.0 / 44100.0;
                int chord_step = ((int)(cur_time * 0.5)) % 4;
                const float* chord = cur_prog[chord_step];

                // Chord notes synthesis with wrapped phase accumulators
                float sample_l = 0.0f;
                float sample_r = 0.0f;

                for (int note = 0; note < 4; ++note) {
                    float freq = chord[note];
                    self->m_phases[note] = std::fmod(self->m_phases[note] + (double)freq / 44100.0, 1.0);

                    // Fundamental sine + soft warm 2nd harmonic
                    float sine_l = std::sin((float)(self->m_phases[note] * 2.0 * M_PI));
                    float harm2_l = std::sin((float)(self->m_phases[note] * 4.0 * M_PI)) * 0.22f;
                    float harm3_l = std::sin((float)(self->m_phases[note] * 6.0 * M_PI)) * 0.08f;

                    // Slight spatial stereo chorus offset on right ear
                    double r_phase = std::fmod(self->m_phases[note] + 0.004 * (note + 1), 1.0);
                    float sine_r = std::sin((float)(r_phase * 2.0 * M_PI));
                    float harm2_r = std::sin((float)(r_phase * 4.0 * M_PI)) * 0.22f;
                    float harm3_r = std::sin((float)(r_phase * 6.0 * M_PI)) * 0.08f;

                    sample_l += (sine_l + harm2_l + harm3_l) * 0.20f;
                    sample_r += (sine_r + harm2_r + harm3_r) * 0.20f;
                }

                // Sub-bass fundamental (octave below lowest chord note)
                float bass_freq = chord[0] * 0.5f;
                self->m_bass_phase = std::fmod(self->m_bass_phase + (double)bass_freq / 44100.0, 1.0);
                float bass = std::sin((float)(self->m_bass_phase * 2.0 * M_PI)) * 0.28f;
                sample_l += bass;
                sample_r += bass;

                // Gentle organic breathing tremolo LFO (1.5 Hz)
                self->m_lfo_phase = std::fmod(self->m_lfo_phase + 1.5 / 44100.0, 1.0);
                float lfo = 0.88f + 0.12f * std::sin((float)(self->m_lfo_phase * 2.0 * M_PI));

                // Soft saturation limiter to prevent any digital clipping
                float out_l = std::tanh(sample_l * master_vol * lfo);
                float out_r = std::tanh(sample_r * master_vol * lfo);

                buffer[i] = out_l;
                buffer[i + 1] = out_r;
            }

            self->m_song_time.store(cur_time);
        }

        SDL_PutAudioStreamData(stream, buffer.data(), (int)(buffer.size() * sizeof(float)));
    }

    std::atomic<bool> m_playing{true}; // Auto-start playing warm chords
    std::atomic<float> m_volume{0.75f};
    std::atomic<int> m_vibe{0};
    std::atomic<double> m_song_time{0.0};
    bool m_initialized = false;
    SDL_AudioStream* m_stream = nullptr;

    double m_phases[4]{0.0, 0.0, 0.0, 0.0};
    double m_bass_phase = 0.0;
    double m_lfo_phase = 0.0;
};

} // namespace MusicUI
