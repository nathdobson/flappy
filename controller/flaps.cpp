#include "flaps.h"

class ShiftRegister {
public:
  ShiftRegister(const ShiftRegister&) = delete;
  ShiftRegister& operator=(const ShiftRegister&) = delete;
  ShiftRegister(int dataPin, int clockPin, int latchPin, int length)
    : _dataPin(dataPin), _clockPin(clockPin), _latchPin(latchPin), _buffer(length) {
    pinMode(_dataPin, OUTPUT);
    pinMode(_clockPin, OUTPUT);
    pinMode(_latchPin, OUTPUT);
    flushPins();
  }
  ~ShiftRegister() {
    for (int i = 0; i < _buffer.size(); i++) {
      _buffer[i] = false;
    }
    flushPins();
  }
  void setPin(int pin, bool value) {
    _buffer[pin] = value;
  }
  void flushPins() {
    digitalWrite(_latchPin, LOW);
    for (auto it = _buffer.rbegin(); it != _buffer.rend(); ++it) {
      digitalWrite(_dataPin, *it);
      digitalWrite(_clockPin, HIGH);
      digitalWrite(_clockPin, LOW);
    }
    digitalWrite(_latchPin, HIGH);
  }

private:
  int _dataPin;
  int _clockPin;
  int _latchPin;
  std::vector<bool> _buffer;
};

class ShiftRegisterPin {
public:
  ShiftRegisterPin()
    : _reg(nullptr), _pin(-1) {}
  ShiftRegisterPin(ShiftRegister* reg, int pin)
    : _reg(reg), _pin(pin) {
  }
  void set(bool value) {
    _reg->setPin(_pin, value);
  }
private:
  ShiftRegister* _reg;
  int _pin;
};

constexpr int MOTOR_PHASE_COUNT = 4;

const std::array<std::array<bool, MOTOR_PHASE_COUNT>, 8> SEQUENCE = { {
  { 1, 0, 0, 0 },
  { 1, 1, 0, 0 },
  { 0, 1, 0, 0 },
  { 0, 1, 1, 0 },
  { 0, 0, 1, 0 },
  { 0, 0, 1, 1 },
  { 0, 0, 0, 1 },
  { 1, 0, 0, 1 },
} };

class StepperMotor {
public:
  StepperMotor(const StepperMotor&) = delete;
  StepperMotor& operator=(const StepperMotor&) = delete;
  StepperMotor(std::array<ShiftRegisterPin, MOTOR_PHASE_COUNT> pins)
    : _step(0), _pins(pins) {
    disable();
  }
  void step(bool forward) {
    for (int i = 0; i < MOTOR_PHASE_COUNT; i++) {
      _pins[i].set(SEQUENCE[_step][i]);
    }
    if (forward) {
      _step++;
      if (_step >= SEQUENCE.size()) {
        _step = 0;
      }
    } else {
      _step--;
      if (_step < 0) {
        _step = SEQUENCE.size() - 1;
      }
    }
  }

  void disable() {
    for (int i = 0; i < MOTOR_PHASE_COUNT; i++) {
      _pins[i].set(false);
    }
  }
private:
  int _step;
  std::array<ShiftRegisterPin, MOTOR_PHASE_COUNT> _pins;
};

enum class HallSignal {
  FLAT,
  RISING,
  FALLING,
};

class HallSensor {
public:
  HallSensor(const HallSensor&) = delete;
  HallSensor& operator=(const HallSensor&) = delete;
  HallSensor(int pin)
    : _pin(pin) {
    pinMode(_pin, INPUT);
    _previous = digitalRead(_pin);
  }
  HallSignal readNext() {
    bool next = digitalRead(_pin);
    if (next == _previous) {
      return HallSignal::FLAT;
    } else if (next) {
      _previous = next;
      return HallSignal::RISING;
    } else {
      _previous = next;
      return HallSignal::FALLING;
    }
  }
private:
  int _pin;
  bool _previous;
};

const char* const FLAPS = " ABCDEFGHIJKLMNOPQRSTUVWXYZ$&#0123456789:.-?!";
const int FLAP_COUNT = 45;
const int STEPS_PER_REVOLUTION = 4096;
const int STEPS_PER_FLAP = 91;

int flapForChar(char c) {
  char* pos = strchr(FLAPS, toupper(c));
  if (pos == nullptr) {
    return 0;
  } else {
    return pos - FLAPS;
  }
}

int computeCalibration(char macroCalib, int microCalib) {
  int total = (FLAP_COUNT - flapForChar(macroCalib) - 1) * STEPS_PER_FLAP + microCalib + STEPS_PER_FLAP / 2;
  return (total + STEPS_PER_REVOLUTION) % STEPS_PER_REVOLUTION;
}

class SplitFlap {
public:
  SplitFlap(const SplitFlap&) = delete;
  SplitFlap& operator=(const SplitFlap&) = delete;
  SplitFlap(std::unique_ptr<StepperMotor> motor, std::unique_ptr<HallSensor> sensor, int calibration, long start)
    : _sensor(std::move(sensor)),
      _motor(std::move(motor)),
      _calibration(calibration),
      _position(0),
      _homed(false),
      _target(0),
      _lastStepTime(start) {
  }
  int maximumRemainingSteps() {
    if (_homed) {
      return (STEPS_PER_REVOLUTION + _target - _position) % STEPS_PER_REVOLUTION;
    } else {
      return STEPS_PER_REVOLUTION + _target - _position;
    }
  }
  long fastestGuaranteedEndTime(int minStepDelay) {
    return _lastStepTime + maximumRemainingSteps() * minStepDelay;
  }
  long nextStepTime(long endTime, int minStepDelay) {
    if (_homed && _position == _target) {
      return std::numeric_limits<long>::max();
    }
    long delay = 0;
    if (_homed) {
      int distance = (STEPS_PER_REVOLUTION + _target - _position) % STEPS_PER_REVOLUTION;
      delay = (endTime - _lastStepTime) / distance;
    } else {
      delay = minStepDelay;
    }
    return _lastStepTime + delay;
  }
  void disable() {
    _motor->disable();
  }
  void step(long now) {
    _position++;
    _motor->step(false);
    _lastStepTime = now;
    if (_sensor->readNext() == HallSignal::FALLING) {
      _position = 0;
      _homed = true;
    }
  }
  void setTarget(char c) {
    int flap = flapForChar(c);
    int step = flap * STEPS_PER_FLAP;
    _target = (step + _calibration) % STEPS_PER_REVOLUTION;
  }
private:
  std::unique_ptr<HallSensor> _sensor;
  std::unique_ptr<StepperMotor> _motor;
  int _calibration;
  int _position;
  bool _homed;
  int _target;
  long _lastStepTime;
};

SplitFlapDisplay::SplitFlapDisplay(std::unique_ptr<ShiftRegister> reg, std::vector<std::unique_ptr<SplitFlap>> motors)
  : _reg(std::move(reg)),
    _motors(std::move(motors)) {
}
SplitFlapDisplay::~SplitFlapDisplay() {}

void SplitFlapDisplay::display(std::string_view message, int minStepDelay) {
  for (int i = 0; i < _motors.size(); i++) {
    if (i < message.length()) {
      _motors[i]->setTarget(message[i]);
    } else {
      _motors[i]->setTarget(' ');
    }
  } 

  std::vector<int> usedMotors;
  for (int i = 0; i < 10; i++) {
    usedMotors.push_back(i);
  }
  std::vector<int> nextMotors;
  while (true) {
    long endTime = 0;
    for (int motor : usedMotors) {
      endTime = std::max(endTime, _motors[motor]->fastestGuaranteedEndTime(minStepDelay));
    }
    nextMotors.clear();
    long nextMotorStepTime = std::numeric_limits<long>::max();
    for (int motor : usedMotors) {
      long stepTime = _motors[motor]->nextStepTime(endTime, minStepDelay);
      if (stepTime < nextMotorStepTime) {
        nextMotors.clear();
        nextMotors.push_back(motor);
        nextMotorStepTime = stepTime;
      } else if (stepTime == nextMotorStepTime && stepTime < std::numeric_limits<long>::max()) {
        nextMotors.push_back(motor);
      }
    }
    if (nextMotors.empty()) {
      break;
    }
    long now = micros();
    if (now < nextMotorStepTime) {
      long delay = nextMotorStepTime - now;
      delayMicroseconds(delay);
      now = nextMotorStepTime;
    }

    for (int nextMotor : nextMotors) {
      _motors[nextMotor]->step(now);
    }
    _reg->flushPins();
  }
  for (int motor : usedMotors) {
    _motors[motor]->disable();
  }
  _reg->flushPins();
}

std::unique_ptr<SplitFlapDisplay> createSplitFlapDisplay() {
  const int MOTOR_COUNT = 10;
  const int DATA_PIN = 2;
  const int CLOCK_PIN = 3;
  const int LATCH_PIN = 4;
  const std::array<int, MOTOR_COUNT> HALL_PINS = { 12, 11, 10, 9, 8, 14, 15, 16, 17, 18 };
  const std::array<char, MOTOR_COUNT> MACRO_CALIBRATIONS = { ' ', 'V', 'K', 'U', 'Q', 'G', '$', 'R', 'R', '9' };
  const std::array<int, MOTOR_COUNT> MICRO_CALIBRATIONS = { 60, 40, 86, 40, 55, 248, 0, 50, 50, 25 };

  auto reg = std::make_unique<ShiftRegister>(DATA_PIN, CLOCK_PIN, LATCH_PIN, MOTOR_COUNT * 4);
  std::vector<std::unique_ptr<SplitFlap>> motors;
  long start = micros();
  for (int motor = 0; motor < MOTOR_COUNT; motor++) {
    std::array<ShiftRegisterPin, MOTOR_PHASE_COUNT> motorPins;
    for (int k = 0; k < MOTOR_PHASE_COUNT; k++) {
      motorPins[k] = ShiftRegisterPin(&*reg, motor * MOTOR_PHASE_COUNT + k);
    }
    auto stepper = std::make_unique<StepperMotor>(motorPins);
    auto sensor = std::make_unique<HallSensor>(HALL_PINS[motor]);
    int calibration = computeCalibration(MACRO_CALIBRATIONS[motor], MICRO_CALIBRATIONS[motor]);
    Serial.print("Calibration ");
    Serial.print(motor);
    Serial.print(" is ");
    Serial.print(calibration);
    Serial.println();
    motors.push_back(std::make_unique<SplitFlap>(std::move(stepper), std::move(sensor), calibration, start));
  }
  return std::make_unique<SplitFlapDisplay>(std::move(reg), std::move(motors));
}