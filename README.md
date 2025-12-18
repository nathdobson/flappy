# A 3d-printed Split Flap Display Project

This project is a redesign of David Kingsman's [Split Flap Display](https://www.printables.com/model/69464-split-flap-display).

## How it works
This display shows a sequence of characters in individual 3d-printed modules. Each module contains a drum with flaps
displaying individual symbols, like a Rolodex. A motor rotates the drum to change from one symbol to another.

A single microcontroller (MCU) listens to updates from an MQTT server over WiFi. Once the MCU receives an update, it
begins communicating with a series of shift registers over a serial connection. The shift registers drive one unipolar
stepper motor and read one hall sensor for each character. Once the hall sensor detects a magnet in the drum, it can
determine the absolute orientation of the drum. To get to the desired letter, the motor spins a certain number of steps
past the point detected by the hall sensor.

## Prerequisites
### Tools
Required
* 3D printer with multi-material support (e.g. Bambu Labs A1 mini with AMS).
* [Soldering iron with temperature adjustment](https://www.amazon.com/Soldering-Digital-Welding-Portable-Electric/dp/B08R3515SF?th=1)
  for assembling boards and pushing Ruthex inserts into 3D printed parts.

Recommended
* ["Dupont" crimping tool](https://www.amazon.com/IWISS-SN-28B-Crimping-AWG28-18-Dupont/dp/B00OMM4YUY?th=1) for assembling cables.
* Multimeter for basic connection and voltage tests.

## Bill of Materials
### For each display

| Name of Part                                                                    | Quantity |
|---------------------------------------------------------------------------------|----------|
| [Raspberry Pi Pico 2 W MCU with headers](https://www.adafruit.com/product/6328) | 1        |
| USB micro power supply for MCU                                                  | 1        |
| 12v power supply for motors (250mA per character)                               | 1        | 

### For each one-character module
| Name of Part                                                                                                                                                                                       | Quantity |
|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------|
| [Custom driver board](https://github.com/nathdobson/flappy/tree/main/driver2) with a [DRV8804PWPR](https://www.ti.com/product/DRV8804) and [SN74HCS16507](https://www.ti.com/product/SN74HCS16507) | 1        |
| [28BYJ-48 12v motor](https://www.amazon.com/Podazz-4-Phase-5-Line-Stepper-28BYJ-48/dp/B0DCFV3C2C)                                                                                                  | 1        |
| [KY-003 hall sensor module](https://www.amazon.com/Ransanx-Magnetic-KY-003-3-3V-5V-Pressure/dp/B0F32N1LHX?th=1)                                                                                    | 1        |
| [PLA (background color)](https://us.store.bambulab.com/products/pla-basic-filament)                                                                                                                | ~217 g   |
| [PLA (foreground color)](https://us.store.bambulab.com/products/pla-basic-filament)                                                                                                                | ~18 g    |
| [PLA support](https://us.store.bambulab.com/products/support-for-pla-new) for stacked prints (optional)                                                                                            | ~30 g    |
| Ruthex M2 and M3 inserts                                                                                                                                                                           |          |
| Assorted M2 and M3 button-head screws                                                                                                                                                              |          |
| Assorted dupont crimps, housings, and pin headers                                                                                                                                                  |          |
| [26 AWG UL1061 stranded wire](https://www.remingtonindustries.com/hook-up-wire/hook-up-wire-26-awg-ul1061-stranded-kit-2-color-sets-2-spool-sizes-available) for hall sensor cables                |          |
| Assorted solid wire for manually soldered boards                                                                                                                                                   |          |
| Assorted stranded wire for cables                                                                                                                                                                  |          |
| Magnets for drum                                                                                                                                                                                   |          |

## Repository
The repository is divided into several hardware and software components:
* [common/](common/) A cargo workspace with utilities and configuration in use by several components
* [driver3/](driver3/) A KiCad PCB design for the motor driver attached to each character.
* [mobile/](mobile/) An Android app made with MIT App Inventor for configuring the display over Bluetooth.
* [models/](models/) A cargo workspace with binaries that generate .3mf files for each 3D-printed part.
* [pico/](pico/) A cargo workspace with the firmware for the Raspberry Pi Pico 2 W.
* [setup/](setup/) A cargo workspace with a binary for configuring the display over USB or Bluetooth.
* [submodules/](submodules/) A set of git submodules with forked dependencies.
* [submodules/patina-rs](submodules/patina-rs/) A CAD library for generating 3D meshes from SDFs (signed distance functions).
* [web-client](web-client/) A WASM web application for activating the display over MQTT.

## Instructions
For each module, print one each of the following:
* [Housing](models/simplified/housing.3mf)
* [Inner Drum](models/simplified/inner.3mf)
* [Outer Drum](models/simplified/outer.3mf)
* [Flaps](models/flaps.3mf)

These files are optimized for the Bambu Labs A1 mini with AMS, but should theoretically support other printers.

