#import <Foundation/Foundation.h>
#import <AppKit/AppKit.h>
#include <dlfcn.h>
#import <objc/message.h>
typedef void *(*MouseMessage)(CGPoint *, CGPoint *, uint32_t, NSEventType, CGSize, uint32_t);
typedef void *(*ButtonMessage)(int, int, int);
static int send(id client, void *message) {
  if (!message) { fprintf(stderr, "no message\n"); return 1; }
  dispatch_semaphore_t done = dispatch_semaphore_create(0);
  __block NSError *sendError = nil;
  ((void (*)(id, SEL, void *, BOOL, id, void (^)(NSError *)))objc_msgSend)(client, NSSelectorFromString(@"sendWithMessage:freeWhenDone:completionQueue:completion:"), message, YES, dispatch_get_global_queue(0, 0), ^(NSError *e) { sendError = e; dispatch_semaphore_signal(done); });
  dispatch_semaphore_wait(done, dispatch_time(DISPATCH_TIME_NOW, 5 * NSEC_PER_SEC));
  if (sendError) { fprintf(stderr, "send failed %s\n", sendError.description.UTF8String); return 1; }
  return 0;
}
int main(int argc, const char **argv) {
  @autoreleasepool {
    if (argc < 4) { fprintf(stderr, "simtap SET UDID x-ratio y-ratio | SET UDID drag x1 y1 x2 y2 [steps] [edge] [hold-ms] | SET UDID home [presses]\n"); return 2; }
    NSString *set = @(argv[1]), *udid = @(argv[2]);
    dlopen("/Library/Developer/PrivateFrameworks/CoreSimulator.framework/CoreSimulator", RTLD_NOW);
    NSString *developer = [[NSProcessInfo processInfo] environment][@"DEVELOPER_DIR"] ?: @"/Applications/Xcode.app/Contents/Developer";
    NSString *kitPath = [[developer stringByDeletingLastPathComponent] stringByAppendingPathComponent:@"SharedFrameworks/SimulatorKit.framework/SimulatorKit"];
    void *kit = dlopen(kitPath.UTF8String, RTLD_NOW);
    if (!kit) { fprintf(stderr, "no SimulatorKit: %s\n", dlerror()); return 1; }
    MouseMessage mouse = (MouseMessage)dlsym(kit, "IndigoHIDMessageForMouseNSEvent");
    ButtonMessage button = (ButtonMessage)dlsym(kit, "IndigoHIDMessageForButton");
    if (!mouse) { fprintf(stderr, "no mouse message\n"); return 1; }
    NSError *error = nil;
    id context = ((id (*)(id, SEL, id, NSError **))objc_msgSend)(NSClassFromString(@"SimServiceContext"), NSSelectorFromString(@"sharedServiceContextForDeveloperDir:error:"), developer, &error);
    id deviceSet = ((id (*)(id, SEL, id, NSError **))objc_msgSend)(context, NSSelectorFromString(@"deviceSetWithPath:error:"), set, &error);
    id device = nil;
    for (id candidate in [deviceSet valueForKey:@"devices"]) if ([[[candidate valueForKey:@"UDID"] UUIDString] isEqualToString:udid]) device = candidate;
    if (!device) { fprintf(stderr, "device not found %s\n", error.description.UTF8String); return 1; }
    id client = ((id (*)(id, SEL, id, NSError **))objc_msgSend)([NSClassFromString(@"SimulatorKit.SimDeviceLegacyHIDClient") alloc], NSSelectorFromString(@"initWithDevice:error:"), device, &error);
    if (!client) { fprintf(stderr, "no hid client %s\n", error.description.UTF8String); return 1; }
    if (strcmp(argv[3], "home") == 0) {
      if (!button) { fprintf(stderr, "no button message\n"); return 1; }
      int presses = argc > 4 ? atoi(argv[4]) : 1;
      for (int press = 0; press < presses; press++) {
        if (send(client, button(0x0, 1, 0x33)) || send(client, button(0x0, 2, 0x33))) return 1;
        usleep(150000);
      }
      printf("pressed home %d\n", presses);
      return 0;
    }
    if (strcmp(argv[3], "drag") == 0) {
      if (argc < 8) { fprintf(stderr, "simtap SET UDID drag x1 y1 x2 y2 [steps] [edge] [hold-ms]\n"); return 2; }
      CGPoint start = CGPointMake(atof(argv[4]), atof(argv[5])), end = CGPointMake(atof(argv[6]), atof(argv[7]));
      int steps = argc > 8 ? atoi(argv[8]) : 12;
      uint32_t edge = argc > 9 ? (uint32_t)strtoul(argv[9], NULL, 0) : 0;
      int hold = argc > 10 ? atoi(argv[10]) : 0;
      for (int step = 0; step <= steps; step++) {
        CGPoint point = CGPointMake(start.x + (end.x - start.x) * step / steps, start.y + (end.y - start.y) * step / steps);
        if (send(client, mouse(&point, NULL, 0x32, NSEventTypeLeftMouseDown, CGSizeMake(1.0, 1.0), edge))) return 1;
        usleep(16000);
      }
      if (hold > 0) usleep(hold * 1000);
      if (send(client, mouse(&end, NULL, 0x32, NSEventTypeLeftMouseUp, CGSizeMake(1.0, 1.0), edge))) return 1;
      printf("dragged %s %s to %s %s\n", argv[4], argv[5], argv[6], argv[7]);
      return 0;
    }
    if (argc < 5) { fprintf(stderr, "simtap SET UDID x-ratio y-ratio\n"); return 2; }
    CGPoint point = CGPointMake(atof(argv[3]), atof(argv[4]));
    NSEventType presses[2] = { NSEventTypeLeftMouseDown, NSEventTypeLeftMouseUp };
    for (int index = 0; index < 2; index++) {
      if (send(client, mouse(&point, NULL, 0x32, presses[index], CGSizeMake(1.0, 1.0), 0))) return 1;
      usleep(100000);
    }
    printf("tapped %s %s\n", argv[3], argv[4]);
  }
  return 0;
}
