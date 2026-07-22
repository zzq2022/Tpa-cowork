# TPA CoWork 上游版本迁移模板

> 使用方法：每次 Hope Agent 发布新版本时复制本文件，命名为 `tpa-migration-v<版本>.md`。  
> 本模板记录“本次版本如何迁移”；长期需求以 `tpa-cowork-customization-spec.md` 为准。

## 1. 版本信息

- 上游仓库：
- 上游版本/tag：
- 上游基线 commit：
- 迁移分支：`migrate/tpa-v________`
- 基线标签：`baseline/v________`
- TPA 目标版本：
- 开始日期：
- 完成日期：
- 负责人：

## 2. 状态定义

- `待分析`：尚未检查新版。
- `上游已实现`：新版已有等价能力，不移植旧代码。
- `改用配置`：新版可通过配置满足需求。
- `待迁移`：仍需根据新版架构实现。
- `已迁移`：代码完成，尚待全部验收。
- `已验收`：实现和验收标准均通过。
- `暂缓`：非关键需求，本版本不处理。
- `已废弃`：产品明确不再需要。

## 3. 需求决策矩阵

| ID | 优先级 | 新版现状 | 决策 | 实现方式/位置 | 状态 | 验收证据 |
|---|---:|---|---|---|---|---|
| BRAND-001 | P0 |  |  |  | 待分析 |  |
| DATA-001 | P0 |  |  |  | 待分析 |  |
| SKILLHUB-001 | P0 |  |  |  | 待分析 |  |
| RUNTIME-001 | P0 |  |  |  | 待分析 |  |
| BROWSER-001 | P0 |  |  |  | 待分析 |  |
| UPDATE-001 | P1 |  |  |  | 待分析 |  |
| ONBOARD-001 | P1 |  |  |  | 待分析 |  |
| UI-001 | P1 |  |  |  | 待分析 |  |
| SUPPORT-001 | P1 |  |  |  | 待分析 |  |
| PACKAGE-001 | P1 |  |  |  | 待分析 |  |
| PERF-001 | P2 |  |  |  | 待分析 |  |
| KNOWLEDGE-001 | P2 |  |  |  | 待分析 |  |

## 4. 上游变化调查

- [ ] 阅读上游 release notes、CHANGELOG 和迁移说明。
- [ ] 检查 `ha-core`、Tauri、HTTP Transport、配置和路径系统是否重构。
- [ ] 检查本地修复是否已经由上游解决。
- [ ] 检查依赖、Rust toolchain、Node/pnpm 和 Tauri 版本变化。
- [ ] 检查数据库 schema、数据路径和一次性迁移变化。
- [ ] 记录不兼容变化及对应需求 ID。

## 5. 迁移实施顺序

### 阶段 A：核心业务

- [ ] 移植 SkillHub 核心领域模块。
- [ ] 接入 HTTP 路由和 Tauri commands。
- [ ] 接入前端 Transport、页面和状态管理。
- [ ] 验证 Tauri/HTTP 能力对齐。

### 阶段 B：数据和运行环境

- [ ] 实现 `.tpa-cowork` 数据根目录。
- [ ] 实现旧数据幂等迁移。
- [ ] 适配 agent-venv 打包、定位和解压。
- [ ] 验证全新安装和升级安装。

### 阶段 C：浏览器和产品策略

- [ ] 统一 Native Host 和扩展标识。
- [ ] 应用更新策略。
- [ ] 应用首次启动/onboarding 策略。

### 阶段 D：UI 与品牌

- [ ] 移植需要保留的侧边栏和知识空间改动。
- [ ] 补齐新增 i18n key，不覆盖新版 locale 文件。
- [ ] 最后应用品牌文案、包元数据和资源。

### 阶段 E：打包

- [ ] 适配本地快速打包脚本。
- [ ] 生成目标平台安装包。
- [ ] 执行安装/升级/卸载烟测。

## 6. 高风险文件处理记录

以下文件禁止从旧版整文件覆盖，必须记录手工接线方式：

| 文件/模块 | 新版变化 | 本次处理 | 复核人 |
|---|---|---|---|
| `src/App.tsx` |  |  |  |
| `src/components/common/IconSidebar.tsx` |  |  |  |
| `crates/ha-core/src/paths.rs` |  |  |  |
| `crates/ha-server/src/lib.rs` |  |  |  |
| `src-tauri/src/lib.rs` |  |  |  |
| `src/lib/transport-http.ts` |  |  |  |
| `src-tauri/windows/installer-hooks.nsh` |  |  |  |

## 7. 提交计划

- [ ] SkillHub Core/API 独立提交。
- [ ] SkillHub UI 独立提交。
- [ ] 数据目录与迁移独立提交。
- [ ] agent-venv/安装器独立提交。
- [ ] 浏览器扩展独立提交。
- [ ] 更新/onboarding 策略独立提交。
- [ ] UI 改动独立提交。
- [ ] 品牌改动独立提交。
- [ ] 本地打包独立提交。

## 8. 单点验证

- [ ] Rust Core：`cargo check -p ha-core`
- [ ] HTTP Server：`cargo check -p ha-server`
- [ ] 跨 crate/安装接线较大时：`cargo check --workspace`
- [ ] TypeScript：`pnpm typecheck`
- [ ] i18n：`node scripts/sync-i18n.mjs --check`
- [ ] 本地安装包：`pnpm pack:local` 或新版等价命令
- [ ] SkillHub Tauri 流程烟测
- [ ] SkillHub HTTP 流程烟测
- [ ] 旧数据目录迁移烟测
- [ ] agent-venv 首次解压烟测
- [ ] Chrome 扩展/Native Host 连接烟测

## 9. 暂缓、废弃与已知问题

| 需求/问题 | 决策 | 原因 | 后续版本 |
|---|---|---|---|
|  |  |  |  |

## 10. 发布前验收

- [ ] 所有 P0 项均为“已验收”。
- [ ] P1 暂缓项已明确记录，不存在无意遗漏。
- [ ] 没有从旧版覆盖 `App.tsx`、`IconSidebar.tsx` 或 `paths.rs`。
- [ ] 没有用旧版 `Cargo.lock` 覆盖新版。
- [ ] locale 仅合并所需 key。
- [ ] Tauri 和 HTTP 新命令保持对齐。
- [ ] 安装、升级、卸载和数据保留行为符合预期。
- [ ] 品牌残留搜索结果已人工分类。
- [ ] 最终提交按主题拆分。

## 11. 迁移结论

- 最终版本：
- 未完成项目：
- 已知风险：
- 回滚方式：
- 下一版本建议：
