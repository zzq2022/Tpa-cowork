# TPA CoWork 新项目启动手册（基于 Hope v0.22.0）

> 创建日期：2026-07-22  
> 工作区：`D:\Pyprojects\tpa-cowork`  
> 分支：`migrate/tpa-v0.22`  
> 上游：`v0.22.0` / `baseline/v0.22.0`

---

## 1. 已经做完的事

- [x] `git fetch upstream --tags`
- [x] 标签 `baseline/v0.22.0` → 官方 `v0.22.0`
- [x] worktree：`D:\Pyprojects\tpa-cowork`，分支 `migrate/tpa-v0.22`，**干净上游树**
- [x] 拷入设计与需求文档到 `.scratch/`
- [x] 按主题从 v0.21 供体重植 TPA 能力（见 §4；同结构移植，未卫星化）
- [ ] （推荐）卫星化：SkillHub 等进 `tpa-*` 目录，而不是永久散落

在 IDE 中请 **打开文件夹** `D:\Pyprojects\tpa-cowork` 作为工作区（不要继续在 `hope-agent-021-migration` 里改 v0.22）。

---

## 2. 三个目录怎么用

| 目录 | 角色 | 以后 |
|---|---|---|
| **`tpa-cowork`** | **唯一主工程**：基于 v0.22 维护 TPA | 日常开发、发版、再升级 |
| `hope-agent-021-migration` | TPA@v0.21 **供体**（已实现 SkillHub/路径/venv/品牌） | 只读参考；能力迁完可归档 |
| `hope-agent-020` | v0.20 历史 | 只读；不要再当 merge base |

远程仍指向你的 fork `origin` + 官方 `upstream`。长期可把 `origin` 改成独立仓库名（如 `tpa-cowork`），不阻塞开工。

---

## 3. v0.21 → v0.22 上游变化（与挂载相关）

官方约 **8 个提交 / 106 文件**，比 0.20→0.21 小一个数量级。

### 3.1 与 TPA 挂载点冲突面（好消息）

下列文件在 **v0.21.0 → v0.22.0 官方 diff 中未改**，v0.21 供体上的接线可较直接移植：

- `src/App.tsx`
- `src/components/common/IconSidebar.tsx`
- `crates/ha-core/src/paths.rs`
- `src-tauri/windows/installer-hooks.nsh`

### 3.2 官方改过、接线时必须 merge 的文件

| 文件 | v0.22 变化要点 | TPA 注意 |
|---|---|---|
| `src/lib/transport-http.ts` | 终端相关 COMMAND_MAP | 追加 SkillHub 映射时 **保留** 终端条目 |
| `crates/ha-server/src/lib.rs` | 终端路由 | SkillHub 路由插在合适块，不覆盖终端 |
| `src-tauri/src/lib.rs` | 终端 commands | invoke 注册并存 |
| `crates/ha-core/src/lib.rs` | 可能有 terminal 模块 | `skillhub` / 未来 `tpa_*` 并列注册 |
| `package.json` / `tauri.conf.json` | 版本等到 0.22.0 | 品牌/二进制名在 TPA 阶段再改，先别用旧 package 整文件盖 |
| `AGENTS.md` + 新 `src/AGENTS.md` | 文档瘦身/前端规范迁移 | 读新契约；TPA 笔记放 `.scratch`，勿回灌巨大 AGENTS |

### 3.3 上游新能力（保留，不要盖掉）

- 对话底部 **内嵌终端**（`terminal` 全栈）
- Git / PR 工作台增强
- LSP 语义诊断注入修复
- 评测策略调整等

移植 TPA 时：**禁止**用 v0.21 的 `App.tsx` / `ChatScreen` 整文件覆盖，以免丢掉终端入口。

---

## 4. 推荐实施顺序（在本仓库）

按 [tpa-satellite-layer/design.md](./tpa-satellite-layer/design.md) 与需求矩阵，**从官方树重植**，供体路径：

`D:\Pyprojects\hope-agent-021-migration`（分支 `migrate/tpa-v0.21`）

### 阶段 A — SkillHub（P0）

1. 复制 `crates/ha-core/src/skillhub/`（或直接落到 `crates/tpa-skillhub`，见阶段 0/1 卫星化）
2. 注册 `ha-core` mod；接 `ha-server` 路由 + `src-tauri` commands
3. `transport-http` **追加** 10 命令（保留终端 map）
4. 复制 UI/hooks/types；`App.tsx` / `IconSidebar` **最小 diff**（保留 v0.22 其它视图）
5. i18n **只 merge key**，不覆盖 locale 整文件
6. 验证：`cargo check -p ha-core -p ha-server`、`pnpm typecheck`

### 阶段 B — 数据目录 DATA-001（P0）

- 在 **v0.22 的 `paths.rs`** 上重植 `~/.tpa-cowork` + legacy 迁移（或调用即将抽出的 `tpa-product`）
- 跑 paths 单测；勿整文件盖

### 阶段 C — agent-venv + 安装器（P0）

- 合并 v0.22 安装器钩子 + venv 段（VC++ 逻辑保留）
- 资源声明与 pack-local 适配 0.22 版本号

### 阶段 D — 浏览器 Host / 更新默认 / UI 入口 / 支持链接（P0–P1）

- Native Host `com.tpacowork.chrome`
- 更新默认关（保留能力）
- 侧栏 SkillHub、About 企业链接等

### 阶段 E — 品牌与打包（最后）

- product 身份、图标、二进制名
- 本地 pack 烟测

### 并行可选：卫星层阶段 1

若希望 v0.22 一起扶正结构：SkillHub 直接进 `crates/tpa-skillhub` + `src/tpa/**`，宿主只挂载。  
若赶发版：先按 v0.21 同构目录重植，再开 PR 做卫星化。

---

## 5. 决策矩阵（v0.22 初稿）

复制自需求 spec，升级时改「现状/决策/状态」：

| ID | 优先级 | v0.22 官方现状 | 建议决策 | 状态 |
|---|---:|---|---|---|
| BRAND-001 | P0 | Hope Agent | 最后品牌 | 待迁移 |
| DATA-001 | P0 | `~/.hope-agent` | 重植 TPA 根+迁移 | 待迁移 |
| SKILLHUB-001 | P0 | 不存在 | 从 v0.21 供体移植 | 待迁移 |
| RUNTIME-001 | P0 | 无 agent-venv | 重植+合并安装器 | 待迁移 |
| BROWSER-001 | P0 | `com.hope_agent.chrome` | 改身份，用内嵌扩展机制 | 待迁移 |
| UPDATE-001 | P1 | 需确认默认 | 默认关检查/下载 | 待分析 |
| ONBOARD-001 | P1 | 完整 onboarding | 最小导航精简 | 待分析 |
| UI-001 | P1 | 含终端等新入口 | 最小加 SkillHub，保留终端 | 待迁移 |
| SUPPORT-001 | P1 | About 帮助 | 条件企业链接 | 待迁移 |
| PACKAGE-001 | P1 | 0.22 打包链 | 适配 pack-local | 待迁移 |
| PERF-001 | P2 | 上游已有多轮修复 | 先测再决定 | 暂缓 |
| KNOWLEDGE-001 | P2 | 供体有笔记重命名 | 可独立移植 | 待分析 |

详细填写用：复制 [upstream-version-migration-template.md](./upstream-version-migration-template.md) → `migration/tpa-migration-v0.22.0.md`。

---

## 6. 供体快速索引（v0.21 实现位置）

在 `hope-agent-021-migration` 中主要看这些提交主题（`git log migrate/tpa-v0.21 --not v0.21.0`）：

| 主题 | 说明 |
|---|---|
| skillhub core/api/ui | 最大业务块 |
| paths + legacy migration | DATA-001 |
| agent-venv packaging | RUNTIME-001 |
| browser native host | BROWSER-001 |
| updater default off | UPDATE-001 |
| UI nav / support / note rename | UI / SUPPORT / KNOWLEDGE |
| brand commits | 最后做，或改为 product.toml |

复制文件时优先：

```text
git show migrate/tpa-v0.21:crates/ha-core/src/skillhub/...
# 或
git checkout migrate/tpa-v0.21 -- crates/ha-core/src/skillhub
```

在 **tpa-cowork** 目录执行；冲突时以 v0.22 宿主为准，手工接线。

---

## 7. 验证清单（单点，勿默认全量 pre-push）

- `cargo check -p ha-core`
- `cargo check -p ha-server`
- 接线面大：`cargo check --workspace`（需先问用户若耗时长）
- `pnpm typecheck`
- `node scripts/sync-i18n.mjs --check`
- 手工：SkillHub 登录流、数据迁移三场景、venv、扩展 Host、终端仍可用

---

## 8. 红线（重申）

1. 不从供体整文件覆盖：`App.tsx`、`IconSidebar.tsx`、`paths.rs`、`Cargo.lock`、locale 整包  
2. 不丢 v0.22 终端 / Git PR 工作台  
3. 新 invoke 必须 Tauri + HTTP 双适配  
4. 业务进模块，宿主只挂载  
5. 旧目录只作供体，不在 `hope-agent-021-migration` 上继续堆 v0.22 功能  

---

## 9. 建议的第一周节奏

| 日 | 动作 |
|---|---|
| Day 0 | IDE 打开 `tpa-cowork`；读完本文件 + satellite design |
| Day 1 | SkillHub core + API 编译通过 |
| Day 2 | SkillHub UI + transport + 侧栏/App 最小接线 |
| Day 3 | paths + 迁移 |
| Day 4 | venv + 安装器合并 |
| Day 5 | Host / 更新默认 / 品牌 / 打包烟测 |

---

## 10. 下一步命令备忘

```powershell
# 进入新项目
cd D:\Pyprojects\tpa-cowork

# 确认基线
git log -1 --oneline
# 应显示: 212be00a release: v0.22.0

# 列出供体 TPA 提交
git -C D:\Pyprojects\hope-agent-021-migration log --oneline migrate/tpa-v0.21 --not v0.21.0

# 从供体取 SkillHub 目录示例（在 tpa-cowork 内）
git checkout migrate/tpa-v0.21 -- crates/ha-core/src/skillhub
# 注意：两 worktree 同仓库时可直接 checkout 路径；然后立刻改 mod 注册并 cargo check
```

若 `git checkout migrate/tpa-v0.21 -- <path>` 在 worktree 报错，改用：

```powershell
git --git-common-dir
# 或从供体目录 copy / git show 导出
```
