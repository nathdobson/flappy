#include <vector>
#include <optional>
#include <memory>
#include "flaps.h"
#include <WiFiS3.h>
#include "arduino_secrets.h"
#include <ArduinoMDNS.h>
#include <WiFiUdp.h>

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

void urlDecode(std::string_view input, std::string* output) {
  output->clear();
  int index = 0;
  while (index < input.length()) {
    if (input[index] == '%') {
      if (index + 2 >= input.length()) {
        return;
      }
      std::array<char, 3> temp = { input[index + 1], input[index + 2], 0 };
      Serial.println(temp[0]);
      Serial.println(temp[1]);
      Serial.println(temp[2]);
      long result = strtol(&temp[0], nullptr, 16);
      Serial.println(result);
      output->push_back(result);
      index += 3;
    } else {
      output->push_back(input[index]);
      index++;
    }
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
    _client.println("    <style>");
    _client.println("      body {");
    _client.println("        background: #235c40;");
    _client.println("        padding: 0 24px;");
    _client.println("        margin: 0;");
    _client.println("        height: 100vh;");
    _client.println("        color: white;");
    _client.println("        justify-content: center;");
    _client.println("        align-items: center;");
    _client.println("        display: flex;");
    _client.println("      }");
    _client.println("      h1 {");
    _client.println("        text-align: center;");
    _client.println("      }");
    _client.println("      .inputbox {");
    _client.println("        width:1000px;");
    _client.println("        font-size:80pt;");
    _client.println("        background-color: #163b29;");
    _client.println("        color: white;");
    _client.println("        font-family: Consolas,Monaco,Lucida Console,Liberation Mono,DejaVu Sans Mono,Bitstream Vera Sans Mono,Courier New, monospace;");
    _client.println("      }");
    _client.println("    </style>");
    _client.println("  </head>");
    _client.println("  <body>");
    _client.println("    <div>");
    _client.println("      <h1>Hi, I'm Flappy! What should I display?</h1>");
    _client.println("      <form>");
    _client.println("        <input class=\"inputbox\" type=\"textbox\" name=\"text\" autofocus />");
    _client.println("        <input type=\"submit\" style=\"display: none\" />");
    _client.println("      </form>");
    _client.println("    </div>");
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
  const char* fv = WiFi.firmwareVersion();
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

  Serial.println("Advertising via MDNS");
  WiFiUDP udp;
  MDNS mdns(udp);
  mdns.begin(WiFi.localIP(), "flappy");

  Serial.println("Listening for connections...");
  WiFiServer server(80);
  server.begin();
  HttpServlet servlet;
  std::string urlDecoded;
  while (true) {
    mdns.run();
    WiFiClient client = server.available();
    if (client) {
      servlet.init(client);
      if (auto uri = servlet.run()) {
        if (auto ig = readUntil(&*uri, '?')) {
          if (auto ig = readUntil(&*uri, '=')) {
            urlDecode(*uri, &urlDecoded);
            display->display(urlDecoded, 1000);
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
