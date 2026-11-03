// Public AppKit/WebKit routing for focused-window musicOS shortcuts.
// A native Edit menu may consume Command-V before DOM keydown. Check the
// actual focused editor before intercepting; forward editing events unchanged.
#import <AppKit/AppKit.h>
#import <WebKit/WebKit.h>
typedef void (*ShortcutDispatch)(void *, unsigned char);
typedef void (*ShortcutRelease)(void *);
@interface MusicOSPlayerShortcuts : NSObject <WKScriptMessageHandler>
@property(nonatomic,weak) WKWebView *web;
@property(nonatomic,strong) id monitor;
@property(nonatomic) void *context;
@property(nonatomic) ShortcutDispatch dispatch;
@property(nonatomic) ShortcutRelease releaseContext;
@property(nonatomic) BOOL enabled;
@property(nonatomic) BOOL forwarding;
@property(nonatomic) BOOL editing;
@end
@implementation MusicOSPlayerShortcuts
-(void)dealloc {if(_monitor)[NSEvent removeMonitor:_monitor];if(_releaseContext)_releaseContext(_context);}
-(void)userContentController:(WKUserContentController *)controller didReceiveScriptMessage:(WKScriptMessage *)message {
 if([message.name isEqualToString:@"musicosPlayerShortcutFocus"] && [message.body isKindOfClass:[NSNumber class]])self.editing=[message.body boolValue];
}
-(void)forward:(NSEvent *)event {
 if(!self.web.window || NSApp.keyWindow!=self.web.window)return;
 self.forwarding=YES;if(![NSApp.mainMenu performKeyEquivalent:event])[NSApp sendEvent:event];self.forwarding=NO;
}
@end
void *musicos_shortcuts_attach(void *rawWeb,void *context,ShortcutDispatch dispatch,ShortcutRelease releaseContext){
 NSCAssert([NSThread isMainThread],@"Shortcut routing belongs on the main thread");
 MusicOSPlayerShortcuts *owner=[MusicOSPlayerShortcuts new];owner.web=(__bridge WKWebView *)rawWeb;owner.context=context;owner.dispatch=dispatch;owner.releaseContext=releaseContext;owner.enabled=YES;
 NSString *focusScript=@"(()=>{if(window.__musicosShortcutFocus)return;window.__musicosShortcutFocus=true;const update=()=>{const e=document.activeElement;window.webkit.messageHandlers.musicosPlayerShortcutFocus?.postMessage(!!(e&&e.closest('input:not([type=range]):not([type=checkbox]):not([type=radio]):not([type=button]):not([type=submit]):not([type=reset]):not([type=image]):not([type=color]):not([type=file]),textarea,select,[contenteditable]:not([contenteditable=false]),[role=textbox],[role=combobox]')));};document.addEventListener('focusin',update,true);document.addEventListener('focusout',()=>queueMicrotask(update),true);update();})()";
 [owner.web.configuration.userContentController addScriptMessageHandler:owner name:@"musicosPlayerShortcutFocus"];
 [owner.web.configuration.userContentController addUserScript:[[WKUserScript alloc] initWithSource:focusScript injectionTime:WKUserScriptInjectionTimeAtDocumentEnd forMainFrameOnly:YES]];
 [owner.web evaluateJavaScript:focusScript completionHandler:nil];
 __weak MusicOSPlayerShortcuts *weakOwner=owner;
 owner.monitor=[NSEvent addLocalMonitorForEventsMatchingMask:NSEventMaskKeyDown handler:^NSEvent *(NSEvent *event){
  MusicOSPlayerShortcuts *self=weakOwner;
  if(!self || !self.enabled || self.forwarding || !self.web.window || NSApp.keyWindow!=self.web.window || event.window!=self.web.window)return event;
  NSEventModifierFlags flags=event.modifierFlags & NSEventModifierFlagDeviceIndependentFlagsMask;
  if(!(flags & NSEventModifierFlagCommand) || (flags & (NSEventModifierFlagControl|NSEventModifierFlagOption|NSEventModifierFlagShift)))return event;
  NSString *key=event.charactersIgnoringModifiers.lowercaseString;
  unsigned char action=[key isEqualToString:@"p"]?1:[key isEqualToString:@"n"]?2:[key isEqualToString:@"v"]?3:[key isEqualToString:@"b"]?4:0;
  if(!action || self.editing)return event;
  // Check on every press rather than relying on an asynchronously cached
  // focus flag, which could steal Paste immediately after focusing an input.
  [self.web evaluateJavaScript:@"(()=>{const e=document.activeElement;return !!(e&&e.closest('input:not([type=range]):not([type=checkbox]):not([type=radio]):not([type=button]):not([type=submit]):not([type=reset]):not([type=image]):not([type=color]):not([type=file]),textarea,select,[contenteditable]:not([contenteditable=false]),[role=textbox],[role=combobox]'));})()" completionHandler:^(id editing,NSError *error){
   if(!self.web.window || NSApp.keyWindow!=self.web.window)return;
   if(error || !self.enabled || [editing boolValue]){[self forward:event];return;}
   if(!event.isARepeat)self.dispatch(self.context,action);
  }];
  return nil;
 }];
 return (__bridge_retained void *)owner;
}
void musicos_shortcuts_detach(void *raw){
 NSCAssert([NSThread isMainThread],@"Shortcut routing belongs on the main thread");
 MusicOSPlayerShortcuts *owner=(__bridge_transfer MusicOSPlayerShortcuts *)raw;owner.enabled=NO;
 if(owner.monitor){[NSEvent removeMonitor:owner.monitor];owner.monitor=nil;}
 [owner.web.configuration.userContentController removeScriptMessageHandlerForName:@"musicosPlayerShortcutFocus"];
}
