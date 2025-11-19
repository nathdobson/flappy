#ifndef FLAPS_H
#define FLAPS_H

#include <vector>
#include <memory>
#include "Arduino.h"

class ShiftRegister;
class SplitFlap;

class SplitFlapDisplay {
public:
  SplitFlapDisplay(const SplitFlapDisplay&) = delete;
  SplitFlapDisplay& operator=(const SplitFlapDisplay&) = delete;
  SplitFlapDisplay(std::unique_ptr<ShiftRegister> reg, std::vector<std::unique_ptr<SplitFlap>> motors);
  ~SplitFlapDisplay();
  void display(std::string_view message, int minStepDelay);
private:
  std::unique_ptr<ShiftRegister> _reg;
  std::vector<std::unique_ptr<SplitFlap>> _motors;
};

std::unique_ptr<SplitFlapDisplay> createSplitFlapDisplay();

#endif