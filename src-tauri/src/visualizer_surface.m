#import <AppKit/AppKit.h>
#import <WebKit/WebKit.h>
#import <QuartzCore/CAMetalLayer.h>
#import <QuartzCore/CATransaction.h>
#import <Metal/Metal.h>
typedef void (*MusicOSSurfaceUpdate)(void *, uint32_t, uint32_t, bool);
typedef void (*MusicOSSurfaceRelease)(void *);
typedef void (*MusicOSSurfacePresented)(void *, double);
@interface MusicOSVisualizerView : NSView
@property(nonatomic) void *context;
@property(nonatomic) MusicOSSurfaceUpdate update;
@property(nonatomic) MusicOSSurfaceRelease releaseContext;
@property(nonatomic) MusicOSSurfacePresented presented;
@property(nonatomic,weak) WKWebView *web;
@property(nonatomic) BOOL priorDrawsBackground;
@property(nonatomic) BOOL priorOpaque;
@property(nonatomic,strong) NSColor *priorBackgroundColor;
@property(nonatomic,strong) NSMutableArray *observers;
@end
// Public drawable presentation callbacks measure displayed pacing; no display
// link, polling timer, swizzling, or additional drawing work is introduced.
@interface MusicOSVisualizerLayer : CAMetalLayer
@property(nonatomic,weak) MusicOSVisualizerView *owner;
@end
@implementation MusicOSVisualizerLayer
-(id<CAMetalDrawable>)nextDrawable {
 id<CAMetalDrawable> drawable=[super nextDrawable];
 __weak MusicOSVisualizerView *weakView=self.owner;
 // Runtime selector gating also supports deployment targets predating this
 // optional measurement API, without depending on Clang's availability helper.
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunguarded-availability-new"
 if ([drawable respondsToSelector:@selector(addPresentedHandler:)]) [drawable addPresentedHandler:^(id<MTLDrawable> shown){
  MusicOSVisualizerView *view=weakView;
  if(view.update && view.presented && view.context && shown.presentedTime>0) view.presented(view.context,shown.presentedTime);
 }];
#pragma clang diagnostic pop
 return drawable;
}
@end
@implementation MusicOSVisualizerView
// A layer-backed view lets AppKit own the root layer's anchor, bounds and
// backing scale. Assigning/repositioning a hosted root layer offsets it by half
// the window on AppKit's zero-anchor layers.
-(CALayer *)makeBackingLayer {
 MusicOSVisualizerLayer *layer=[MusicOSVisualizerLayer layer];layer.owner=self;layer.opaque=YES;
 CGColorSpaceRef colorspace=CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
 layer.colorspace=colorspace;CGColorSpaceRelease(colorspace);
 layer.wantsExtendedDynamicRangeContent=NO;
 return layer;
}
-(BOOL)wantsUpdateLayer { return YES; }
-(void)updateLayer { /* Metal owns drawing; AppKit owns geometry. */ }
-(BOOL)isOpaque { return YES; }
-(BOOL)acceptsFirstResponder { return NO; }
-(NSView *)hitTest:(NSPoint)point { return nil; }
-(void)publish {
 if (!self.update) return;
 NSRect pixels=[self convertRectToBacking:self.bounds];
 BOOL visible=self.window && self.window.visible && !self.window.miniaturized && (self.window.occlusionState & NSWindowOcclusionStateVisible) && !self.hidden;
 self.update(self.context,(uint32_t)MAX(1,pixels.size.width),(uint32_t)MAX(1,pixels.size.height),visible);
}
-(void)setFrameSize:(NSSize)size { [super setFrameSize:size]; [self publish]; }
-(void)layout { [super layout]; [self publish]; }
-(void)viewDidChangeBackingProperties { [super viewDidChangeBackingProperties]; [self publish]; }
-(void)viewDidMoveToWindow { [super viewDidMoveToWindow]; [self publish]; }
-(void)dealloc {
 for(id observer in self.observers) [[NSNotificationCenter defaultCenter] removeObserver:observer];
 if(self.context && self.releaseContext) self.releaseContext(self.context);
}
@end
// The view retains its callback context until deallocation. No polling timer or
// hit-testing view is left behind when the native renderer is inactive.
void *musicos_visualizer_attach(void *web,void *context,MusicOSSurfaceUpdate update,MusicOSSurfaceRelease releaseContext,MusicOSSurfacePresented presented) {
 if (![NSThread isMainThread]) return NULL;
 WKWebView *wk=(__bridge WKWebView *)web;
 NSView *parent=wk.window.contentView ?: wk.superview;if (!parent) return NULL;
 NSView *overlay=wk;while(overlay.superview && overlay.superview!=parent)overlay=overlay.superview;
 if(overlay.superview!=parent)return NULL;
 MusicOSVisualizerView *view=[[MusicOSVisualizerView alloc] initWithFrame:parent.bounds];
 view.autoresizingMask=NSViewWidthSizable|NSViewHeightSizable;view.wantsLayer=YES;
 view.web=wk;view.priorDrawsBackground=[[wk valueForKey:@"drawsBackground"] boolValue];view.priorOpaque=wk.window.opaque;view.priorBackgroundColor=wk.window.backgroundColor;
 view.context=context;view.update=update;view.releaseContext=releaseContext;view.presented=presented;
 [parent addSubview:view positioned:NSWindowBelow relativeTo:overlay];
 [wk setValue:@NO forKey:@"drawsBackground"];wk.window.opaque=NO;wk.window.backgroundColor=NSColor.blackColor;
 view.observers=[NSMutableArray array];__weak MusicOSVisualizerView *weakView=view;
 for(NSNotificationName name in @[NSWindowDidChangeOcclusionStateNotification,NSWindowDidMiniaturizeNotification,NSWindowDidDeminiaturizeNotification,NSWindowDidResizeNotification,NSWindowDidChangeScreenNotification,NSWindowWillCloseNotification]){
  id observer=[[NSNotificationCenter defaultCenter] addObserverForName:name object:wk.window queue:[NSOperationQueue mainQueue] usingBlock:^(NSNotification *note){if([note.name isEqualToString:NSWindowWillCloseNotification])weakView.hidden=YES;[weakView publish];}];
  [view.observers addObject:observer];
 }
 [view publish];return (__bridge_retained void *)view;
}
void musicos_visualizer_detach(void *raw) {
 if(!raw)return;
 void(^detach)(void)=^{
  MusicOSVisualizerView *view=(__bridge_transfer MusicOSVisualizerView *)raw;
  view.update=NULL;[view removeFromSuperview];
  WKWebView *wk=view.web;if(wk){[wk setValue:@(view.priorDrawsBackground) forKey:@"drawsBackground"];wk.window.opaque=view.priorOpaque;wk.window.backgroundColor=view.priorBackgroundColor;}
 };
 if([NSThread isMainThread])detach();else dispatch_async(dispatch_get_main_queue(),detach);
}
