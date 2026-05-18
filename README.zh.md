# 拍拍你的笔记本🦞

> [English](README.md) | 简体中文

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Apple%20Silicon-black.svg)](https://support.apple.com/en-us/116943)

> 拍拍你的笔记本，你的 AI 助手会（嘴上 + 音效）拍回来。

**拍拍你的笔记本🦞** 是一个 Rust CLI 工具，通过内置加速度计检测 Apple Silicon MacBook 上的物理拍打和晃动，播放音效反馈，并在终端实时输出事件。

```
你: *拍了笔记本一下*
slap-your-laptop: *播放"嗷！"音效* + {"senderId":"slap","text":"SLAP #5 CHOC_MOYEN","correlationId":""}
```

## 目录

- [这东西为什么存在？](#这东西为什么存在)
- [运作方式](#运作方式)
- [运作模式](#运作模式)
- [系统需求](#系统需求)
- [快速开始](#快速开始)
- [严重等级](#严重等级)
- [事件类型](#事件类型)
- [CLI 参考](#cli-参考)
- [事件内容](#事件内容)
- [检测算法](#检测算法)
- [项目结构](#项目结构)
- [启动流程](#启动流程)
- [防误报措施](#防误报措施)
- [调校建议](#调校建议)
- [测试](#测试)
- [疑难排解](#疑难排解)
- [参与贡献](#参与贡献)
- [致谢](#致谢)
- [授权条款](#授权条款)

## 这东西为什么存在？

因为有人看了一眼每台 Apple Silicon MacBook 里的博世 BMI286 加速度计，然后想：「要是我的笔记本能感受到疼痛呢？」

这个工具以 800Hz 频率读取原始 IMU 数据，通过地震学等级的检测算法处理（原本是为地震检测设计的，现在被征用来检测办公室笔记本虐待行为），将冲击分为 6 个严重等级，从「那是蝴蝶吗？」到「你这个恶魔」，播放音效反馈，并把事件通过标准输出发送。

你的 MacBook 早就已经在默默地评判你了。现在它可以大声尖叫了。

## 运作方式

```
                    你的手
                        |
                        | (暴力行为)
                        v
┌─────────────────────────────────────┐
│  Apple Silicon MacBook              │
│  ┌───────────────────────────────┐  │
│  │ 博世 BMI286 IMU              │  │
│  │ (加速度计, ~800Hz 原始频率)  │  │
│  └──────────────┬────────────────┘  │
└─────────────────┼───────────────────┘
                  │
                  │ IOKit HID (需要 sudo，因为
                  │ 苹果也不信任你)
                  v
    ┌─────────────────────────────┐
    │ C 适配层 (iokit.c)          │
    │ - 唤醒 SPU 传感器驱动       │
    │ - 自动锁定加速度计 HID      │
    │ - 800Hz → 100Hz 降采样      │
    │ - 无锁环形缓冲区            │
    └──────────────┬──────────────┘
                   │
                   │ Q16 定点数 → 重力加速度 (g)
                   v
    ┌─────────────────────────────┐
    │ 检测器 (纯 Rust)            │
    │ ┌─────────┐ ┌────────────┐ │
    │ │ STA/LTA │ │   CUSUM    │ │
    │ │(3 尺度) │ │ (漂移检测) │ │
    │ ├─────────┤ ├────────────┤ │
    │ │ 峰度    │ │ Peak/MAD   │ │
    │ │(脉冲)   │ │ (离群点)   │ │
    │ └─────────┘ └────────────┘ │
    │                             │
    │ 高通滤波器移除重力分量      │
    │ (你的笔记本大概没有在坠落)  │
    └──────────────┬──────────────┘
                   │
                   │ 事件: 类型 + 严重等级 + 振幅
                   v
    ┌─────────────────────────────┐
    │ 分类                         │
    │                             │
    │ 拍打 = 短脉冲 (<100ms)      │
    │ 晃动 = 持续振荡 (>200ms)    │
    │                             │
    │ 6 个严重等级                 │
    │ (见下表)                     │
    └──────────────┬──────────────┘
                   │
                   │ 冷却时间 + 振幅过滤
                   v
    ┌─────────────────────────────┐
    │ 音频反馈                     │
    │ - 4 个内置音效包             │
    │ - 随机 / 递进模式            │
    │ - 根据冲击幅度调整音量       │
    │ - 支持自定义 MP3             │
    └──────────────┬──────────────┘
                   │
                   v
    ┌─────────────────────────────┐
    │ stdout JSON output          │
    │ {"senderId":"slap",         │
    │  "text":"SLAP #5 CHOC"}    │
    └─────────────────────────────┘
```

## 运作模式

本工具支持两种模式：

| 模式 | 指令 | 说明 |
|------|------|------|
| **Standalone** (默认) | `sudo slap-your-laptop` | 检测事件、播放音效反馈、并输出 JSON 到终端 |
| **MCP Server** | `sudo slap-your-laptop mcp` | 通过 stdio 提供 MCP 工具，供 AI 代理集成 |

两种模式共用相同的传感器线程和检测循环，区别在于事件的输出方式。Standalone 模式支持音频反馈（可用 `--no-audio` 禁用）。

### MCP 工具

| 工具 | 说明 |
|------|------|
| `slap_status` | 检测器阶段、已处理样本数、传感器健康状态、运行时间 |
| `slap_get_events` | 最近事件历史（可按数量、最低级别筛选） |
| `slap_wait_for_event` | 阻塞等待事件发生或超时 |
| `slap_get_config` | 获取当前的运行时配置 |
| `slap_set_config` | 动态更新配置（冷却时间、阈值等） |

## 系统需求

- **Apple Silicon Mac**（M1、M2、M3、M4 — 任何型号）
- **Root 权限**（`sudo`）— IOKit HID 加速计访问需要
- **Rust 工具链** — 建议使用 `rustup`

## 快速开始

### 1. 构建

```bash
git clone https://github.com/Galileo01/slap-your-laptop
cd slap-your-laptop
cargo build --release
```

### 2. 本地测试

```bash
sudo ./target/release/slap-your-laptop standalone
```

你会看到暖机进度条，然后进入布防阶段。当 `detector: ready` 出现时，就可以拍你的笔记本，听到音效反馈，并看到事件输出到终端。

```
warmup: [#########################] 0.0s remaining
arming: [#########################] 0.0s remaining
detector: [#########################] ready
>>> SLAP #5 [CHOC_MOYEN  amp=0.04231g] sources=["STA/LTA", "CUSUM", "PEAK"]  🔊 播放"嗷！"
```

如果什么都没出现：拍用力一点。这不是触摸屏。如果没听到声音：检查是否设置了 `--no-audio` 以及系统音量是否开启。

### 3. MCP 服务器模式

```bash
sudo ./target/release/slap-your-laptop mcp
```

以 stdio MCP 服务器启动，AI 代理可以通过标准 MCP 协议调用 `slap_status`、`slap_wait_for_event` 等工具来实时监控拍打事件。

## 严重等级

你的笔记本是个戏精。它把冲击分为 6 个等级：

| 等级 | 名称 | 发生了什么 | 你的笔记本的心情 |
|------|------|-----------|-----------------|
| 1 | MICRO_VIB | 你在旁边呼吸了一下 | "刚有动静吗？" |
| 2 | VIB_LEGERE | 打字太用力了 | "我感觉到了哦" |
| 3 | VIBRATION | 桌子被撞到、隔壁关门 | "不好意思？？" |
| 4 | MICRO_CHOC | 轻拍、用力敲 | "你不是认真的吧" |
| 5 | CHOC_MOYEN | 结结实实一巴掌 | "报警！报警！" |
| 6 | CHOC_MAJEUR | 全力出击，所有算法同时尖叫 | "我要打 AppleCare 电话了" |

分类基于有多少检测算法同意发生了什么以及振幅有多大。当 4 个检测器同时触发时，你的笔记本知道你是认真的。

## 事件类型

| 类型 | 持续时间 | 示例 |
|------|---------|------|
| **SLAP（拍打）** | < 100ms STA/LTA 激活时间 | 快速击打、敲击 |
| **SHAKE（晃动）** | > 200ms 持续振荡 | 愤怒地拿起笔记本、桌面振动 |

100-200ms 之间的事件会被分类为 UNKNOWN 并直接忽略——你的笔记本很困惑，选择不发表评论。

## CLI 参考

```
slap-your-laptop [选项] [命令]
```

命令：`standalone`（默认）、`mcp`

### 检测调校

| 参数 | 环境变量 | 默认值 | 说明 |
|------|---------|-------|------|
| `--cooldown <MS>` | `SLAP_COOLDOWN` | `500` | 事件之间的最小冷却时间（毫秒） |
| `--min-level <1-6>` | `SLAP_MIN_LEVEL` | `4` | 忽略低于此级别的事件 |
| `--min-slap-amp <G>` | `SLAP_MIN_SLAP_AMP` | `0.010` | 最小拍打振幅（g） |
| `--min-shake-amp <G>` | `SLAP_MIN_SHAKE_AMP` | `0.030` | 最小晃动振幅（g） |

### 音频反馈

| 参数 | 环境变量 | 默认值 | 说明 |
|------|---------|-------|------|
| `--sound <SOUND>` | `SLAP_SOUND` | `pain` | 音效包：`pain`、`sexy`、`halo`、`lizard`、`custom` |
| `--volume-scaling` | `SLAP_VOLUME_SCALING` | `true` | 根据冲击幅度调整音量 |
| `--speed <SPEED>` | `SLAP_SPEED` | `1` | 播放速度比率 |
| `--custom-path <DIR>` | `SLAP_CUSTOM_PATH` | — | 自定义音频目录（需配合 `--sound custom`） |
| `--custom-files <FILES>` | `SLAP_CUSTOM_FILES` | — | 逗号分隔的 MP3 文件路径（需配合 `--sound custom`） |
| `--list-audio <PACK>` | — | — | 列出音效包中的文件并退出 |
| `--no-audio` | `SLAP_NO_AUDIO` | — | 完全禁用音频播放 |

## 事件内容

每个事件以结构化 JSON 打印到 stdout：

```json
{"senderId":"slap","text":"SLAP #5 CHOC_MOYEN","correlationId":""}
```

或者晃动事件：

```json
{"senderId":"slap","text":"SHAKE #4 MICRO_CHOC","correlationId":""}
```

## 检测算法

四种算法在每个采样点上并行运行。对于「有人拍了笔记本」来说这确实大材小用了，但我们就是来玩信号处理的。

### STA/LTA（短期平均 / 长期平均）

借鉴自地震学。在 3 个时间尺度上比较近期能量与背景能量：

| 尺度 | 短窗口 | 长窗口 | 灵敏度 |
|------|-------|-------|--------|
| 快速 | 3 个采样 (30ms) | 100 个采样 (1s) | 捕捉尖锐脉冲 |
| 中等 | 15 个采样 (150ms) | 500 个采样 (5s) | 捕捉中等冲击 |
| 慢速 | 50 个采样 (500ms) | 2000 个采样 (20s) | 捕捉持续扰动 |

当比值超过启动阈值时，通道启动。启动持续时间决定是拍打还是晃动。

### CUSUM（累积和）

漂移检测——累积与运行均值的偏差。像记仇一样，小偏移不断累积直到突破阈值。

### 峰度（Kurtosis）

在 100 个采样的窗口上测量信号分布的"尖峭度"。正常噪声的峰度约等于 3。脉冲式拍打会使其飙升到 6 以上。简单说就是：「这看起来像不像有人打了什么东西？」

### Peak/MAD（中位绝对偏差）

在 200 个采样的窗口上进行稳健离群点检测。如果当前采样与中位数（MAD 估计）偏离超过 4 个标准差，说明刚才发生了异常。

## 项目结构

```
src/
├── main.rs            # CLI + 暖机/就绪交互 + 主循环 + 模式分发 + 音频线程
├── config.rs          # clap 派生 CLI 参数 + 环境变量 + 子命令
├── shared.rs          # SharedState, DetectorConfig, run_detection_loop()
├── sensor/
│   ├── mod.rs         # 模块导出
│   ├── iokit.rs       # Rust FFI: 环形缓冲区读取器, Q16→g 转换
│   └── iokit.c        # C 适配层: IOKit HID, SPU 驱动唤醒, 设备自动锁定
├── detector/
│   ├── mod.rs         # 4 种检测算法 + 严重等级分类器
│   └── ring.rs        # 固定容量环形缓冲区 (RingFloat)
├── audio/
│   ├── mod.rs         # AudioError, AudioCommand, spawn_audio_thread(), 类型重导出
│   ├── pack.rs        # SoundPackId, PlayMode, SoundPack（内置 + 自定义加载）
│   ├── player.rs      # AudioPlayer（基于 rodio，非阻塞，音量缩放）
│   └── tracker.rs     # SlapTracker（Random/Escalation 索引选择）
└── mcp/
    ├── mod.rs         # MCP 模块声明
    └── server.rs      # SlapServer: 5 个 MCP 工具 (rmcp)
```

### 为什么用 C 适配层？

IOKit 和 CoreFoundation 是 C 框架。你*可以*通过原始 FFI 从 Rust 调用它们，但那意味着 200 多行 `extern "C"` 声明、不透明类型转换和 `CFRelease` 编排。C 适配层约 240 行，处理所有 macOS 框架调用，向 Rust 暴露 3 个函数：

```c
int iokit_sensor_init(void);    // 归零环形缓冲区
void iokit_sensor_run(void);    // 唤醒传感器 + 执行 CFRunLoop（阻塞）
const uint8_t* iokit_ring_ptr(void);  // 共享环形缓冲区指针
```

### 设备自动锁定

Apple Silicon Mac 通过 `AppleSPUHIDDevice` 暴露 4-8 个 HID 设备。其中只有一个是加速度计。C 适配层使用投票系统自动检测正确的设备：

1. 打开所有厂商页面（`0xFF00`）HID 设备
2. 过滤 22 字节 IMU 格式的报告
3. 验证原始 L1 范数在合理重力范围（0.5g–4g）
4. 同一设备连续 3 次有效报告 → 锁定设备
5. 同一报告 ID 连续 6 次有效报告 → 锁定报告

这意味着同一个二进制文件可以在 M1、M2、M3、M4 上执行，无需硬编码设备索引。

## 启动流程

执行工具时，你会看到：

```
iokit: woke 8 SPU drivers
iokit: device 1: UsagePage=0xff00 Usage=255
iokit: registered accel callback on idx=0 usage=255
...
iokit: locked accelerometer device idx=0 usage=255
iokit: locked accelerometer reportID=0
warmup: [################---------] 0.9s remaining
arming: [#########################] 0.0s remaining
detector: [#########################] ready
```

**阶段一 — 暖机（2s）：** 高通滤波器和运行平均值需要时间稳定。暖机期间事件被抑制。

**阶段二 — 布防（1s）：** 暖机后再给一小段安静时间，让统计值稳定。这段期间仍会抑制事件。

**阶段三 — 就绪：** 检测器已上线。你的笔记本现在情绪就位。

## 防误报措施

因为没人希望自己打邮件的时候笔记本在那大喊「遇袭了」：

1. **暖机门控** — 前 200 个采样（2s）完全抑制
2. **布防门控** — 额外 100 个采样（1s）的安静稳定期
3. **UNKNOWN 事件丢弃** — 只发布 SLAP 和 SHAKE
4. **防打字误判** — 没有 PEAK 检测来源且振幅 < 0.03g 的 SLAP 事件会被直接忽略（键盘产生的低振幅微振动看起来像轻拍）
5. **振幅下限** — SLAP（0.01g）和 SHAKE（0.03g）分别可设定最小值
6. **严重等级过滤** — 默认 `--min-level 4` 完全忽略 1-3 级
7. **冷却时间** — 事件之间最少 500ms

## 调校建议

**太灵敏了？**（打字、桌面碰撞都会触发）

```bash
sudo ./target/release/slap-your-laptop --min-level 5 --min-slap-amp 0.025
```

**不够灵敏？**（需要揍一拳才能触发）

```bash
sudo ./target/release/slap-your-laptop --min-level 3 --min-slap-amp 0.005 --min-shake-amp 0.010
```

**被刷屏了？**（连续太多事件）

```bash
sudo ./target/release/slap-your-laptop --cooldown 3000  # 3 秒冷却时间
```

**想要不同的音效？**（试试其他音效包或自定义音频）

```bash
sudo ./target/release/slap-your-laptop --sound sexy          # 俏皮递进音效
sudo ./target/release/slap-your-laptop --sound halo          # 光环武器音效
sudo ./target/release/slap-your-laptop --sound custom --custom-path ~/my-sounds/  # 你自己的 MP3
```

## 测试

```bash
cargo test        # 单元测试（检测器、环形缓冲区、配置、MCP、集成路径）
cargo clippy      # 代码检查
cargo fmt --check # 格式检查
```

测试使用合成加速计数据——CI 过程中无需实际的笔记本暴力行为。

## 疑难排解

**"requires root privileges"**
→ 使用 `sudo` 执行。IOKit HID 需要它，没有绕过方法。

**"Failed to initialize IOKit HID sensor"**
→ 不是 Apple Silicon，或者你的 Mac 没有 BMI286 IMU。只支持 M 系列芯片。

**检测不到事件**
→ 等待「detector: ready」消息出现。用力拍掌托区域（不是屏幕，求你了）。检查 `--min-level` 是否设得太高。

**打字时触发事件**
→ 提高 `--min-slap-amp`（试试 `0.020` 或 `0.025`）。防打字误判功能能挡住大多数情况，但某些 MacBook 型号上的重度打字者可能需要更高的阈值。

**进度条卡住**
→ 传感器线程可能失败了。检查上方的 iokit 日志行是否有错误。某些 M4 Mac 上传感器的 usage page 可能不同——自动锁定系统应该能处理，但如果不行请提 issue。

## 参与贡献

欢迎贡献！本项目还在早期开发阶段，有很多可以改进的地方。

### 开发环境设定

```bash
git clone https://github.com/Galileo01/slap-your-laptop
cd slap-your-laptop
cargo build
```

### 执行测试

```bash
cargo test
cargo clippy
cargo fmt --check
```

### 欢迎贡献的方向

- **硬件测试** — 在不同 MacBook 型号（M1/M2/M3/M4）上试用并反馈表现
- **检测调校** — 改善误报过滤或提出新的算法
- **新输出模式** — MCP 以外的额外集成
- **文档** — 翻译、教程或改善疑难排解指南

请在开始大型修改前先开 issue 讨论方向。

## 致谢

检测算法移植自：

- [taigrr/spank](https://github.com/taigrr/spank) — 原版 Go 实现
- [taigrr/apple-silicon-accelerometer](https://github.com/taigrr/apple-silicon-accelerometer)

使用的函数库：

- [clap](https://docs.rs/clap) — CLI 框架
- [tokio](https://tokio.rs) — 异步运行时
- [rmcp](https://docs.rs/rmcp) — MCP 服务器框架
- [rodio](https://docs.rs/rodio) — 音频播放
- [cc](https://docs.rs/cc) — C 适配层编译

## 授权条款

本项目采用 MIT 授权条款——详见 [LICENSE](LICENSE) 文件。

请负责任地拍打。
