#include <vector>
#include <optional>
#include <memory>
#include "flaps.h"
#include <WiFiS3.h>



void setup() {
  Serial.begin(115200);

  // std::unique_ptr<SplitFlapDisplay> display = createSplitFlapDisplay();
  // display->display("", 1000);
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
  exit(0);
}
