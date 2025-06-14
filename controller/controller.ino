#include <vector>
#include <optional>
#include <memory>
#include "flaps.h"
#include <WiFiS3.h>
#include "arduino_secrets.h"

std::optional<std::string_view> readUntil(std::string_view* s, char delim) {
  size_t split = s->find(delim);
  if (split == std::string::npos) {
    return std::nullopt;
  } else {
    std::string_view ret = s->substr(0, split);
    *s = s->substr(split + 1);
    return ret;
  }
}

enum class HttpMethod {
  GET,
  POST
};

enum class HttpProto {
  HTTP1_0,
  HTTP1_1,
};

std::optional<HttpMethod> readMethod(std::string_view* s) {
  auto methodStr = readUntil(s, ' ');
  if (!methodStr) {
    Serial.println("No proto");
    return std::nullopt;
  }
  if (methodStr == "GET") {
    return HttpMethod::GET;
  } else if (methodStr == "POST") {
    return HttpMethod::POST;
  } else {
    Serial.print("Unknown method ");
    Serial.write(methodStr->data(), methodStr->length());
    Serial.println();
    return std::nullopt;
  }
}

class HttpServlet {
public:
  HttpServlet(const HttpServlet&) = delete;
  HttpServlet& operator=(const HttpServlet&) = delete;
  HttpServlet() {
    _buffer.reserve(1024);
    _uri_buffer.reserve(128);
  }

  void init(WiFiClient client) {
    _client = std::move(client);
    _buffer = "";
  }
  void uninit() {
    _client = WiFiClient();
    _buffer = "";
  }

  std::string_view readHeader() {
    _buffer = "";
    while (_client.connected()) {
      int c = _client.read();
      if (c == -1) { continue; }
      if (_buffer.size() == _buffer.capacity()) {
        Serial.println("Buffer overflow");
        return "";
      }
      if (c == '\n') {
        Serial.print("Request: ");
        Serial.print(_buffer.c_str());
        Serial.println();
        return std::string_view(_buffer.c_str(), _buffer.length());
      }
      if (c != '\r') {
        _buffer += c;
      }
    }
  }

  std::optional<std::string_view> run() {
    std::string_view first = readHeader();
    auto method = readMethod(&first);
    if (!method) {
      return std::nullopt;
    }
    auto uri = readUntil(&first, ' ');
    if (!uri) {
      Serial.println("No uri");
      return std::nullopt;
    }
    if (uri->length() >= _uri_buffer.capacity()) {
      Serial.println("Uri overflow");
      return std::nullopt;
    }
    HttpProto proto;
    if (first == "HTTP/1.0") {
      proto = HttpProto::HTTP1_0;
    } else if (first == "HTTP/1.1") {
      proto = HttpProto::HTTP1_1;
    } else {
      Serial.println("Unknown HTTP proto version");
    }
    _uri_buffer = *uri;
    while (true) {
      std::string_view line = readHeader();
      if (line == "") {
        break;
      }
    }
    _client.println("HTTP/1.1 200 Ok");
    _client.println("Content-Type: text/html");
    _client.println();
    _client.println("<!DOCTYPE html>");
    _client.println("<html lang=\"en\">");
    _client.println("  <head>");
    _client.println("    <meta charset=\"utf-8\">");
    _client.println("    <title>Flappy McFlappyFace</title>");
    _client.println("  </head>");
    _client.println("  <body>");
    _client.println("    Welcome!");
    _client.println("    <form>");
    _client.println("      <input type=\"textbox\" name=\"text\" autofocus />");
    _client.println("      <input type=\"submit\" style=\"display: none\" />");
    _client.println("    </form>");
    _client.println("  </body>");
    _client.stop();
    return _uri_buffer;
  }

private:
  WiFiClient _client;
  std::string _buffer;
  std::string _uri_buffer;
};

void setup() {
  Serial.begin(115200);
  std::unique_ptr<SplitFlapDisplay> display = createSplitFlapDisplay();
  while (Serial.read() != '^') {}
  Serial.println("Battlecruiser Operational.");

  Serial.println("Homing flaps...");
  display->display("", 1000);

  if (WiFi.status() == WL_NO_MODULE) {
    Serial.println("Cannot connect to WiFi module.");
    exit(1);
  }
  String fv = WiFi.firmwareVersion();
  if (fv < WIFI_FIRMWARE_LATEST_VERSION) {
    Serial.println("Update WiFi firmware.");
    exit(1);
  }
  Serial.print("Connecting to WiFi (");
  Serial.print(SECRET_SSID);
  Serial.print(")...");
  Serial.println();
  int status = WiFi.begin(SECRET_SSID, SECRET_PASSWORD);
  while (status != WL_CONNECTED) {
    delay(100);
    status = WiFi.status();
  }
  Serial.println("Connecting to network...");

  IPAddress ip;
  while (true) {
    ip = WiFi.localIP();
    if (ip != IPAddress()) {
      break;
    }
    delay(100);
  }
  Serial.print("Starting server at http://");
  Serial.print(ip);
  Serial.print("/");
  Serial.println();


  Serial.println("Listening for connections...");
  WiFiServer server(80);
  server.begin();
  HttpServlet servlet;
  while (true) {
    WiFiClient client = server.available();
    if (client) {
      servlet.init(client);
      if (auto uri = servlet.run()) {
        if (auto path1 = readUntil(&*uri, '/')) {
          if (path1 == "") {
            if (auto path2 = readUntil(&*uri, '/')) {
              if (path2 == "flappy") {
                if (auto ig = readUntil(&*uri, '?')) {
                  if (auto ig = readUntil(&*uri, '=')) {
                    display->display(*uri, 1000);
                  }
                }
              }
            }
          }
        }
      }
      servlet.uninit();
    }
  }

  // Serial.setTimeout(std::numeric_limits<long>::max());
  // while (true) {
  //   while (Serial.available() == 0) {}
  //   int minStepDelay = Serial.parseInt();
  //   Serial.read();
  //   String input = Serial.readStringUntil('\n');
  //   display->display(std::string(input.c_str()), minStepDelay);
  //   Serial.println("Displayed.");
  //   delay(1000);
  // }
}

void loop() {
  Serial.println("Program terminating.");
  exit(0);
}
