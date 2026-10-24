#import <Foundation/Foundation.h>
#import <AppKit/AppKit.h>
#include <dlfcn.h>
#import <objc/message.h>
typedef void *(*MouseMessage)(CGPoint *, CGPoint *, uint32_t, NSEventType, CGSize, uint32_t);
int main(int argc, const char **argv) {
  @autoreleasepool {
    if (argc < 5) { fprintf(stderr, "simtap SET UDID x-ratio y-ratio\n"); return 2; }
    NSString *set = @(argv[1]), *udid = @(argv[2]);
    CGPoint point = CGPointMake(atof(argv[3]), atof(argv[4]));
    dlopen("/Library/Developer/PrivateFrameworks/CoreSimulator.framework/CoreSimulator", RTLD_NOW);
    NSString *developer = [[NSProcessInfo processInfo] environment][@"DEVELOPER_DIR"] ?: @"/Applications/Xcode.app/Contents/Developer";
    NSString *kitPath = [[developer stringByDeletingLastPathComponent] stringByAppendingPathComponent:@"SharedFrameworks/SimulatorKit.framework/SimulatorKit"];
    void *kit = dlopen(kitPath.UTF8String, RTLD_NOW);
    if (!kit) { fprintf(stderr, "no SimulatorKit: %s\n", dlerror()); return 1; }
    MouseMessage mouse = (MouseMessage)dlsym(kit, "IndigoHIDMessageForMouseNSEvent");
    if (!mouse) { fprintf(stderr, "no mouse message\n"); return 1; }
    NSError *error = nil;
    id context = ((id (*)(id, SEL, id, NSError **))objc_msgSend)(NSClassFromString(@"SimServiceContext"), NSSelectorFromString(@"sharedServiceContextForDeveloperDir:error:"), developer, &error);
    id deviceSet = ((id (*)(id, SEL, id, NSError **))objc_msgSend)(context, NSSelectorFromString(@"deviceSetWithPath:error:"), set, &error);
    id device = nil;
    for (id candidate in [deviceSet valueForKey:@"devices"]) if ([[[candidate valueForKey:@"UDID"] UUIDString] isEqualToString:udid]) device = candidate;
    if (!device) { fprintf(stderr, "device not found %s\n", error.description.UTF8String); return 1; }
    id client = ((id (*)(id, SEL, id, NSError **))objc_msgSend)([NSClassFromString(@"SimulatorKit.SimDeviceLegacyHIDClient") alloc], NSSelectorFromString(@"initWithDevice:error:"), device, &error);
    if (!client) { fprintf(stderr, "no hid client %s\n", error.description.UTF8String); return 1; }
    NSEventType presses[2] = { NSEventTypeLeftMouseDown, NSEventTypeLeftMouseUp };
    for (int index = 0; index < 2; index++) {
      void *message = mouse(&point, NULL, 0x32, presses[index], CGSizeMake(1.0, 1.0), 0);
      if (!message) { fprintf(stderr, "no message\n"); return 1; }
      dispatch_semaphore_t done = dispatch_semaphore_create(0);
      __block NSError *sendError = nil;
      ((void (*)(id, SEL, void *, BOOL, id, void (^)(NSError *)))objc_msgSend)(client, NSSelectorFromString(@"sendWithMessage:freeWhenDone:completionQueue:completion:"), message, YES, dispatch_get_global_queue(0, 0), ^(NSError *e) { sendError = e; dispatch_semaphore_signal(done); });
      dispatch_semaphore_wait(done, dispatch_time(DISPATCH_TIME_NOW, 5 * NSEC_PER_SEC));
      if (sendError) { fprintf(stderr, "send failed %s\n", sendError.description.UTF8String); return 1; }
      usleep(100000);
    }
    printf("tapped %s %s\n", argv[3], argv[4]);
  }
  return 0;
}
