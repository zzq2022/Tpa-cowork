# TPA CoWork 卫星层设计（方案 1）

> 文档类型：架构设计（已评审方向）  
> 日期：2026-07-22  
> 状态：已确认采用 **编译期卫星包 + 固定挂载点**（非运行时插件）  
> 关联：`tpa-cowork-customization-spec.md`（需求）、`upstream-version-migration-template.md`（升级模板）

---

## 0. 背景与目标

### 0.1 问题

TPA CoWork 基于 Hope Agent 上游 fork 维护。每次上游发版，需要把个性化能力融回新版。当前流程（主题提交 + 决策矩阵 + 禁止整文件覆盖）正确，但成本仍高，因为：

1. **独立业务**（SkillHub）已较模块化，可整包搬迁。
2. **产品身份**（品牌、数据根、Native Host、更新默认）散落宿主，升级时全仓 diff。
3. **共享 UI / 路径 / 安装器** 手工接线，冲突面大。

### 0.2 目标

- 定制代码 **90%+** 住在 TPA 树（`crates/tpa-*`、`src/tpa/**`、`tpa/`）。
- 宿主（上游树）只保留 **少量固定挂载点**。
- 上游发版后：对齐 baseline → 接卫星模块 → 重放挂载点 → 按需求矩阵验收。
- **不做** 运行时动态插件（.dll 热加载）——与 Tauri 打包、安全、双 Transport 冲突大。

### 0.3 非目标

- 把 SkillHub 强行 PR 回上游（除非日后改选「部分回馈」策略）。
- 重写 ha-core 业务内核。
- 第一期就做完整 App/Sidebar registry（可列为二期变薄手段）。

---

## 1. 方案选择

### 1.1 三种深度

| 方案 | 机制 | 结论 |
|---|---|---|
| **1. 编译期卫星包 + 挂载点** | 独立 crate/前端包，编译进同一应用 | **采用** |
| 2. 运行时动态插件 | 热加载扩展 | 不采用 |
| 3. 纯 patch 队列 | 主题 patch 叠上游 | 过渡可用，达不到「很快融入」 |

### 1.2 和「插件」的关系

- **工作方式像插件**：能力分模块、可整包搬迁、升级主要「接模块」。
- **机制不是运行时插件**：编译期静态链接进宿主，不能热插拔。
- 准确名称：**TPA 产品层 / 卫星模块集合（satellite layer）**。

### 1.3 分层

```
┌─────────────────────────────────────────────┐
│  宿主 Hope Agent（尽量 = upstream 树）        │
│  ha-core / ha-server / src-tauri / src/     │
│  只含：少量挂载点 + 可选 Extension 注册       │
└──────────────────┬──────────────────────────┘
                   │ 编译期链接 / 注册
┌──────────────────▼──────────────────────────┐
│  TPA 产品层（独立、可整包搬迁）               │
│  ├─ crates/tpa-product   品牌/路径/Host/默认 │
│  ├─ crates/tpa-skillhub  云技能业务          │
│  ├─ crates/tpa-runtime   agent-venv          │
│  ├─ src/tpa/**           UI / hooks / i18n   │
│  └─ tpa/product.toml     单一产品画像        │
└─────────────────────────────────────────────┘
```

### 1.4 原则

1. 定制逻辑不进上游业务文件（不改 chat_engine / knowledge 内核等）。
2. 允许改上游的只有 **挂载点**（见 `mount-points.md`）。
3. 产品身份 **单一来源** `tpa/product.toml`（+ `tpa-product`）。
4. 每个 TPA 能力 = 可独立验收的主题包。
5. 上游已有等价能力 → 删 TPA 实现，避免永久双轨。
6. 新前端命令仍须 **Tauri + HTTP** 双适配。
7. 业务逻辑优先 Rust 核心侧；桌面/Server 只做薄壳。

---

## 2. 目录与模块边界

### 2.1 目标树

```
crates/
  tpa-product/          # 产品画像：名、路径默认、Host ID、更新默认、二进制名
  tpa-skillhub/         # SkillHub 领域（从 ha-core/skillhub 迁出）
  tpa-runtime/          # agent-venv 定位 / 解压 / 版本刷新

src/
  tpa/
    skillhub/           # SkillHub / MySkills / CloudLogin UI
    hooks/
    i18n/               # 仅 TPA 增量 key
    transport.ts        # COMMAND_MAP 片段，供宿主 merge
    product.ts          # 前端产品常量

tpa/
  product.toml          # 单一产品画像（真相源）
  installer/            # agent-venv.nsh 等安装器片段
  patches/              # 可选：仅挂载点的薄 patch
  MIGRATION.md          # 升级标准动作（可链到本设计 §4）

.scratch/tpa-satellite-layer/   # 本设计与笔记（可后迁 docs/plans）
```

### 2.2 职责

| 模块 | 放什么 | 不放什么 |
|---|---|---|
| **tpa-product** | display_name、data_dir、native_host_id、bin_names、更新默认、支持 URL 配置键、legacy 迁移入口 API | 业务流程、UI |
| **tpa-skillhub** | session / client / cache / service、类型与 URL 解析 | 通用 skills 发现、权限引擎 |
| **tpa-runtime** | venv zip 查找、原子解压、损坏恢复 | 安装器 NSIS 全文（只提供片段） |
| **src/tpa/** | 页面、hooks、Transport 封装、增量 i18n | 主聊天 MessageList 等核心 UI |
| **宿主挂载点** | 注册路由/命令、侧栏入口、App 视图、root_dir 一行、安装器 append | 超过 ~20 行的业务实现 |

### 2.3 红线

1. **业务实现禁止写进挂载点文件**——`App.tsx` 只 lazy import + 渲染分支。
2. **身份只从 product 读**——禁止常规流程再做全仓 `Hope Agent` → `TPA CoWork` 字符串手术。
3. **路径经 `paths::root_dir()`**——业务代码不写 `~/.tpa-cowork` 字面量。
4. **SkillHub 不走 ha-core 私有捷径**——只依赖公开 API；缺扩展点时加小而稳的 hook。
5. **i18n 只追加 key**——不覆盖上游 locale 整文件。
6. **不用旧版高风险文件整文件覆盖新版**（`App.tsx`、`IconSidebar.tsx`、`paths.rs`、`Cargo.lock`）。

### 2.4 从现状映射

| 现在 | 目标 |
|---|---|
| `crates/ha-core/src/skillhub/` | `crates/tpa-skillhub/` |
| `src/components/skillhub|skills/My*|cloud/*` | `src/tpa/**`（可先 re-export 过渡） |
| 散落品牌字符串 | `tpa/product.toml` + 生成/校验 |
| `paths.rs` 内嵌完整迁移 | 算法在 tpa-product；paths 调一行 |
| `App.tsx` / `IconSidebar` 大段 TPA | 缩成注册接线 |

**层验收：** `git diff upstream/vX.Y` 中，非 tpa 树改动应 ⊆ 挂载点清单，且每文件 diff 可控。

---

## 3. 宿主挂载点

完整表见 [mount-points.md](./mount-points.md)。摘要：

### 3.1 必须（P0）

1. `ha-core/src/lib.rs` — 模块/feature 注册  
2. `ha-core/src/paths.rs` — root 默认 + 迁移一行调用  
3. `ha-server` routes + router — SkillHub HTTP  
4. `src-tauri` commands + invoke_handler — SkillHub Tauri  
5. `src/lib/transport-http.ts` — COMMAND_MAP merge  
6. `src/App.tsx` — 视图类型 + lazy + 分支 + 侧栏回调  
7. `IconSidebar.tsx` — 可选入口（理想改为 `extraNavItems`）  
8. `installer-hooks.nsh` — append venv 段（与 VC++ 共存）  
9. 打包清单 — agent-venv 资源、二进制名（product 驱动）

### 3.2 身份类（B）

包元数据、Native Host、更新默认、banner/CLI 名、i18n 增量——全部由 product 驱动或校验，禁止散落手改。

### 3.3 伪挂载（应消失）

业务文件内品牌文案、skills 整片 rebrand、与身份无关的 CI 改名——迁出或生成，不作为常规升级步骤。

### 3.4 二期扩展点（可选）

- Sidebar `extraNavItems`
- App `registerView`
- paths root provider
- HTTP/Tauri `RouterModule` 列表自注册

---

## 4. Git 与升级工作流

### 4.1 分支

```
upstream tag vX.Y.Z
    → baseline/vX.Y.Z
    → migrate/tpa-vX.Y
         提交：sync host → reattach mounts → fix tpa modules → product/packaging
```

- 从 **官方 tag** 开分支，旧 TPA 只当 **供体**，不当 merge base。
- 禁止整棵 rebase 旧 master 到新上游。

### 4.2 标准 8 步

1. 读上游 release notes / CHANGELOG  
2. 打 baseline + 开 `migrate/tpa-vX.Y`  
3. 合并/拷入 `crates/tpa-*`、`src/tpa/**`、`tpa/product.toml`  
4. 按挂载点清单接线（禁止清单外改宿主）  
5. 决策矩阵：已实现 / 可配置 / 补缺口 / 重写 / 废弃  
6. 单点验证（cargo check 相关包、pnpm typecheck、i18n check）  
7. P0 烟测（数据迁移、SkillHub 双模式、venv、Host、品牌）  
8. 主题提交 + 填 `tpa-migration-vX.Y.md`

### 4.3 决策规则（与 customization-spec 一致）

1. 上游已实现 → 采用上游，删本地  
2. 新版可配置 → 改 product/配置  
3. 部分实现 → 只补缺口  
4. 未实现 → 在 tpa 模块内按新架构实现  
5. 不再需要 → 标记废弃

### 4.4 成功体感

- 模块大多原样编译；时间主要花在挂载点 + breaking API。  
- 宿主 diff ⊆ 挂载点清单。  
- 不再把「275 文件品牌扫尾」当常规步骤。

---

## 5. 落地路线图

### 阶段 0：定契约（0.5～1 天）

- 本目录文档齐备  
- 挂载点表与升级 8 步可执行  
- 可选：`tpa-check-mounts` 警告模式  

### 阶段 1：SkillHub 卫星化（1～3 天）— 优先

- `crates/tpa-skillhub` + `src/tpa/**`  
- 挂载点 3/4/5/6/7 只改 import  
- 可 re-export 旧路径一个 minor 过渡  
- **不做** 品牌大扫除  

### 阶段 2：product + 路径/默认策略（2～4 天）

- `tpa/product.toml` + `tpa-product`  
- paths 变薄；迁移算法进模块  
- UPDATE / ONBOARD / SUPPORT / Host 默认从 product 来  

### 阶段 3：runtime / 安装器 / 打包（1～3 天）

- `tpa-runtime`  
- 安装器 include/append 片段  
- pack:local 认 product  

### 阶段 4：身份收敛与挂载变薄（持续）

- 品牌生成/校验替代全仓 replace  
- 可选 registry  
- mount-check warn → error  

### 阶段 5：稳定节奏（v0.22 起）

- 每次只跑 §4 八步 + 迁移模板  

### 风险

| 风险 | 缓解 |
|---|---|
| SkillHub 搬迁破坏引用 | re-export 过渡 |
| paths 回归 | 迁走现有 paths 测试 |
| 安装器互撕 | 永不整文件覆盖钩子 |
| 范围膨胀 | 阶段独立提交 |

---

## 6. 与现有 v0.21 迁移的关系

当前 `migrate/tpa-v0.21` 已按主题提交完成 SkillHub / paths / venv / browser / brand 等能力，**功能可用**。

本设计 **不要求推倒 v0.21 重来**，而是：

1. v0.21 收尾保证可发布；  
2. 按阶段 0→1→… 在后续 PR 中把结构扶正；  
3. **v0.22 起** 按卫星层剧本升级，验证「接模块」是否真快。

---

## 7. 文档维护

| 改动 | 更新 |
|---|---|
| 新增/删除挂载点 | `mount-points.md` + 迁移模板高风险表 |
| 新增 tpa crate | 本设计 §2 + customization-spec 若有新需求 ID |
| 某需求改由上游满足 | customization-spec 决策 + 迁移矩阵标「上游已实现」 |
| 设计晋升正式文档 | 可复制到 `docs/plans/` 并在 docs/README 登记（可选） |

---

## 8. 确认记录

| 项 | 结论 |
|---|---|
| 与上游关系 | C：极薄壳 + 模块化（编译期卫星层） |
| 机制 | 非运行时插件 |
| 模块形态 | 多个 tpa-* + src/tpa，不是单一超级插件 |
| 文档位置 | `.scratch/tpa-satellite-layer/`（用户指定，与杂乱草稿隔离） |
