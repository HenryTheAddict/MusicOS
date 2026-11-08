#import "MusicOSBackgroundHooks.h"
#import <UIKit/UIKit.h>
#import <objc/runtime.h>

// Forward the public optional UIApplicationDelegate callback without replacing Tauri's delegate.
static MusicOSBackgroundCallback callback;
static NSMutableDictionary<NSString *, void (^)(void)> *pending;
static IMP original;
static void handleBackground(id self, SEL selector, UIApplication *application, NSString *identifier, void (^completion)(void)) {
    if ([identifier hasPrefix:@"com.musicos.companion.transfers."]) {
        if (callback) callback(identifier, completion);
        else { if (!pending) pending = [NSMutableDictionary new]; pending[identifier] = [completion copy]; }
    } else if (original) ((void (*)(id,SEL,UIApplication *,NSString *,void (^)(void)))original)(self,selector,application,identifier,completion);
    else completion();
}
void MusicOSSetBackgroundCallback(MusicOSBackgroundCallback value) {
    callback = [value copy];
    for (NSString *identifier in [pending allKeys]) callback(identifier,pending[identifier]);
    [pending removeAllObjects];
}
__attribute__((constructor)) static void registerBackgroundForwarder(void) {
    [[NSNotificationCenter defaultCenter] addObserverForName:UIApplicationDidFinishLaunchingNotification object:nil queue:[NSOperationQueue mainQueue] usingBlock:^(NSNotification *note) {
        id delegate = [UIApplication sharedApplication].delegate;
        if (!delegate) return;
        Class type = object_getClass(delegate);
        SEL selector = @selector(application:handleEventsForBackgroundURLSession:completionHandler:);
        Method existing = class_getInstanceMethod(type,selector);
        if (existing) original = method_getImplementation(existing);
        if (!class_addMethod(type,selector,(IMP)handleBackground,"v@:@@@?")) {
            Method own = class_getInstanceMethod(type,selector);
            if (own) method_setImplementation(own,(IMP)handleBackground);
        }
    }];
}
