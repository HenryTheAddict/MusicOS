// Read-only Core Audio route inspection, called on the playback worker.
#import <CoreAudio/CoreAudio.h>
#include <math.h>
int mos_audio_output_route(unsigned int *device, double *rate) {
    AudioObjectPropertyAddress address = {kAudioHardwarePropertyDefaultOutputDevice,
        kAudioObjectPropertyScopeGlobal, kAudioObjectPropertyElementMain};
    UInt32 size = sizeof(*device);
    if (AudioObjectGetPropertyData(kAudioObjectSystemObject, &address, 0, NULL, &size, device)
        || *device == kAudioObjectUnknown) return -1;
    UInt32 alive = 0;
    address.mSelector = kAudioDevicePropertyDeviceIsAlive;
    size = sizeof(alive);
    if (AudioObjectGetPropertyData(*device, &address, 0, NULL, &size, &alive) || !alive) return -1;
    address.mSelector = kAudioDevicePropertyNominalSampleRate;
    size = sizeof(*rate);
    if (AudioObjectGetPropertyData(*device, &address, 0, NULL, &size, rate)
        || !isfinite(*rate) || *rate <= 0) return -1;
    return 0;
}
