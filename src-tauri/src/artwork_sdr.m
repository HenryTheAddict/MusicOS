#import <Foundation/Foundation.h>
#import <CoreGraphics/CoreGraphics.h>
#import <ImageIO/ImageIO.h>

// Desktop display copies only. Never rewrite the user's original or model input.
int musicos_display_sdr_png(const char *sourcePath, const char *destinationPath, unsigned int maximum) {
    @autoreleasepool {
        CGImageSourceRef source = CGImageSourceCreateWithURL((__bridge CFURLRef)[NSURL fileURLWithPath:@(sourcePath)], NULL);
        if (!source) return -1;
        NSMutableDictionary *options = [@{
            (id)kCGImageSourceCreateThumbnailFromImageAlways:@YES,
            (id)kCGImageSourceCreateThumbnailWithTransform:@YES,
            (id)kCGImageSourceThumbnailMaxPixelSize:@(maximum),
            (id)kCGImageSourceShouldAllowFloat:@NO
        } mutableCopy];
        if (@available(macOS 14.0, *)) options[(id)kCGImageSourceDecodeRequest] = (id)kCGImageSourceDecodeToSDR;
        CGImageRef decoded = CGImageSourceCreateThumbnailAtIndex(source, 0, (__bridge CFDictionaryRef)options);
        CFRelease(source);
        if (!decoded) return -1;
        size_t width = CGImageGetWidth(decoded), height = CGImageGetHeight(decoded);
        CGColorSpaceRef color = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
        CGContextRef context = CGBitmapContextCreate(NULL, width, height, 8, 0, color, (CGBitmapInfo)kCGImageAlphaPremultipliedLast);
        CGColorSpaceRelease(color);
        if (!context) { CGImageRelease(decoded); return -1; }
        if (@available(macOS 15.0, *)) CGContextSetEDRTargetHeadroom(context, 1.0f);
        CGContextSetBlendMode(context, kCGBlendModeCopy);
        CGContextDrawImage(context, CGRectMake(0, 0, width, height), decoded);
        CGImageRelease(decoded);
        CGImageRef display = CGBitmapContextCreateImage(context);
        CGContextRelease(context);
        if (!display) return -1;
        CGImageDestinationRef destination = CGImageDestinationCreateWithURL((__bridge CFURLRef)[NSURL fileURLWithPath:@(destinationPath)], CFSTR("public.png"), 1, NULL);
        BOOL ok = NO;
        if (destination) {
            CGImageDestinationAddImage(destination, display, NULL);
            ok = CGImageDestinationFinalize(destination);
            CFRelease(destination);
        }
        CGImageRelease(display);
        return ok ? 0 : -1;
    }
}
