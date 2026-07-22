# TPA CoWork 迁移到 Hope Agent v0.22.0

> 上游：https://github.com/shiwenwen/hope-agent  
> 上游标签：`v0.22.0`  
> 上游 commit：`212be00ae83e2e8bce0a318ac7c65f8bec8e51dd`  
> 本地基线标签：`baseline/v0.22.0`  
> 迁移分支：`migrate/tpa-v0.22`  
> 独立工作区：`D:\Pyprojects\tpa-cowork`  
> TPA 供体：`D:\Pyprojects\hope-agent-021-migration` @ `migrate/tpa-v0.21`（基于官方 v0.21 + TPA 主题提交）

## 1. 准备状态

- [x] 添加/使用官方远程 `upstream`
- [x] 获取官方 `v0.22.0`
- [x] 创建 `baseline/v0.22.0`
- [x] 从官方 v0.22.0 创建分支 `migrate/tpa-v0.22` + 目录 `tpa-cowork`
- [x] 拷入卫星层设计、需求 spec、升级模板、v0.21 迁移记录
- [x] IDE 以 `tpa-cowork` 为工作区
- [x] 开始阶段 A SkillHub 移植

## 2. 版本事实

- 官方 v0.21.0 → v0.22.0：约 8 commits，106 files，`+4485/-2173`
- 主要能力：内嵌终端、Git/PR 闭环、LSP 诊断注入、AGENTS 瘦身、`src/AGENTS.md`、评测策略等
- **未改**（利于重植）：`App.tsx`、`IconSidebar.tsx`、`paths.rs`、`installer-hooks.nsh`
- **已改**（须 merge）：`transport-http.ts`、`ha-server`/`src-tauri` lib、`package.json`、`tauri.conf.json`、`ha-core/lib.rs` 等

## 3. 需求决策矩阵

| ID | 优先级 | v0.22 现状 | 本次决策 | 状态 |
|---|---:|---|---|---|
| BRAND-001 | P0 | Hope Agent | 精确白名单品牌（包名/窗口/图标/i18n/UA/提示词） | 已落地 |
| DATA-001 | P0 | 默认 hope-agent 路径 | 重植 TPA 根+幂等迁移 | 已落地 |
| SKILLHUB-001 | P0 | 不存在 | 从 v0.21 供体同结构移植（ha-core/skillhub + 双 transport） | 已落地 |
| RUNTIME-001 | P0 | 无 agent-venv | paths + installer-hooks + prepare:zip-venv + windows resource | 已落地 |
| BROWSER-001 | P0 | 旧 Host 名 | `com.tpacowork.chrome` + discovery `.tpa-cowork` + clean_windows_path | 已落地 |
| UPDATE-001 | P1 | 默认开可能 | `check_enabled`/`auto_download` 默认 false；About 不开自动检查 | 已落地 |
| ONBOARD-001 | P1 | 完整 onboarding | 保留上游 onboarding（未做「永远进聊天」） | 暂缓 |
| UI-001 | P1 | 有终端等新 UI | 最小 SkillHub / 我的技能入口 + 个人资料云登录 | 已落地 |
| SUPPORT-001 | P1 | About/帮助 | `VITE_TPA_SUPPORT_URL` → About 企业入口 | 已落地 |
| PACKAGE-001 | P1 | 0.22 打包 | pack-local / prepare-zip-venv 脚本 + package scripts | 已落地（未实机打包烟测） |
| PERF-001 | P2 | 上游持续修 | 先测 | 暂缓 |
| KNOWLEDGE-001 | P2 | 供体有实现 | 可独立移植 | 暂缓 |

## 4. 实施顺序

见 [../BOOTSTRAP.md](../BOOTSTRAP.md) §4。

## 5. 高风险文件

| 文件 | v0.22 变化 | 本次处理 | 复核 |
|---|---|---|---|
| `src/App.tsx` | 官方 0.21→0.22 未改；有终端相关兄弟组件 | 最小加 SkillHub 视图，勿盖 ChatScreen |  |
| `src/components/common/IconSidebar.tsx` | 官方未改 | 最小入口 |  |
| `crates/ha-core/src/paths.rs` | 官方未改 | 重植 TPA root/迁移 |  |
| `crates/ha-server/src/lib.rs` | 有终端路由 | merge 注册 SkillHub |  |
| `src-tauri/src/lib.rs` | 有终端命令 | merge 注册 |  |
| `src/lib/transport-http.ts` | 有终端 map | 追加 SkillHub map |  |
| `src-tauri/windows/installer-hooks.nsh` | 官方未改于 0.21→0.22 | append venv，保留 VC++ |  |
| `package.json` / `tauri.conf.json` | 版本 0.22 | 勿用旧文件覆盖 |  |

## 6. 提交计划（主题）

- [x] SkillHub Core
- [x] SkillHub API（Tauri+HTTP）
- [x] SkillHub UI + i18n 增量
- [x] 数据目录与迁移
- [x] agent-venv / 安装器
- [x] 浏览器 Host
- [x] 更新/support（onboarding 暂缓）
- [ ] UI 杂项 / knowledge rename
- [x] 品牌
- [x] 本地打包脚本（实机 NSIS 烟测待做）

## 7. 迁移结论（完成后填）

- 最终版本：基于 `v0.22.0` 的 `migrate/tpa-v0.22` 工作树；产品身份 TPA CoWork
- 未完成：ONBOARD-001 精简首次启动；KNOWLEDGE-001 笔记重命名；PERF-001；卫星层 `crates/tpa-*` 抽离；完整安装包烟测（agent-venv 实机 / 扩展连接）
- 已知风险：未整文件覆盖 App/Sidebar/paths；部分代码注释与文档仍可能出现 Hope Agent；Cargo package 已改为 `tpa-cowork`（binary 名随之变化）；上游 release 更新 endpoint 仍指向 hope-agent（企业可关自动更新）
- 回滚：按主题 git 还原；数据目录迁移幂等，新目录已存在则不覆盖
- 下一版本：卫星化 SkillHub；onboarding 策略确认；`pnpm pack:local:bundle` 烟测
