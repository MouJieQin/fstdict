#include "http_client.h"
#import <Foundation/Foundation.h>

void httpGetAsync(const std::string &urlStr, int timeoutSec,
                  HttpCallback callback) {
  NSString *nsUrl = [NSString stringWithUTF8String:urlStr.c_str()];
  NSURL *url = [NSURL URLWithString:nsUrl];

  if (!url) {
    callback(false, "Invalid URL format");
    return;
  }

  NSMutableURLRequest *request = [NSMutableURLRequest requestWithURL:url];
  request.timeoutInterval = timeoutSec;
  [request setHTTPMethod:@"GET"];

  // Capture C++ callback into heap-allocated shared pointer for ObjC block
  auto callbackPtr = std::make_shared<HttpCallback>(std::move(callback));

  NSURLSession *session = [NSURLSession sharedSession];
  NSURLSessionDataTask *task =
      [session dataTaskWithRequest:request
                 completionHandler:^(NSData *data, NSURLResponse *response,
                                     NSError *error) {
                   if (error) {
                     std::string msg(error.localizedDescription.UTF8String);
                     (*callbackPtr)(false, msg);
                     return;
                   }

                   if (!data) {
                     (*callbackPtr)(true, "");
                     return;
                   }

                   NSString *utf8Str =
                       [[NSString alloc] initWithData:data
                                             encoding:NSUTF8StringEncoding];
                   std::string body(utf8Str.UTF8String);
                   (*callbackPtr)(true, body);
                 }];

  [task resume];
}
