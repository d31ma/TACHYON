#import <Foundation/Foundation.h>
#include <dlfcn.h>
#import <objc/message.h>
typedef void *(*KeyboardMessage)(uint32_t, uint32_t);
int main(int argc, const char **argv) {
  @autoreleasepool {
    if (argc < 4) { fprintf(stderr, "simkeys SET UDID usage[,usage...]\n"); return 2; }
    NSString *set = @(argv[1]), *udid = @(argv[2]);
    dlopen("/Library/Developer/PrivateFrameworks/CoreSimulator.framework/CoreSimulator", RTLD_NOW);
    NSString *developer = [[NSProcessInfo processInfo] environment][@"DEVELOPER_DIR"] ?: @"/Applications/Xcode.app/Contents/Developer";
    NSString *kitPath = [[developer stringByDeletingLastPathComponent] stringByAppendingPathComponent:@"SharedFrameworks/SimulatorKit.framework/SimulatorKit"];
    void *kit = dlopen(kitPath.UTF8String, RTLD_NOW);
    if (!kit) { fprintf(stderr, "no SimulatorKit: %s\n", dlerror()); return 1; }
    KeyboardMessage keyboard = (KeyboardMessage)dlsym(kit, "IndigoHIDMessageForKeyboardArbitrary");
    if (!keyboard) { fprintf(stderr, "no keyboard message\n"); return 1; }
    NSError *error = nil;
    id context = ((id (*)(id, SEL, id, NSError **))objc_msgSend)(NSClassFromString(@"SimServiceContext"), NSSelectorFromString(@"sharedServiceContextForDeveloperDir:error:"), developer, &error);
    id deviceSet = ((id (*)(id, SEL, id, NSError **))objc_msgSend)(context, NSSelectorFromString(@"deviceSetWithPath:error:"), set, &error);
    id device = nil;
    for (id candidate in [deviceSet valueForKey:@"devices"]) if ([[[candidate valueForKey:@"UDID"] UUIDString] isEqualToString:udid]) device = candidate;
    if (!device) { fprintf(stderr, "device not found %s\n", error.description.UTF8String); return 1; }
    id client = ((id (*)(id, SEL, id, NSError **))objc_msgSend)([NSClassFromString(@"SimulatorKit.SimDeviceLegacyHIDClient") alloc], NSSelectorFromString(@"initWithDevice:error:"), device, &error);
    if (!client) { fprintf(stderr, "no hid client %s\n", error.description.UTF8String); return 1; }
    NSArray<NSString *> *usages = [@(argv[3]) componentsSeparatedByString:@","];
    NSMutableArray *events = [NSMutableArray array];
    for (NSString *usage in usages) [events addObject:@[@(strtoul(usage.UTF8String, NULL, 0)), @1]];
    for (NSString *usage in usages.reverseObjectEnumerator) [events addObject:@[@(strtoul(usage.UTF8String, NULL, 0)), @2]];
    for (NSArray *event in events) {
      void *message = keyboard([event[0] unsignedIntValue], [event[1] unsignedIntValue]);
      dispatch_semaphore_t done = dispatch_semaphore_create(0);
      __block NSError *sendError = nil;
      ((void (*)(id, SEL, void *, BOOL, id, void (^)(NSError *)))objc_msgSend)(client, NSSelectorFromString(@"sendWithMessage:freeWhenDone:completionQueue:completion:"), message, YES, dispatch_get_global_queue(0, 0), ^(NSError *e) { sendError = e; dispatch_semaphore_signal(done); });
      dispatch_semaphore_wait(done, dispatch_time(DISPATCH_TIME_NOW, 5 * NSEC_PER_SEC));
      if (sendError) { fprintf(stderr, "send failed %s\n", sendError.description.UTF8String); return 1; }
      usleep(60000);
    }
    printf("pressed %s\n", argv[3]);
  }
  return 0;
}
