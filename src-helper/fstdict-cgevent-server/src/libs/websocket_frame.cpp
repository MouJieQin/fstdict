#include "websocket_frame.h"
#include <CommonCrypto/CommonDigest.h>
#include <cstring>
#include <sstream>

namespace WebSocketFrame {

std::vector<uint8_t> encodeTextFrame(const std::string &payload) {
  std::vector<uint8_t> frame;
  size_t payloadLen = payload.size();

  // Reserve space for header + payload
  size_t headerSize = 2;
  if (payloadLen >= 126 && payloadLen < 65536)
    headerSize += 2;
  else if (payloadLen >= 65536)
    headerSize += 8;
  frame.reserve(headerSize + payloadLen);

  // FIN + text opcode
  frame.push_back(0x81);

  // Payload length
  if (payloadLen < 126) {
    frame.push_back(static_cast<uint8_t>(payloadLen));
  } else if (payloadLen < 65536) {
    frame.push_back(126);
    frame.push_back(static_cast<uint8_t>((payloadLen >> 8) & 0xFF));
    frame.push_back(static_cast<uint8_t>(payloadLen & 0xFF));
  } else {
    frame.push_back(127);
    for (int i = 7; i >= 0; --i) {
      frame.push_back(static_cast<uint8_t>((payloadLen >> (i * 8)) & 0xFF));
    }
  }

  // Append payload (server-to-client frames are unmasked)
  frame.insert(frame.end(), payload.begin(), payload.end());
  return frame;
}

std::vector<uint8_t> encodePongFrame(const uint8_t *payload,
                                     size_t payloadLen) {
  std::vector<uint8_t> frame;
  frame.reserve(2 + payloadLen);

  frame.push_back(0x8A); // FIN + PONG opcode

  if (payloadLen < 126) {
    frame.push_back(static_cast<uint8_t>(payloadLen));
  } else if (payloadLen < 65536) {
    frame.push_back(126);
    frame.push_back(static_cast<uint8_t>((payloadLen >> 8) & 0xFF));
    frame.push_back(static_cast<uint8_t>(payloadLen & 0xFF));
  }

  if (payload && payloadLen > 0) {
    frame.insert(frame.end(), payload, payload + payloadLen);
  }
  return frame;
}

bool decodeFrame(const char *data, size_t len, std::string &outPayload,
                 uint8_t &outOpcode) {
  if (len < 2) return false;

  uint8_t fin = (static_cast<uint8_t>(data[0]) >> 7) & 1;
  outOpcode = static_cast<uint8_t>(data[0]) & 0x0F;
  uint8_t masked = (static_cast<uint8_t>(data[1]) >> 7) & 1;
  uint64_t payloadLen = static_cast<uint8_t>(data[1]) & 0x7F;

  size_t offset = 2;

  // Extended payload length
  if (payloadLen == 126) {
    if (len < 4) return false;
    payloadLen =
        (static_cast<uint8_t>(data[2]) << 8) | static_cast<uint8_t>(data[3]);
    offset += 2;
  } else if (payloadLen == 127) {
    if (len < 10) return false;
    payloadLen = 0;
    for (int i = 0; i < 8; ++i) {
      payloadLen = (payloadLen << 8) | static_cast<uint8_t>(data[2 + i]);
    }
    offset += 8;
  }

  // Client frames MUST be masked per RFC 6455
  if (!masked) return false;
  if (len < offset + 4 + payloadLen) return false;

  // Unmask payload
  const uint8_t *maskKey = reinterpret_cast<const uint8_t *>(data) + offset;
  offset += 4;

  outPayload.clear();
  outPayload.reserve(payloadLen);
  for (size_t i = 0; i < payloadLen; ++i) {
    outPayload += static_cast<char>(static_cast<uint8_t>(data[offset + i]) ^
                                    maskKey[i % 4]);
  }

  return true;
}

static const char BASE64_TABLE[] =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

std::string base64Encode(const uint8_t *data, size_t len) {
  std::string out;
  out.reserve(((len + 2) / 3) * 4);

  for (size_t i = 0; i < len; i += 3) {
    uint32_t block = static_cast<uint32_t>(data[i]) << 16;
    if (i + 1 < len) block |= static_cast<uint32_t>(data[i + 1]) << 8;
    if (i + 2 < len) block |= static_cast<uint32_t>(data[i + 2]);

    out += BASE64_TABLE[(block >> 18) & 0x3F];
    out += BASE64_TABLE[(block >> 12) & 0x3F];
    out += (i + 1 < len) ? BASE64_TABLE[(block >> 6) & 0x3F] : '=';
    out += (i + 2 < len) ? BASE64_TABLE[block & 0x3F] : '=';
  }
  return out;
}

std::string computeAcceptKey(const std::string &clientKey) {
  const std::string WS_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
  std::string combined = clientKey + WS_GUID;

  uint8_t hash[CC_SHA1_DIGEST_LENGTH];
  CC_SHA1(reinterpret_cast<const uint8_t *>(combined.c_str()),
          static_cast<CC_LONG>(combined.size()), hash);

  return base64Encode(hash, CC_SHA1_DIGEST_LENGTH);
}

std::string extractWsKey(const char *requestData) {
  std::string request(requestData);
  size_t pos = request.find("Sec-WebSocket-Key: ");
  if (pos == std::string::npos) return "";

  pos += 19; // Length of header name
  size_t end = request.find("\r\n", pos);
  return request.substr(pos, end - pos);
}

std::string buildHandshakeResponse(const std::string &acceptKey) {
  std::ostringstream resp;
  resp << "HTTP/1.1 101 Switching Protocols\r\n";
  resp << "Upgrade: websocket\r\n";
  resp << "Connection: Upgrade\r\n";
  resp << "Sec-WebSocket-Accept: " << acceptKey << "\r\n";
  resp << "\r\n";
  return resp.str();
}

} // namespace WebSocketFrame
