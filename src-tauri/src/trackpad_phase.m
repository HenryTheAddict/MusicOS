#import <AppKit/AppKit.h>
static id musicosScrollMonitor;
void musicos_trackpad_install(void *rawView, void (*callback)(int), int remove) {
 if (musicosScrollMonitor) { [NSEvent removeMonitor:musicosScrollMonitor]; musicosScrollMonitor=nil; }
 if (remove) return;
 __weak NSView *view=(__bridge NSView*)rawView;
 musicosScrollMonitor=[NSEvent addLocalMonitorForEventsMatchingMask:NSEventMaskScrollWheel handler:^NSEvent *(NSEvent *event) {
  if (!view || event.window!=view.window || !event.hasPreciseScrollingDeltas) return event;
  if (event.momentumPhase & NSEventPhaseBegan) callback(4);
  else if (event.momentumPhase & (NSEventPhaseEnded | NSEventPhaseCancelled)) callback(5);
  else if (event.phase & NSEventPhaseBegan) callback(1);
  else if (event.phase & NSEventPhaseCancelled) callback(3);
  else if (event.phase & NSEventPhaseEnded) callback(2);
  return event;
 }];
}
