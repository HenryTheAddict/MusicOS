#import <Foundation/Foundation.h>
NS_ASSUME_NONNULL_BEGIN
typedef void (^MusicOSBackgroundCallback)(NSString *identifier, void (^completion)(void));
void MusicOSSetBackgroundCallback(MusicOSBackgroundCallback callback);
NS_ASSUME_NONNULL_END
