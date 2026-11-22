# CC Switch 定制增强版 (Overlay & Patchset)

> [!IMPORTANT]
> **非官方定制修改与技术补丁集 (Unofficial Custom Overlay)**
> 
> 本仓库由 [ZhangJinHui0627](https://github.com/ZhangJinHui0627/CCSWITCH) 维护。
> 原始项目版权归原作者 **Jason Young** 及 [farion1231/cc-switch](https://github.com/farion1231/cc-switch) 所有（遵循 [MIT License](./LICENSE)）。修改声明详见 [NOTICE.md](./NOTICE.md)。
> 本仓库**不存放上游完整源代码**，仅收录个人对 CC Switch v4.0.20 的**增量定制补丁集（Patchset）**、**核心独立模块源码**与**技术架构见解**。

---

## 📖 项目背景与定位

[CC Switch](https://github.com/farion1231/cc-switch) 是一款优秀的 AI 编程工具跨平台管理套件（支持 Claude Code, Codex, Gemini CLI 等）。然而在重度使用国产及前沿 Agent 工具链时，原作在**多工具日志聚合**、**前沿模型动态定价**以及**推理思考强度（Reasoning Effort）审计**方面存在空白。

本项目基于 **CC Switch v4.0.20** 构建了一套完整的独立增强层，通过增量补丁（69 个变更文件，+2156 / -223 行代码）实现深度的会话审计、思考强度捕获与模型计费自洽。

---

## 💡 技术见解与架构思考 (Deep Insights)

在定制与重构 CC Switch 的过程中，针对多 Agent 工具的协同审计与计费，我总结了以下四项核心见解与设计实践：

### 1. 多 Agent 工具链时代的用量孤岛与日志归一化
* **现状痛点**：DeepSeek Harness (DSH)、Antigravity (AGY)、ZCode 等新一代桌面开发工具的日志协议互不兼容。例如 DSH 采用多帧流式 Zstandard 压缩存储（`session.v4.jsonl.zstd`，帧标识 `28 B5 2F FD`），而 AGY 采用分目录的 JSONL 归档。
* **架构解法**：在 Tauri 后端构建可插拔的 `SessionUsageService`，通过 Rust 原生流解压器实时解析 DSH 会话帧，提取 `request_id`、`token_semantics`（输入/输出/缓存细分）并持久化至 SQLite，打破了各 Agent 工具之间的用量孤岛。

### 2. 思考强度（Reasoning Effort）在模型成本核算中的关键性
* **现状痛点**：前沿推理模型（如 Gemini 3.8 Flash High、GLM-5.3-Flash Max、DeepSeek V4 Pro Max、Claude 3.7/4.6 Thinking）引入了显式思考强度。不同档位不仅响应质量不同，且由于推理 token 生成量激增，真实成本产生倍数级差异。原版 CC Switch 仅截取裸模型名称，丢失了思考强度元数据。
* **架构解法**：
  * 在 DSH 会话同步层（`session_usage_dsh.rs`）捕获 `model/selection` 和 `request/header` 事件中的 `reasoningEffort` 标记，动态合成模型后缀（如 `gemini-3.8-flash-high`）；
  * 在统计分析层（`usage_stats.rs`）设计了**双向剥离查价容错链**：支持 9 种 effort 后缀（`-minimal, -low, -medium, -high, -xhigh, -max, -thinking, -sg, -tiered`）的动态剥离与前缀模糊回退，确保既能在明细中展示精确的思考档位，又能在汇总时命中基础定价。

### 3. 模型命名演进与查价容错链（Price Resolution Chain）
* **现状痛点**：国内模型快速迭代，客户端上报的 ID 经常混用长短名（例如腾讯混元 `Hunyuan Hy4 / hy4free / hunyuan-hy4` 均指向 `hy4`）。若数据库完全依赖精确外键匹配，会导致大量记录出现“未定价 / 成本为 0”的假象。
* **架构解法**：
  * 引入**短名优先规范**：在数据库层建立别名自动短名触发器（`trg_short_hy3_model`、`trg_short_hy4_model`、`trg_short_hy4flash_model`）；
  * 构建**三级查价回退机制**：`精确匹配 -> 剥离思考后缀匹配 -> 连字符/点号归一化匹配 -> 前缀 LIKE '{id}-%' 回退`，完美解决模型更新引发的漏计费。

### 4. 客户端主权与去中心化稳定锁定
* **现状痛点**：对于深度定制工具，官方自动升级经常会静默覆盖本地定制逻辑，甚至引入破坏性依赖变动（如上游 v4.0.33+ 剧烈变动）。
* **架构解法**：彻底切除官方自动更新检查（`updater.ts`）与第三方遥测网络连接，将版本稳稳锁定在经过全面回归验证的 `v4.0.20` 纯净版，保证开发环境的极致稳定与隐私安全。

---

## 🛠️ 定制修改全景清单 (69 Files Changed)

本补丁集的所有改动均严格遵循模块化原则，变更分布如下：

| 模块分类 | 核心文件 | 改动内容与目的 |
| :--- | :--- | :--- |
| **会话同步引擎** | `src-tauri/src/services/session_usage_dsh.rs`<br>`src-tauri/src/services/session_usage_agy.rs`<br>`src-tauri/src/services/session_usage_zcode.rs` | 新增 DSH、Antigravity、ZCode 的本地日志增量监听、Zstd 解码与入库逻辑 |
| **思考与用量统计** | `src-tauri/src/services/usage_stats.rs`<br>`src-tauri/src/services/provider/live.rs` | 支持 9 种 reasoning effort 后缀智能剥离，实现多档位思考强度的精准查价与展示 |
| **定价种子与触发器** | `src-tauri/src/database/schema.rs`<br>`src-tauri/src/database/tests.rs` | 补充腾讯混元（hy3/hy4/hy4flash）、DeepSeek、GLM、Claude 最新模型定价种子与短名触发器 |
| **配置与集成支持** | `src-tauri/src/dsh_config.rs`<br>`src-tauri/src/zcode_config.rs`<br>`src-tauri/src/app_config.rs` | 新增 DSH 与 ZCode 的本地配置读取、模型映射与提供商联动 |
| **UI 与交互定制** | `src/components/apps/AppsPage.tsx`<br>`src/components/BrandIcons.tsx`<br>`src/components/settings/AboutSection.tsx` | 新增 DSH 等专属品牌图标与入口；改造关于页面，彻底禁用官方更新器与外部遥测 |
| **合规与版权** | `NOTICE.md` | 明确标注上游版权归属与定制变更范围 |

---

## 📦 补丁包结构

```text
.
├── README.md               # 本文档（技术见解与补丁说明）
├── NOTICE.md               # 上游版权与合规声明
├── patchset/
│   ├── changes.diff        # 针对 v4.0.20 的统一 Git 补丁文件 (~157 KB)
│   ├── manifest.json       # 69 个变更文件的 SHA-256 校验清单
│   └── apply.py            # 一键自动化打补丁与构建脚本
└── src-custom/             # 独立编写的核心定制源码（便于在线查阅）
    ├── session_usage_dsh.rs
    ├── session_usage_agy.rs
    ├── session_usage_zcode.rs
    ├── dsh_config.rs
    └── zcode_config.rs
```

---

## 🚀 补丁使用与编译指南

### 方式 1：使用自动化脚本一键构建（推荐）

确保本地已安装 `git`, `python 3.8+`, `rust` 和 `pnpm`，然后执行：

```bash
# 运行一键构建脚本（会自动拉取 v4.0.20 源码、校验并应用补丁、执行编译）
python patchset/apply.py
```

### 方式 2：手动应用补丁

```bash
# 1. 克隆官方原版仓库并切换至基线 tag v4.0.20
git clone https://github.com/farion1231/cc-switch.git cc-switch-build
cd cc-switch-build
git checkout v4.0.20

# 2. 应用本仓库的增量补丁
git apply /path/to/patchset/changes.diff

# 3. 安装依赖并构建桌面客户端
pnpm install
pnpm tauri build --no-bundle
```

编译产物将生成在 `src-tauri/target/release/cc-switch.exe`。

---

## 📄 开源许可证

本项目定制部分遵循 [MIT License](./LICENSE)。原始项目 CC Switch 版权归 Jason Young 所有。
