# Changelog

## [0.1.3](https://github.com/Galileo01/slap-your-laptop/compare/v0.1.2...v0.1.3) (2026-05-13)


### Features

* add audio module with sound packs, tracker escalation logic, and TODO for scale optimization ([#3](https://github.com/Galileo01/slap-your-laptop/issues/3)) ([66b2f55](https://github.com/Galileo01/slap-your-laptop/commit/66b2f55a7b46c7c8d37e010730759109b7725265))

## [0.1.2](https://github.com/Galileo01/slap-your-laptop/compare/v0.1.1...v0.1.2) (2026-05-10)


### Features

* add CLI config with clap derive + env var support ([e786b79](https://github.com/Galileo01/slap-your-laptop/commit/e786b792088d4abaacaf50b373fc84da893e753c))
* add IOKit HID accelerometer sensor via C shim ([c1bff99](https://github.com/Galileo01/slap-your-laptop/commit/c1bff998885f69792507cf662ca69e35a0020b1d))
* add MCP server mode and comprehensive docs ([#1](https://github.com/Galileo01/slap-your-laptop/issues/1)) ([6f4b610](https://github.com/Galileo01/slap-your-laptop/commit/6f4b610ab5eaf0b019ebe25966c14ffd4b2cfb9f))
* add MQTT publisher with OpenClaw inbound format ([d299650](https://github.com/Galileo01/slap-your-laptop/commit/d29965087299b41e8e9e88705ba2f817d26e71a5))
* add RingFloat ring buffer with tests ([9208275](https://github.com/Galileo01/slap-your-laptop/commit/9208275f5ad195b5125895f28d8c83dddbea1c5c))
* add slap vs shake discrimination via STA/LTA duration ([e057fae](https://github.com/Galileo01/slap-your-laptop/commit/e057fae0c4c8a7c782642b4f03ca806f70143093))
* add vibration detector with STA/LTA, CUSUM, Kurtosis, Peak/MAD ([eebf578](https://github.com/Galileo01/slap-your-laptop/commit/eebf57813a1c47577dee11f95281848251d83f7b))
* rename project to slap-your-laptop, remove OpenClaw integration, add Simplified Chinese README ([77891fe](https://github.com/Galileo01/slap-your-laptop/commit/77891fe16cee5cb0566bc15fdc9427e76b14c964))
* rename project to slap-your-laptop, remove OpenClaw integration… ([0c10ceb](https://github.com/Galileo01/slap-your-laptop/commit/0c10ceb95f5f0bb5f915644f95b0f271af36234e))
* scaffold slap-your-openclaw project ([8775b2d](https://github.com/Galileo01/slap-your-laptop/commit/8775b2d9aea7b33c558611ba6164b87cb67eb89b))
* wire main loop - sensor → detector → MQTT publisher ([f742d74](https://github.com/Galileo01/slap-your-laptop/commit/f742d740ca173e117fae81c59c3833883d32906f))


### Bug Fixes

* change default MQTT topic to openclaw/inbound ([c9c9b11](https://github.com/Galileo01/slap-your-laptop/commit/c9c9b11057667eeefc2de70e5397b5e17dbf7c78))

## [0.1.1](https://github.com/sinhong2011/slap-your-laptop/compare/v0.1.0...v0.1.1) (2026-03-02)


### Features

* add CLI config with clap derive + env var support ([e786b79](https://github.com/sinhong2011/slap-your-laptop/commit/e786b792088d4abaacaf50b373fc84da893e753c))
* add IOKit HID accelerometer sensor via C shim ([c1bff99](https://github.com/sinhong2011/slap-your-laptop/commit/c1bff998885f69792507cf662ca69e35a0020b1d))
* add MCP server mode and comprehensive docs ([#1](https://github.com/sinhong2011/slap-your-laptop/issues/1)) ([6f4b610](https://github.com/sinhong2011/slap-your-laptop/commit/6f4b610ab5eaf0b019ebe25966c14ffd4b2cfb9f))
* add MQTT publisher with OpenClaw inbound format ([d299650](https://github.com/sinhong2011/slap-your-laptop/commit/d29965087299b41e8e9e88705ba2f817d26e71a5))
* add RingFloat ring buffer with tests ([9208275](https://github.com/sinhong2011/slap-your-laptop/commit/9208275f5ad195b5125895f28d8c83dddbea1c5c))
* add slap vs shake discrimination via STA/LTA duration ([e057fae](https://github.com/sinhong2011/slap-your-laptop/commit/e057fae0c4c8a7c782642b4f03ca806f70143093))
* add vibration detector with STA/LTA, CUSUM, Kurtosis, Peak/MAD ([eebf578](https://github.com/sinhong2011/slap-your-laptop/commit/eebf57813a1c47577dee11f95281848251d83f7b))
* scaffold slap-your-laptop project ([8775b2d](https://github.com/sinhong2011/slap-your-laptop/commit/8775b2d9aea7b33c558611ba6164b87cb67eb89b))
* wire main loop - sensor → detector → MQTT publisher ([f742d74](https://github.com/sinhong2011/slap-your-laptop/commit/f742d740ca173e117fae81c59c3833883d32906f))


### Bug Fixes

* change default MQTT topic to openclaw/inbound ([c9c9b11](https://github.com/sinhong2011/slap-your-laptop/commit/c9c9b11057667eeefc2de70e5397b5e17dbf7c78))
