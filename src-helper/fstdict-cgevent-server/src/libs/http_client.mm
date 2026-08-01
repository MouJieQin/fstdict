// http_client.mm
#include "http_client.h"
#import <Foundation/Foundation.h>

void http_get_async(const std::string &url_str, int timeout_sec,
                    HttpCallback cb) {
  NSString *nsUrlStr = [NSString stringWithUTF8String:url_str.c_str()];
  NSURL *url = [NSURL URLWithString:nsUrlStr];
  if (!url) {
    cb(false, "invalid url");
    return;
  }

  NSMutableURLRequest *req = [NSMutableURLRequest requestWithURL:url];
  req.timeoutInterval = timeout_sec;
  [req setHTTPMethod:@"GET"];

  // 捕获C++回调到block（拷贝lambda）
  auto callback = std::make_shared<HttpCallback>(std::move(cb));

  NSURLSession *session = [NSURLSession sharedSession];
  NSURLSessionDataTask *task = [session
      dataTaskWithRequest:req
        completionHandler:^(NSData *data, NSURLResponse *resp, NSError *err) {
          if (err) {
            std::string msg(err.localizedDescription.UTF8String);
            (*callback)(false, msg);
            return;
          }

          if (!data) {
            (*callback)(true, "");
            return;
          }

          NSString *utf8Str =
              [[NSString alloc] initWithData:data
                                    encoding:NSUTF8StringEncoding];
          std::string body(utf8Str.UTF8String);
          (*callback)(true, body);
        }];
  [task resume];
}