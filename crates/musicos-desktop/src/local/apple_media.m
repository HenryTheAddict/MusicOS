#import <Foundation/Foundation.h>
#import <AVFoundation/AVFoundation.h>
#import <CoreML/CoreML.h>
#import <ImageIO/ImageIO.h>
#import <AudioToolbox/AudioToolbox.h>
#include <math.h>
#import <WebKit/WebKit.h>
#import <objc/message.h>

static void message(char *out, size_t size, NSString *text) {
    if (size) snprintf(out, size, "%s", (text ?: @"Media processing failed").UTF8String);
}
@interface MOSAudio : NSObject
@property AVURLAsset *asset;
@property AVAssetReader *reader;
@property AVAssetReaderTrackOutput *output;
@property AVAssetTrack *track;
@property double rate;
@end
@implementation MOSAudio
- (BOOL)seek:(double)seconds error:(NSError **)error {
    [self.reader cancelReading];
    self.reader = [[AVAssetReader alloc] initWithAsset:self.asset error:error];
    if (!self.reader) return NO;
    NSDictionary *settings = @{AVFormatIDKey:@(kAudioFormatLinearPCM), AVLinearPCMBitDepthKey:@32,
        AVLinearPCMIsFloatKey:@YES, AVLinearPCMIsNonInterleaved:@NO,
        AVSampleRateKey:@(self.rate), AVNumberOfChannelsKey:@2};
    self.output = [[AVAssetReaderTrackOutput alloc] initWithTrack:self.track outputSettings:settings];
    self.output.alwaysCopiesSampleData = NO;
    if (![self.reader canAddOutput:self.output]) return NO;
    [self.reader addOutput:self.output];
    self.reader.timeRange = CMTimeRangeMake(CMTimeMakeWithSeconds(seconds, 1000000000), kCMTimePositiveInfinity);
    if (![self.reader startReading]) { if (error) *error=self.reader.error; return NO; }
    return YES;
}
@end
void *mos_audio_open(const char *path, double *rate, double *duration, char *error, size_t size) {
    @autoreleasepool {
        MOSAudio *audio = [MOSAudio new];
        audio.asset = [AVURLAsset URLAssetWithURL:[NSURL fileURLWithPath:@(path)] options:@{AVURLAssetPreferPreciseDurationAndTimingKey:@YES}];
        if (audio.asset.hasProtectedContent) { message(error,size,@"This file is protected. Import an unprotected audio file."); return NULL; }
        audio.track = [audio.asset tracksWithMediaType:AVMediaTypeAudio].firstObject;
        if (!audio.track) { message(error,size,@"No readable audio stream in this file."); return NULL; }
        CMAudioFormatDescriptionRef format=(__bridge CMAudioFormatDescriptionRef)audio.track.formatDescriptions.firstObject;
        const AudioStreamBasicDescription *asbd=format ? CMAudioFormatDescriptionGetStreamBasicDescription(format) : NULL;
        audio.rate=asbd && asbd->mSampleRate>0 ? asbd->mSampleRate : 48000;
        *rate=audio.rate; *duration=CMTimeGetSeconds(audio.asset.duration);
        NSError *failure=nil;
        if (![audio seek:0 error:&failure]) { message(error,size,failure.localizedDescription); return NULL; }
        return (__bridge_retained void *)audio;
    }
}
int mos_audio_read(void *handle, float *samples, size_t capacity, char *error, size_t size) {
    @autoreleasepool {
        MOSAudio *audio=(__bridge MOSAudio *)handle;
        CMSampleBufferRef buffer=[audio.output copyNextSampleBuffer];
        if (!buffer) {
            if (audio.reader.status==AVAssetReaderStatusFailed) { message(error,size,audio.reader.error.localizedDescription); return -1; }
            return 0;
        }
        CMBlockBufferRef block=CMSampleBufferGetDataBuffer(buffer);
        size_t length=block ? CMBlockBufferGetDataLength(block) : 0;
        if (length>capacity*sizeof(float)) { CFRelease(buffer); message(error,size,@"Decoded audio packet exceeds the bounded buffer."); return -1; }
        OSStatus result=block ? CMBlockBufferCopyDataBytes(block,0,length,samples) : -1;
        CFRelease(buffer);
        if (result) { message(error,size,@"Cannot read decoded audio samples."); return -1; }
        return (int)(length/sizeof(float));
    }
}
int mos_audio_seek(void *handle, double seconds, char *error, size_t size) {
    @autoreleasepool { NSError *failure=nil; BOOL ok=[(__bridge MOSAudio *)handle seek:seconds error:&failure];
        if (!ok) message(error,size,failure.localizedDescription); return ok ? 0 : -1; }
}
void mos_audio_close(void *handle) {
    @autoreleasepool { MOSAudio *audio=CFBridgingRelease(handle); [audio.reader cancelReading]; }
}

static CGImageRef oriented_image(const char *path, size_t maximum) {
    CGImageSourceRef source=CGImageSourceCreateWithURL((__bridge CFURLRef)[NSURL fileURLWithPath:@(path)],NULL);
    if (!source) return NULL;
    CGImageRef image=CGImageSourceCreateThumbnailAtIndex(source,0,(__bridge CFDictionaryRef)@{
        (id)kCGImageSourceCreateThumbnailFromImageAlways:@YES,(id)kCGImageSourceCreateThumbnailWithTransform:@YES,
        (id)kCGImageSourceThumbnailMaxPixelSize:@(maximum)});
    CFRelease(source); return image;
}
static BOOL write_png(CGImageRef image, const char *path) {
    CGImageDestinationRef destination=CGImageDestinationCreateWithURL((__bridge CFURLRef)[NSURL fileURLWithPath:@(path)],CFSTR("public.png"),1,NULL);
    if (!destination) return NO;
    CGImageDestinationAddImage(destination,image,NULL); BOOL ok=CGImageDestinationFinalize(destination);CFRelease(destination);return ok;
}
int mos_oriented_png(const char *source, const char *destination) {
    @autoreleasepool { CGImageRef image=oriented_image(source,8192);if(!image)return -1;
        BOOL ok=write_png(image,destination);CGImageRelease(image);return ok?0:-1; }
}
// Called on the serialized depth worker only. Core ML never runs on an audio or UI thread.
int mos_depth(const char *package, const char *source, const char *destination, char *error, size_t size) {
    @autoreleasepool {
        static MLModel *model=nil; static NSString *loaded=nil;
        NSError *failure=nil; NSString *path=@(package);
        if (!model || ![loaded isEqualToString:path]) {
            NSURL *compiled=[MLModel compileModelAtURL:[NSURL fileURLWithPath:path] error:&failure];
            MLModelConfiguration *config=[MLModelConfiguration new];config.computeUnits=MLComputeUnitsCPUAndNeuralEngine;
            model=compiled ? [MLModel modelWithContentsOfURL:compiled configuration:config error:&failure] : nil;
            loaded=path;
        }
        if (!model) { message(error,size,failure.localizedDescription);return -1; }
        NSString *name=nil; MLImageConstraint *constraint=nil;
        for (NSString *key in model.modelDescription.inputDescriptionsByName) {
            MLFeatureDescription *description=model.modelDescription.inputDescriptionsByName[key];
            if(description.type==MLFeatureTypeImage){name=key;constraint=description.imageConstraint;break;}
        }
        if (!constraint) { message(error,size,@"Depth model has no image input.");return -1; }
        CVPixelBufferRef pixel=NULL;
        CVReturn result=CVPixelBufferCreate(NULL,constraint.pixelsWide,constraint.pixelsHigh,kCVPixelFormatType_32BGRA,
            (__bridge CFDictionaryRef)@{(id)kCVPixelBufferCGImageCompatibilityKey:@YES,(id)kCVPixelBufferCGBitmapContextCompatibilityKey:@YES},&pixel);
        CGImageRef image=oriented_image(source,2048);
        if(result||!image){if(pixel)CVPixelBufferRelease(pixel);message(error,size,@"Cannot decode artwork for depth.");return -1;}
        CVPixelBufferLockBaseAddress(pixel,0);CGColorSpaceRef color=CGColorSpaceCreateDeviceRGB();
        CGContextRef context=CGBitmapContextCreate(CVPixelBufferGetBaseAddress(pixel),constraint.pixelsWide,constraint.pixelsHigh,8,
            CVPixelBufferGetBytesPerRow(pixel),color,kCGImageAlphaPremultipliedFirst|kCGBitmapByteOrder32Little);
        CGColorSpaceRelease(color);
        if(!context){CGImageRelease(image);CVPixelBufferUnlockBaseAddress(pixel,0);CVPixelBufferRelease(pixel);return -1;}
        CGContextDrawImage(context,CGRectMake(0,0,constraint.pixelsWide,constraint.pixelsHigh),image);
        CGContextRelease(context);CGImageRelease(image);CVPixelBufferUnlockBaseAddress(pixel,0);
        MLDictionaryFeatureProvider *input=[[MLDictionaryFeatureProvider alloc] initWithDictionary:@{name:[MLFeatureValue featureValueWithPixelBuffer:pixel]} error:&failure];
        id<MLFeatureProvider> prediction=input ? [model predictionFromFeatures:input error:&failure] : nil;
        CVPixelBufferRelease(pixel);MLMultiArray *depth=nil;CVPixelBufferRef depthPixel=NULL;
        for(NSString *key in prediction.featureNames){MLFeatureValue *value=[prediction featureValueForName:key];
            if(value.type==MLFeatureTypeMultiArray){depth=value.multiArrayValue;break;}
            if(value.type==MLFeatureTypeImage){depthPixel=value.imageBufferValue;break;}}
        size_t h=0,w=0;
        if(depth&&depth.shape.count>=2){h=depth.shape[depth.shape.count-2].unsignedIntegerValue;w=depth.shape.lastObject.unsignedIntegerValue;}
        if(depthPixel){h=CVPixelBufferGetHeight(depthPixel);w=CVPixelBufferGetWidth(depthPixel);CVPixelBufferLockBaseAddress(depthPixel,kCVPixelBufferLock_ReadOnly);}
        if(!w||!h||w*h>1048576){if(depthPixel)CVPixelBufferUnlockBaseAddress(depthPixel,kCVPixelBufferLock_ReadOnly);message(error,size,failure.localizedDescription?:@"Depth output is invalid.");return -1;}
        OSType pixelType=depthPixel?CVPixelBufferGetPixelFormatType(depthPixel):0;
        if(depthPixel&&pixelType!=kCVPixelFormatType_OneComponent32Float&&pixelType!=kCVPixelFormatType_OneComponent16Half&&pixelType!=kCVPixelFormatType_OneComponent8){CVPixelBufferUnlockBaseAddress(depthPixel,kCVPixelBufferLock_ReadOnly);message(error,size,@"Depth pixel format is unsupported.");return -1;}
        double (^sample)(size_t)=^double(size_t i){if(depth)return depth[i].doubleValue;
            uint8_t *row=(uint8_t *)CVPixelBufferGetBaseAddress(depthPixel)+(i/w)*CVPixelBufferGetBytesPerRow(depthPixel);size_t x=i%w;
            if(pixelType==kCVPixelFormatType_OneComponent32Float)return ((float *)row)[x];
            if(pixelType==kCVPixelFormatType_OneComponent16Half)return ((__fp16 *)row)[x];return row[x];};
        double lo=INFINITY,hi=-INFINITY;
        for(size_t i=0;i<w*h;i++){double v=sample(i);if(!isfinite(v)){if(depthPixel)CVPixelBufferUnlockBaseAddress(depthPixel,kCVPixelBufferLock_ReadOnly);message(error,size,@"Depth output is not finite.");return -1;}lo=fmin(lo,v);hi=fmax(hi,v);}
        if(hi-lo<0.00001){if(depthPixel)CVPixelBufferUnlockBaseAddress(depthPixel,kCVPixelBufferLock_ReadOnly);message(error,size,@"This artwork has no useful depth.");return -1;}
        uint8_t *data=malloc(w*h);if(!data){if(depthPixel)CVPixelBufferUnlockBaseAddress(depthPixel,kCVPixelBufferLock_ReadOnly);return -1;}
        for(size_t i=0;i<w*h;i++)data[i]=(uint8_t)lrint((sample(i)-lo)/(hi-lo)*255.);
        if(depthPixel)CVPixelBufferUnlockBaseAddress(depthPixel,kCVPixelBufferLock_ReadOnly);
        color=CGColorSpaceCreateDeviceGray();context=CGBitmapContextCreate(data,w,h,8,w,color,(CGBitmapInfo)kCGImageAlphaNone);CGColorSpaceRelease(color);
        image=context?CGBitmapContextCreateImage(context):NULL;BOOL ok=image&&write_png(image,destination);
        if(image)CGImageRelease(image);if(context)CGContextRelease(context);free(data);
        if(!ok)message(error,size,@"Cannot save the depth cache.");return ok?0:-1;
    }
}
int mos_audio_probe(const char *path, double *rate, double *duration, uint32_t *codec, uint32_t *channels, uint32_t *bits, double *bitrate) {
 @autoreleasepool {AVURLAsset *asset=[AVURLAsset URLAssetWithURL:[NSURL fileURLWithPath:@(path)] options:nil];
 AVAssetTrack *track=[asset tracksWithMediaType:AVMediaTypeAudio].firstObject;if(!track||asset.hasProtectedContent)return -1;
 CMAudioFormatDescriptionRef format=(__bridge CMAudioFormatDescriptionRef)track.formatDescriptions.firstObject;
 const AudioStreamBasicDescription *asbd=format?CMAudioFormatDescriptionGetStreamBasicDescription(format):NULL;if(!asbd)return -1;
 *rate=asbd->mSampleRate;*duration=CMTimeGetSeconds(asset.duration);*codec=asbd->mFormatID;*channels=asbd->mChannelsPerFrame;*bits=asbd->mBitsPerChannel;*bitrate=track.estimatedDataRate;return 0;}
}

// WebKit exposes no public embedded frame-rate preference. Guard the exact
// internal feature at runtime; missing/changed APIs retain the platform default.
int mos_webview_native_refresh(void *pointer) {
 @autoreleasepool { @try {
  if(![NSThread isMainThread] || !pointer) return 0;
  WKWebView *view=(__bridge WKWebView *)pointer;
  id preferences=view.configuration.preferences;
  Class type=[preferences class]; SEL features=NSSelectorFromString(@"_features"), setter=NSSelectorFromString(@"_setEnabled:forFeature:");
  if(![type respondsToSelector:features]||![preferences respondsToSelector:setter])return 0;
  NSArray *list=((id(*)(id,SEL))objc_msgSend)(type,features);
  for(id feature in list){SEL key=NSSelectorFromString(@"key");if(![feature respondsToSelector:key])continue;
   NSString *name=((id(*)(id,SEL))objc_msgSend)(feature,key);
   if([name isEqualToString:@"PreferPageRenderingUpdatesNear60FPSEnabled"]){
    ((void(*)(id,SEL,BOOL,id))objc_msgSend)(preferences,setter,NO,feature);return 1;
   }
  }
 } @catch(NSException *exception) { return 0; } return 0; }
}
