#import "MusicOSBackgroundHooks.h"
#import <UIKit/UIKit.h>
#import <objc/runtime.h>

// Forward the public optional UIApplicationDelegate callback without replacing Tauri's delegate.
static MusicOSBackgroundCallback callback;
static NSMutableDictionary<NSString *, void (^)(void)> *pending;
static IMP original;
static IMP originalOpenURL;
static void (^syncLinkCallback)(NSString *);
static NSString *pendingSyncLink;

static BOOL handleOpenURL(id self, SEL selector, UIApplication *application, NSURL *url, NSDictionary *options) {
    NSString *host = url.host.lowercaseString;
    if ([url.scheme.lowercaseString isEqualToString:@"musicos"] && ([host isEqualToString:@"sync"] || [host isEqualToString:@"activity"])) {
        NSString *destination = [host isEqualToString:@"activity"] ? (url.lastPathComponent ?: @"activity") : @"sync";
        if (syncLinkCallback) syncLinkCallback(destination);
        else pendingSyncLink = [destination copy];
        return YES;
    }
    return originalOpenURL
        ? ((BOOL (*)(id,SEL,UIApplication *,NSURL *,NSDictionary *))originalOpenURL)(self,selector,application,url,options)
        : NO;
}

void MusicOSSetSyncLinkCallback(void (^value)(NSString *)) {
    syncLinkCallback = [value copy];
    if (pendingSyncLink) {
        NSString *destination = pendingSyncLink;
        pendingSyncLink = nil;
        syncLinkCallback(destination);
    }
}
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
        if (!class_addMethod(type,selector,(IMP)handleBackground,"v@:@@@?") && existing) {
            method_setImplementation(existing,(IMP)handleBackground);
        }
        SEL openURL = @selector(application:openURL:options:);
        Method existingOpenURL = class_getInstanceMethod(type,openURL);
        if (existingOpenURL) originalOpenURL = method_getImplementation(existingOpenURL);
        const char *types = existingOpenURL ? method_getTypeEncoding(existingOpenURL) : "B@:@@@";
        if (!class_addMethod(type,openURL,(IMP)handleOpenURL,types) && existingOpenURL) {
            method_setImplementation(existingOpenURL,(IMP)handleOpenURL);
        }
    }];
}
