#pragma once
#include <cstdint>
#include <string>
#include <vector>

/// WebSocket frame codec utilities (RFC 6455)
namespace WebSocketFrame {

/// Encode a text frame for sending
/// @param payload UTF-8 text payload
/// @return Raw frame bytes
std::vector<uint8_t> encodeTextFrame(const std::string &payload);

/// Encode a PONG control frame with matching payload
std::vector<uint8_t> encodePongFrame(const uint8_t *payload, size_t payloadLen);

/// Decode and unmask a client frame
/// @param data Raw received bytes
/// @param len Byte length
/// @param[out] outPayload Decoded payload (text/binary)
/// @param[out] opcode Frame opcode (0x1=text, 0x8=close, 0x9=ping, 0xA=pong)
/// @return True if frame is complete and valid
bool decodeFrame(const char *data, size_t len, std::string &outPayload,
                 uint8_t &outOpcode);

/// Base64 encoding for WebSocket handshake
std::string base64Encode(const uint8_t *data, size_t len);

/// Compute Sec-WebSocket-Accept value from client key
std::string computeAcceptKey(const std::string &clientKey);

/// Extract Sec-WebSocket-Key from HTTP upgrade request
std::string extractWsKey(const char *requestData);

/// Build HTTP 101 Switching Protocols response
std::string buildHandshakeResponse(const std::string &acceptKey);

} // namespace WebSocketFrame
