# TPA 宿主挂载点清单

> 升级时 **只允许** 在这些文件上改上游树（外加 product 生成物白名单）。  
> 规则：只接线，不写业务；单文件业务接线建议 ≤20 行，超出下沉 `tpa-*` / `src/tpa/**`。  
> 完整设计见 [design.md](./design.md)。

---

## A. 必须挂载点（P0）

| ID | 文件 | 允许改动 | 禁止 | 目标形态 |
|---|---|---|---|---|
| M01 | `crates/ha-core/src/lib.rs` | `pub mod` / feature 注册 tpa 模块 | 业务实现 | 1～3 行 |
| M02 | `crates/ha-core/src/paths.rs` | `root_dir()` 读 product 默认；`ensure_dirs` 前调用迁移 | 迁移算法、copy/staging | 迁移一行：`tpa_product::migrate_if_needed()` |
| M03 | `crates/ha-server/src/routes/mod.rs` | `pub mod skillhub`（或 tpa 路由模组） | handler 逻辑 | 一行 mod |
| M04 | `crates/ha-server/src/lib.rs`（router） | 挂载 SkillHub / TPA 路由块 | 业务 | 路由表追加 |
| M05 | `src-tauri/src/commands/mod.rs` | `pub mod skillhub` | 命令实现 | 一行 mod |
| M06 | `src-tauri/src/lib.rs`（invoke_handler） | 注册 TPA 命令 | 命令实现 | 注册列表追加 |
| M07 | `src/lib/transport-http.ts` | merge `COMMAND_MAP` + 必要 normalize | 业务类型/UI | `...tpaCommandMap` |
| M08 | `src/App.tsx` | AppView 联合、lazy、视图分支、侧栏回调 | SkillHub 页面逻辑 | ≤15 行 |
| M09 | `src/components/common/IconSidebar.tsx` | 可选 props / 入口位；理想 `extraNavItems` | 强调色整盘、业务状态 | 扩展点优先 |
| M10 | `src-tauri/windows/installer-hooks.nsh` | **append** venv 段，与上游 VC++ 共存 | 整文件覆盖 | `#include` tpa 片段 |
| M11 | 打包清单（`tauri.conf.json` / resources / workspace Cargo） | 声明 agent-venv、二进制名 | 无关资源策略大改 | product 驱动 |

---

## B. 身份类（product 驱动）

| ID | 区域 | 正确做法 | 错误做法 |
|---|---|---|---|
| B01 | `package.json` / `src-tauri/Cargo.toml` / `tauri.conf.json` 产品名 | 由 `tpa/product.toml` 生成或 CI 校验 | 手工全局 replace |
| B02 | Native Host / 扩展 ID | 常量只在 tpa-product，扩展与诊断读一处 | 多文件各写 `com.tpacowork.chrome` |
| B03 | 更新默认策略 | product → AutoUpdate 默认 | 删除更新能力或 UI |
| B04 | 二进制 / banner / CLI 名 | product + 薄壳 bin | 全仓改字符串 |
| B05 | i18n | 只 merge `src/tpa/i18n` 增量 | 覆盖 12 个 locale 整文件 |

---

## C. 伪挂载（升级时不应再大面积出现）

- 大量 `ha-core` 业务文件内显示名/路径注释替换  
- 内置 skills 文案整片 rebrand（应走 product 或仅 tpa 自有 skills）  
- 与身份无关的 CI workflow 改名（能生成则生成，否则极小补丁并白名单）

目标：

```text
合法宿主 diff  ≈ 本清单 A + B 生成物
其余定制       ≈ crates/tpa-* + src/tpa/** + tpa/**
```

---

## D. 二期：把挂载变成注册（可选）

| 扩展点 | 效果 |
|---|---|
| Sidebar `extraNavItems[]` | M09 不再改侧栏内部结构 |
| App `registerView` | M08 不再手写联合类型 |
| paths root provider | M02 零 TPA 分支 |
| Server/Tauri module list | M03–M06 自注册 |

---

## E. 每次升级检查表

- [ ] 仅打开本清单文件做三方合并  
- [ ] 每个挂载点 diff 行数预算（业务接线 ≤20 行）  
- [ ] `git diff baseline --name-only` 清单外路径已解释或撤销  
- [ ] 未整文件覆盖 App / IconSidebar / paths / Cargo.lock / locale  
- [ ] Tauri 与 HTTP 新命令仍对齐  
- [ ] 迁移文档记录：文件 | 上游变化 | 本次处理  

---

## F. 当前 v0.21 对照（迁移中状态，非终态）

| 清单项 | v0.21 大致状态 |
|---|---|
| SkillHub Core | 仍在 `ha-core/src/skillhub`（阶段 1 迁出） |
| SkillHub API/UI | 已独立文件 + App/Sidebar 接线（阶段 1 收口 import） |
| paths / 迁移 | 逻辑仍在 `paths.rs`（阶段 2 变薄） |
| agent-venv | 安装器内嵌段（阶段 3 抽片段） |
| 品牌 | 多文件已改（阶段 2/4 收敛 product） |
