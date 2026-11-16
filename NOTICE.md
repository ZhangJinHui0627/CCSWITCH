# NOTICE / 版权与修改声明

本项目为 [farion1231/cc-switch](https://github.com/farion1231/cc-switch) 的非官方定制修改版。

## 版权说明 (Copyright)

- **原始软件版权 (Original Work)**：
  Copyright (c) 2025 Jason Young.
  Licensed under the MIT License (详见根目录 [LICENSE](./LICENSE))。

- **定制修改部分 (Modifications)**：
  Copyright (c) 2026 ZhangJinHui0627.
  Licensed under the MIT License.

## 免责声明 (Disclaimer)

本仓库及由其生成的构建产物均为独立维护的非官方分支，与原作者 Jason Young 及官方项目 CC Switch 团队无关联、未获其背书或赞助。

## 主要定制内容 (Modifications Summary)

基于上游基线版本 `v4.0.20` 进行的定制包括：

1. **会话同步扩展 (Session Sync)**：
   - 支持 Antigravity (AGY) 会话同步与冷却去重；
   - 支持 DeepSeek / DSH（多帧 zstd 解析、模型映射）会话用量同步；
   - 支持 ZCode 会话同步与独立配置。
2. **定价与用量统计 (Usage & Pricing Seeds)**：
   - 增加 DeepSeek、混元、Gemini 系列模型定价种子；
   - 增强请求日志（RequestLogTable）与统计面板。
3. **供应商与代理路由 (Providers & Proxy)**：
   - 定制代理提供商与 Codex 等代理端点路由。
4. **图标与界面定制 (Brand Icons & UI)**：
   - 新增定制品牌图标与应用 Glyph。
5. **更新机制改造 (Updater Modification)**：
   - 彻底禁用自动更新检测，断开官方更新源连接，防止覆盖定制特性。
