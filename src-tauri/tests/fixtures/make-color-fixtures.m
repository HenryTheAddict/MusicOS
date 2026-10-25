// Reproduce: clang -fobjc-arc -framework Foundation -framework CoreGraphics
// -framework ImageIO make-color-fixtures.m -o /tmp/musicos-color-fixtures
// Run from this directory. Small synthetic swatches; no user artwork.
#import <Foundation/Foundation.h>
#import <CoreGraphics/CoreGraphics.h>
#import <ImageIO/ImageIO.h>
static void write_fixture(CGImageRef image, NSString *name) {
    CGImageDestinationRef out=CGImageDestinationCreateWithURL((__bridge CFURLRef)[NSURL fileURLWithPath:name],CFSTR("public.png"),1,NULL);
    CGImageDestinationAddImage(out,image,NULL);
    NSCAssert(CGImageDestinationFinalize(out),@"Fixture encoding failed");
    CFRelease(out);
}
int main(){@autoreleasepool {
    unsigned char p3[32*32*4];
    for(size_t i=0;i<32*32;i++){p3[4*i]=178;p3[4*i+1]=51;p3[4*i+2]=26;p3[4*i+3]=255;}
    CGColorSpaceRef color=CGColorSpaceCreateWithName(kCGColorSpaceDisplayP3);
    CGDataProviderRef provider=CGDataProviderCreateWithData(NULL,p3,sizeof(p3),NULL);
    CGImageRef image=CGImageCreate(32,32,8,32,32*4,color,(CGBitmapInfo)kCGImageAlphaLast,provider,NULL,NO,kCGRenderingIntentDefault);
    write_fixture(image,@"display-p3.png");CGImageRelease(image);CGDataProviderRelease(provider);CGColorSpaceRelease(color);
    unsigned short pq[96*32*4];
    for(size_t y=0;y<32;y++)for(size_t x=0;x<96;x++){
        size_t i=(y*96+x)*4;unsigned short v=x<32?26214:x<64?39321:52428;
        pq[i]=pq[i+1]=pq[i+2]=v;pq[i+3]=65535;
    }
    color=CGColorSpaceCreateWithName(kCGColorSpaceITUR_2100_PQ);
    provider=CGDataProviderCreateWithData(NULL,pq,sizeof(pq),NULL);
    image=CGImageCreate(96,32,16,64,96*8,color,(CGBitmapInfo)(kCGImageAlphaLast|kCGBitmapByteOrder16Little),provider,NULL,NO,kCGRenderingIntentDefault);
    write_fixture(image,@"hdr-pq.png");CGImageRelease(image);CGDataProviderRelease(provider);CGColorSpaceRelease(color);
}}
