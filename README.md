# Introduction
This is the implementation of an open hardware smart beer coaster.
As a core principle the beer coaster is simply a battery powered scale with LEDs and BLE.
These components enable a variety of interesting applications that can be implemented on top of the provided hardware.
## Default app
Currently the default app implementation is a hardcoded rainbow pattern, which gets dimmed relative to the analog input value of the force sensor's measurement. Due to part to part variance on the assembled PCBs the analog values can be different, which means the internal thresholds have to be adapted between PCBs.

## Electro-Mechanical Core Principle
The core component to allow for weight measurements is the piezo-electric force sensor. This sensor translates the force transferred from the coaster's feet into an electric signal, which can be used by the ESP32-C3 microcontroller.

# Code
The code is written in Rust and built with the Rust toolchain provided by Espressif. **It is highly recommended to use Linux for building the code**. The Windows version of the tools needed have some issues, which will make it a lot harder or almost impossible to build the project.


## Setup
There are a few things to install to properly build and flash the firmware onto the device. 
First and foremost the general Rust tooling has to be installed. This means installing `rustup` and making sure `cargo` is working.

### ESP Toolchain
For the ESP toolchain one has to follow the installation and quickstart guide on the `espup` repository.

https://github.com/esp-rs/espup?tab=readme-ov-file#installation

### Flash Tool
To flash the built firmware the `espflash` tool is needed:

https://github.com/esp-rs/espflash/blob/main/cargo-espflash/README.md#installation

## Building
If everything got properly installed and configured in the previous steps, to build the code one has to simply call:

```
cargo build
```

## Flashing
To build and flash the firmware onto the microcontroller the following command is needed:

```
cargo espflash flash -B 115200 --partition-table partitions.csv --erase-parts ota_1
```

### OTA
The reference beer coaster code implements an OTA update scheme with A/B partitioning.
To generate a OTA-image you simply have to run:

```
espflash save-image --chip esp32c3 target/riscv32imc-esp-espidf/debug/ota-updating-test ota-updating-test.bin
```

After this you have to copy the `ota-updating-test.bin` file into the `bt_flasher` folder and run the flasher with:
```
cargo run
```

## Serial
To connect to the serial interface of the beer coaster this command can be used:
```
espflash  monitor -p /dev/ttyUSB0 -B 115200
```

# PCB Design
The PCB is designed with KiCad. The project should contain all the components to work with it.
When ordering the current version a 1.6mm thick single layer PCB is required, due to assumptions from the mechanical base plate.

# Mechanical
This part of the project is done in FreeCAD and consists of 2 parts which act as the housing for the PCB. The mechanical parts are designed to be 3d printable. The base clips onto the PCB and holds itself in place. 
The key feature of the mechanical design are thin membrane sections, which allow these sections of the base plate to elastically deform. This enables the actuation of buttons on the PCB and the pressure sensor for the weight measurement.

## Material
The design has been tested with PET and ASA, but you can use anything that is:
* water resistant
* doesn't degrade under cyclical deformation
* allows for enough flex to actuate buttons and the pressure sensor

## Assembly Water Tightness
* It is recommended to use silicone to seal the sides of the assembly from water ingress.
* The center LED has to be covered with clear resin to prevent water from getting in.


## BoM
| Part | Amount |
| ---  | ---    |
| Base | 1      |
| Foot | 4      |

# Calculations
Contains some files with related calculations for the PCB design. 

# Licensing
The different parts of the beer coaster require different licenses, due to the licenses only covering for example either code or hardware. For this reason the repository contains different licenses. Each subfolder contains the appropriate license for all files and subfolders within.
The current licenses used are:
* GPL-3.0
* CERN-OHL-S-2.0
* Beerware
