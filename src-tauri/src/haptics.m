#import <AppKit/AppKit.h>
// AppKit decides whether the hardware and system feedback setting support this.
void musicos_haptic(int pattern) {
    if (![NSThread isMainThread] || ![NSApp isActive]) return;
    NSHapticFeedbackPattern p = pattern == 1 ? NSHapticFeedbackPatternAlignment : pattern == 2 ? NSHapticFeedbackPatternLevelChange : NSHapticFeedbackPatternGeneric;
    [[NSHapticFeedbackManager defaultPerformer] performFeedbackPattern:p performanceTime:NSHapticFeedbackPerformanceTimeNow];
}
