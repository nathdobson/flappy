# A 3d-printed Split Flap Display Project

This project is a redesign of David
Kingsman's [Split Flap Display](https://www.printables.com/model/69464-split-flap-display).

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

* ["Dupont" crimping tool](https://www.amazon.com/IWISS-SN-28B-Crimping-AWG28-18-Dupont/dp/B00OMM4YUY?th=1) for
  assembling cables.
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

## Repository structure

The repository is divided into several hardware and software components:

* [common/](common/) A cargo workspace with utilities and configuration in use by several components
* [driver/](driver/) A KiCad PCB design for the motor driver attached to each character.
* [mobile/](mobile/) An Android app made with MIT App Inventor for configuring the display over Bluetooth.
* [models/](models/) A cargo crate with binaries that generate .3mf files for each 3D-printed part.
* [firmware/](firmware/) A cargo workspace with the firmware for the Raspberry Pi Pico 2 W.
* [setup/](setup/) A cargo workspace with a binary for configuring the display over USB or Bluetooth.
* [submodules/](submodules/) A set of git submodules with forked dependencies.
* [submodules/patina-rs](submodules/patina-rs/) A CAD library for generating 3D meshes from SDFs (signed distance
  functions).
* [web-client](web-client/) A WASM web application for activating the display over MQTT.

## Instructions

### Hardware

1. Download the [latest release](https://github.com/nathdobson/flappy/releases/latest).
1. Order driver PCBs with JLCPCB:
    * `driver-jlcpcb-GERBER.zip` for producing the raw boards.
    * `driver-jlcpcb-BOM.csv` for ordering parts.
    * `driver-jlcpcb-CPL.csv` for placing and assembling parts.
1. For each module, print one each of the following:
    * `model-housing.3mf`: An enclosure for the module.
    * `model-inner.3mf`: The inner half of the drum.
    * `model-outer.3mf`: The outer half of the drum.
    * `model-flaps.3mf`: All flaps, stacked with PLA support.
1. Press M2 and M3 Ruthex threaded inserts into the prints with a soldering iron.
1. Glue magnets into the inner drums. Ensure magnets are in the appropriate orientation for your sensors.
1. Loosely connect inner and outer drums with screws.
1. Insert flaps into drum. Aligning the first letter with the magnet will simplify later calibration.
1. Tighten drum screws.
1. Remove the pull-up resistor or pull-up LED from the sensors[^1]. This is easy with flush cutters.
1. Route the motor and sensor cables through housings.
1. Connect all cables.
1. Connect the motors, sensors, and drivers to the housings with M2 and M3 screws. Ensure the motor tabs are properly
   centered on the supports.
1. Press the drum assemblies onto the motor axle. Ensure all flaps are pointed clockwise so they won't jam as the drum
   assembly rotates counter-clockwise.
1. Install [picotool](https://github.com/raspberrypi/picotool)

### Software

1. Set up an MQTT broker (e.g. with https://www.emqx.com/).
1. Connect the Pico to a computer with a USB cable. Flash the firmware with:
   `picotool load -f -u -v -x -t elf firmware.elf`
1. Configure the device by executing the appropriate `setup-*` binary.
1. Navigate to https://flappy-7d77d.web.app/www/ or set up custom web hosting by unzipping `web-client.zip` and
   uploading to a web host.
1. Connect to the display by specifying the parameters for the MQTT broker.

These files are optimized for the Bambu Labs A1 mini with AMS, but should theoretically support other printers.

[^1]
The KY-003 board contains an A3144 digital hall sensor chip. The A3144 has a minimum Vcc of 4.5V and open-collector
active-low digital output. The KY-003 includes a resistor and LED in series to pull-up the output signal to Vcc. The
Pico's maximum logic voltage is 3.3V, so it cannot connect to a KY-003 without some extra work. Also, the LED introduces
a
voltage drop, which makes it's use as a pull-up in the first place questionable. Instead, we remove the pull-ups from
the KY-003, provide 12V to Vcc, and add a pull-up resistor to 3.3V on the driver board. Failure to remove the pull-ups
will likely result in damage to the driver boards.