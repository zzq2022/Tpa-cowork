# TPA CoWork 迁移到 Hope Agent v0.21.0

> 上游：https://github.com/shiwenwen/hope-agent  
> 上游标签：`v0.21.0`  
> 上游 commit：`1bcf44137b02d7c96ae87b2250d09709f51fabf9`  
> 本地基线标签：`baseline/v0.21.0`  
> 迁移分支：`migrate/tpa-v0.21`  
> 独立 worktree：`D:\Pyprojects\hope-agent-021-migration`  
> 旧版代码供体：`D:\Pyprojects\hope-agent-020` 的 `master@110c32c`

## 1. 当前准备状态

- [x] 添加官方远程 `upstream`。
- [x] 获取官方 `v0.20.0`、`v0.21.0` 和 `release/v0.21`。
- [x] 创建 `baseline/v0.21.0` 标签。
- [x] 从官方 v0.21.0 创建独立分支 `migrate/tpa-v0.21`。
- [x] 创建独立 worktree，未改动当前 v0.20 `master`。
- [ ] 在 Kiro 中打开 `D:\Pyprojects\hope-agent-021-migration` 为新工作区后开始实现。
- [ ] 记录官方 v0.21 原始基线的单点检查结果。

## 2. 已确认的版本事实

- 官方 v0.20.0 → v0.21.0：1356 个文件变化，约 `+90210/-14339`。
- 本地定制：239 个文件；与上游 v0.21 变化有 86 个同路径文件。
- 86 个重叠文件中包含大量图标和 12 个 locale；代码/配置冲突集中在约 40 个文件。
- 官方 v0.21 中不存在 SkillHub、My Skills、CloudLogin 或 agent-venv 实现。
- v0.21 将内置技能和 Chrome 扩展编译进主程序，不能继续照搬旧外置扩展打包方案。
- v0.21 新增帮助中心、能力评测、流式消息可靠落盘，并调整发行包资源压缩。
- v0.21 自动更新默认仍为 `checkEnabled=true`、`autoDownload=true`。
- v0.21 Native Host 默认仍为 `com.hope_agent.chrome`。
- v0.21 Windows 安装器新增 VC++ Runtime 安装逻辑，迁移 agent-venv 时必须合并而不能覆盖。

## 3. 重要基线风险

本地 `baseline/v0.20.0` 与官方 `v0.20.0` 并非完全同一棵树：有 53 个文件、约 `+290/-932` 的差异。因此：

- [ ] 不把本地 `baseline/v0.20.0` 当成官方 v0.20→v0.21 的 Git merge base。
- [ ] 不对当前 `master` 执行整体 rebase 到 v0.21。
- [ ] 不一次性 cherry-pick 当前 20 个提交。
- [ ] 只把当前仓库作为需求和代码供体，在官方 v0.21 分支上按主题重植。

## 4. v0.21 需求决策矩阵

| ID | 优先级 | v0.21 现状 | 本次决策 | 状态 |
|---|---:|---|---|---|
| BRAND-001 | P0 | 仍为 Hope Agent | 最后精确迁移品牌与资源 | 大部分完成（图标+二进制已改；提交拆分待做）|
| DATA-001 | P0 | `paths.rs` 新增 48 行 | 基于新版路径入口重写根目录和迁移 | 已完成（工作区未提交）|
| SKILLHUB-001 | P0 | 完全不存在 | 分 Core/API/UI 三阶段移植 | 已完成（已提交 Core/API/UI）|
| RUNTIME-001 | P0 | 不存在 agent-venv；打包链已变化 | 重新设计资源声明和原子解压 | 实现完成（包验收待做）|
| BROWSER-001 | P0 | 扩展改为编译内嵌，Host 仍是旧名 | 采用新版内嵌机制，只改身份与 Windows 修复 | 已完成（工作区未提交）|
| UPDATE-001 | P1 | 自动检查和下载默认开启 | 保留新版更新能力和 UI，仅改默认策略 | 已完成（工作区未提交）|
| ONBOARD-001 | P1 | 保留完整 onboarding | 仅精简桌面导航决策，不删初始化 | 已完成（工作区未提交）|
| UI-001 | P1 | App/Sidebar 有帮助中心新入口 | 保留帮助中心，最小接入 SkillHub/TPA UI | 已完成（侧栏强调色已补）|
| SUPPORT-001 | P1 | About 新增帮助入口 | 在新版 About 上追加内网链接条件显示 | 已完成（工作区未提交）|
| PACKAGE-001 | P1 | 新增 eval sidecar、brotli 内嵌资源 | 最后适配 pack-local/release-fast | 实现完成（真实打包待做）|
| PERF-001 | P2 | 已修侧栏冻结和流式落盘可靠性 | 不迁移重叠旧补丁，其余先测再决定 | 待分析 |
| KNOWLEDGE-001 | P2 | 关键文件未与 v0.21 变化重叠 | 可独立移植纯函数和定向 UI 接线 | 已完成（工作区未提交）|

## 5. 第一阶段：SkillHub Core

目标：先让 `ha-core` 编译，不接前端页面。

- [x] 复制 `crates/ha-core/src/skillhub/`（7 文件）到 v0.21 worktree（`git restore --source=master`，纯新增，未覆盖上游文件）。
- [x] 在新版 `ha-core/src/lib.rs` 注册 `pub mod skillhub;`（插在 `settings_reset` 与 `skills` 之间，保留 v0.21 全部新增模块）。
- [x] 根据 v0.21 skill frontmatter 机制重新接入 `parse_skill_name_fallback`：定向重做而非覆盖——在 v0.21 `frontmatter.rs` 追加 `parse_frontmatter_fallback` + `parse_skill_name_fallback`，并把 `parse_frontmatter` 两处早退（无 `---` / 无闭合 `---`）改为走 fallback；`skills/mod.rs` 加 `pub(crate) use`。v0.21 `ParsedFrontmatter` 字段与 v0.20 一致，安全。行为严格更宽松（原本返回 None=技能被忽略）。
- [ ] 将 SkillHub 地址从前后端散落常量改为集中配置。（当前保留 `types.rs` 的 `resolve_skillhub_server_url`：session→`TPA_SKILLHUB_URL`→`HOPE_SKILLHUB_URL`→默认常量；配置化留待与 GUI 设置一并做）
- [ ] 配置项同步 GUI、`ha-settings` 风险分类和技能文档。
- [ ] 网络请求复核 SSRF 策略；云 session/token 禁止写日志。
- [ ] Tauri/Server async 边界的 session/cache 文件 IO 使用 blocking pool。
- [x] 不复制旧 `Cargo.lock`，由 v0.21 依赖解析生成。

单点验证：

- [x] `cargo check -p ha-core` —— 通过（EXIT=0，无 error）。
- [ ] SkillHub client/session/cache 的离线定向验证

## 6. 第二阶段：SkillHub 双适配器

- [x] 复制并适配 `crates/ha-server/src/routes/skillhub.rs`（纯新增，v0.21 `AppError` 路径一致，无需改）。
- [x] 在新版 `routes/mod.rs`（加 `pub mod skillhub;`）和 `lib.rs` router 中注册 10 个端点（插在 skills 块之后、channel 块之前）：`GET /cloud/session`、`POST /cloud/login`、`POST /cloud/logout`、`POST /cloud/session/refresh`、`GET /my-skills`、`POST /my-skills/refresh-cloud`、`POST /my-skills/submit-review`、`POST /skillhub/public/search`、`POST /skillhub/public/detail`、`POST /skillhub/download`。
- [x] 复制并适配 `src-tauri/src/commands/skillhub.rs`（纯新增，`CmdError::from` 模式与既有命令一致）。
- [x] 在新版 `commands/mod.rs`（加 `pub mod skillhub;`）和 `invoke_handler!`（skills 块之后）注册 10 个命令。
- [ ] 在新版 `transport-http.ts` 添加 10 个 `COMMAND_MAP` 项。（第三阶段前端一并做）
- [ ] 重做三个 request wrapper 的参数归一化。（第三阶段）
- [ ] 更新 v0.21 API reference，保证 Tauri 与 HTTP 请求/响应一致。

单点验证：

- [x] `cargo check -p ha-server` —— 通过（EXIT=0，无 error）。
- [x] `cargo check -p hope-agent`（Tauri 壳）—— 通过，仅 4 个既有 warning（misc.rs / menu_labels.rs，与 SkillHub 无关），无 error。
      **前置**：v0.21 全新 worktree 缺 3 个打包资源导致 tauri-build 存在性校验失败（`binaries/hope-agent-eval-*.exe`、`resources/vc_redist.x64.exe`、`resources/browser-host/`）。已放本地占位文件越过校验；三路径均在 `.gitignore` 内，不会提交/打包。正式资源由 `pnpm prepare:eval-sidecar` / `pnpm dev:browser-host` / release 流程生成。
- [ ] `pnpm typecheck`（第三阶段前端后）
- [ ] 对比 10 个命令在 Tauri/HTTP 的 JSON 结构

## 7. 第三阶段：SkillHub UI

- [x] 复制 SkillHub、My Skills、CloudLogin 独立组件、hooks、types 和测试（16 个纯新增文件）。
- [x] 在 v0.21 `transport-http.ts` COMMAND_MAP 添加 10 个映射（Help Center 之后、Skills 之前）。
- [x] 在 `normalizeHttpCommandArgs` 添加 `cloud_login`、`skillhub_search_public`、`skillhub_get_public_detail` 三命令的 `request` 解包。
- [x] 在 v0.21 `App.tsx` 上最小增加：
  - `AppView` 类型加 `"skillhub" | "mySkills"`
  - 懒加载 `SkillHubView`、`MySkillsView`
  - `{view === "skillhub" && ...}` / `{view === "mySkills" && ...}` 渲染块（artifacts 之后、ChatScreen 之前）
  - `IconSidebar` 实例传入 `onOpenSkillHub` / `onOpenMySkills` 回调
- [x] 在 v0.21 `IconSidebar.tsx` 上最小增加：
  - Props 接口加 `"skillhub" | "mySkills"` 视图值 + `onOpenSkillHub?` / `onOpenMySkills?` 可选回调
  - 解构新 props
  - 导入 `Globe`、`FolderDown` 图标
  - 在 Skills 按钮与 Memory 按钮之间插入两个可选入口（仅当 callback 传入时显示）
- [x] 在 v0.21 Profile 面板接入云登录/登出入口（`useCloudSession` + `CloudLoginDialog`，手工植入「Cloud Account」区块，未覆盖 v0.21 面板结构）。SkillHub/MySkills 走顶层视图（对齐 knowledge/design），不进 SETTINGS_SECTION_IDS，故不移植 v0.20 的 types.ts 那部分。
- [x] 仅合并新增 i18n key，不覆盖 locale 文件：`cloud` / `skillhub` / `mySkills` 三命名空间注入全部 12 locale，脚本保留 CRLF + 2 空格缩进、单次追加保证顺序一致；`cloud` 在 v0.20 只翻译了 en/zh/zh-TW，其余 9 语言用英文兜底（对齐 `sync-i18n --apply` 策略）；品牌串中性化（TPA CoWork→Hope Agent），留待品牌阶段统一替换。修正 IconSidebar 误用的 `skillhub.mySkills` → `mySkills.title`。
- [x] SkillHub 服务地址集中配置：单一来源 `types.rs::DEFAULT_SKILLHUB_SERVER_URL` + `resolve_skillhub_server_url`（session→`TPA_SKILLHUB_URL`→`HOPE_SKILLHUB_URL`→默认），前端 `DEFAULT_SKILLHUB_SERVER_URL`（`@/types/skillhub`）+ 登录对话框 `serverUrl` 字段可覆盖。无散落硬编码。
- [ ] 保留 v0.21 设置页默认首分区的新行为（未触碰，天然保留）。

验收 / 单点验证：

- [x] `pnpm typecheck`（`tsc -b`）—— 通过，EXIT=0，无 error。
- [x] `node scripts/sync-i18n.mjs --check` —— 通过，EXIT=0，0 缺失 / 0 顺序漂移，12 locale 均 10557 keys。
- [ ] 手工烟测（需在应用内实测）：未登录搜索、登录/刷新/退出、下载、我的技能、提交审核、服务离线。

### SKILLHUB-001 提交记录

```
feat(skillhub): port core domain to v0.21              (a87b73b8)
feat(skillhub-api): add tauri and http adapters        (9249c1a9)
feat(skillhub-ui): add cloud and skill management surfaces (b7bb284f)
feat(skillhub-ui): add cloud login entry and skillhub i18n (COMMIT4)
```

**SKILLHUB-001 代码侧全部完成**，仅剩应用内手工烟测（需运行 Tauri / HTTP 两种模式实测全流程）。

## 8. 第四阶段：数据目录

v0.21 的 `paths.rs` 和 `app_init.rs` 均已变化，其中 `app_init.rs` 上游新增约 381 行。必须重植而非覆盖。

- [x] 在 v0.21 `root_dir()` 中加入 `TPA_DATA_DIR`，保留 `HA_DATA_DIR` 兼容。
- [x] 默认目录改为 `~/.tpa-cowork`。
- [x] 在任何 ensure、默认 Agent、内嵌资源和数据库初始化前执行旧目录迁移：迁移挂在 `paths::ensure_dirs()` 最前面，覆盖 Desktop / Server / MCP / CLI Auth 的所有早期入口。
- [x] 对跨卷 rename 失败设计 copy+逐文件校验+同卷 staging 原子 promote；失败保留源目录且不暴露半成品目标。
- [x] 明确“双目录并存”和自定义 data dir 的处理策略：目标已存在时不覆盖；`TPA_DATA_DIR` / `HA_DATA_DIR` 显式覆盖时不触发 home 目录迁移。
- [x] 保留 v0.21 新增帮助、评测、内嵌技能、更新 staging 等全部既有 path helper，仅修改根目录解析并新增迁移入口。

验证：`cargo check -p ha-core` 通过；定向执行 `paths::tests` 9 项通过（0 failed），覆盖全新/仅旧目录/双目录/复制 fallback/失败保源。

验收：临时用户目录下验证全新、仅旧目录、双目录并存、迁移中断四种场景。

## 9. 第五阶段：agent-venv 与 Windows 安装器

v0.21 安装器会安装并删除 `vc_redist.x64.exe`，原钩子末尾执行 `RMDir "$INSTDIR\resources"`。agent-venv 必须与该流程合并。

- [x] 复制并更新 `agent-venv-requirements.txt` 与 zip 构建脚本。
- [x] 在 v0.21 Windows bundle resources 中明确加入 `agent-venv.zip`。
- [x] 保留 VC++ Runtime 安装逻辑，并在现有 hook 内增量接入 venv。
- [x] 调整资源清理：VC++ 安装器始终单独删除，venv zip 仅在校验成功后删除。
- [x] 改为临时目录解压、验证 `Scripts/python.exe`、再原子 promote。
- [x] 使用 `.tpa-cowork-venv-complete` sentinel 判断完整性，半解压目录不会被运行时视为可用。
- [x] 安装器与 Rust 首次启动恢复共用 `Scripts/python.exe` + sentinel 完成判据。
- [ ] 最终直接检查 NSIS 包中确实包含 `agent-venv.zip`。

实现验证：`cargo check -p ha-core` 通过；最新测试二进制执行 `paths::tests` 为 10 passed / 0 failed；JSON 配置和 `prepare-zip-venv.mjs` 语法检查通过。NSIS 实包验收待本阶段打包命令执行。

验收：干净 Windows、无外部 Python、路径含空格/CJK、半解压恢复、升级安装。

## 10. 第六阶段：浏览器扩展与 Native Host

- [x] 不恢复 v0.20 的 `prepare-chrome-extension.mjs` 外置打包方案。
- [x] 保留 v0.21 的浏览器扩展内嵌和自动更新机制。
- [x] 把 `DEFAULT_NATIVE_HOST_NAME` 改为 `com.tpacowork.chrome`，并同步 service worker / 示例 manifest。
- [x] 同步诊断与 host 注册的 Windows AppData 路径为 `TpaCoWork`，保留 v0.21 自动注册流程。
- [x] 选择性重植 Windows extended path 清理与 `SECURITY_IMPERSONATION`。
- [x] 保留 v0.21 浏览器 host 二进制和 updater extra binary 接线，未覆盖新版扩展打包机制。

实现验证：`cargo check -p ha-core`、`cargo check -p ha-browser-host`、扩展脚本/JSON 检查通过；diagnostics 定向测试 10 passed / 0 failed。

验收：扩展 reload 后能注册、连接、诊断和执行基本浏览器动作。

## 11. 第七阶段：产品策略、UI 与品牌

### 自动更新

- [x] Rust `AutoUpdateConfig::default()` 与 Serde 空配置 fallback 改为 `check_enabled=false`、`auto_download=false`。
- [x] 前端 `DEFAULT_AUTO_UPDATE_CONFIG` 同步改为 false。
- [x] 更新对应测试断言，保持 Rust/TS 默认一致。
- [x] 保留 v0.21 About 页更新控件、手动检查和底层 updater 能力，仅改变默认策略。

实现验证：`cargo check -p ha-core`、`pnpm typecheck` 通过；updater config 定向测试 3 passed / 0 failed。

### Onboarding

- [x] 只修改桌面首次导航决策：启动后直接进入聊天。
- [x] 保留 Provider/Agent/安全/数据迁移初始化和 Server/ACP setup；未删除 onboarding 组件或服务端 setup。
- [x] 未配置 Provider 时保留聊天侧设置/配置恢复入口。

### UI 与知识空间

- [ ] 侧栏只迁移明确需要的入口、颜色和布局，不覆盖 v0.21 帮助中心入口。
- [x] About 企业支持入口由 `VITE_TPA_SUPPORT_URL` 控制，未配置时不显示企业链接，并保留 v0.21 帮助入口。
- [x] 知识笔记重命名纯函数和测试已复制并接入 `KnowledgeView`，覆盖 IME Enter、目录保留和扩展名规则。
- [ ] v0.21 已修复侧栏加载动画冻结和流式持久化，不迁移对应旧补丁。
- [ ] 其他启动性能/null safety 补丁先复现问题，再决定是否移植。

### 品牌

- [ ] 最后替换 productName、identifier、二进制、菜单、托盘、Server banner 和图标。
- [ ] 保留 `.hope-agent` 作为历史迁移源路径。
- [ ] 明确上游仓库 URL 和 updater endpoint 是否仍指向官方；不要被全局替换误伤。
- [ ] v0.21 内置帮助和扩展资源中的品牌也必须纳入检查。

## 12. 第八阶段：打包

- [x] 适配 `pack-local.mjs` 和 `release-fast`，保留 compile / bundle / ship 三种本地模式。
- [x] 保留 v0.21 的 `prepare:eval-sidecar`、内嵌资源和 brotli 机制。
- [x] 未复制旧 `.cargo/config.toml` 的机器假设。
- [x] 未复制旧 `Cargo.lock`。
- [x] 输出清单包含主程序、NSIS、browser host、eval sidecar、内嵌扩展、内嵌技能和 agent-venv。

静态验证：`pack-local.mjs` 语法、package scripts 和 Cargo metadata/profile 解析通过；实际 compile/bundle 留待用户准备好 agent-venv 资源后执行。

## 13. 高风险重叠文件

必须手工重做：

- `crates/ha-core/src/app_init.rs`（本地 `+20/-1`；上游 `+381/-16`）
- `crates/ha-core/src/paths.rs`（本地 `+285/-4`；上游 `+48`）
- `crates/ha-core/src/browser/extension/diagnostics.rs`
- `crates/ha-server/src/lib.rs`
- `src-tauri/src/lib.rs`
- `src/lib/transport-http.ts`
- `src/App.tsx`
- `src/components/common/IconSidebar.tsx`
- `src/components/settings/AboutPanel.tsx`
- `src/components/settings/SettingsView.tsx`
- `src-tauri/tauri.conf.json`
- `src-tauri/tauri.windows.conf.json`
- `src-tauri/windows/installer-hooks.nsh`
- `package.json`、各 Cargo.toml、build.rs

禁止整文件覆盖：`App.tsx`、`IconSidebar.tsx`、`paths.rs`、`app_init.rs`、两个 `lib.rs` 和安装器 hook。

## 14. 推荐提交顺序

1. `feat(skillhub): port core domain to v0.21`
2. `feat(skillhub-api): add tauri and http adapters`
3. `feat(skillhub-ui): add cloud and skill management surfaces`
4. `feat(paths): add TPA data root and robust legacy migration`
5. `feat(packaging): bundle and atomically install agent venv`
6. `fix(browser): align embedded extension and TPA native host`
7. `chore(updater): default desktop auto update to off`
8. `feat(ui): apply selected navigation and note rename changes`
9. `chore(brand): apply TPA CoWork identity and assets`
10. `chore(packaging): adapt local fast packaging for v0.21`

## 15. 开始实施前的操作

1. 在 Kiro 中打开或添加文件夹：`D:\Pyprojects\hope-agent-021-migration`。
2. 确认状态：

```text
git status --short --branch
git describe --tags --exact-match HEAD
```

预期分支为 `migrate/tpa-v0.21`，HEAD 为 `v0.21.0`。

3. 先记录上游原始基线检查：

```text
cargo check -p ha-core
cargo check -p ha-server
pnpm typecheck
```

如果新 worktree 没有 `node_modules`，先按项目锁文件安装依赖，再运行 typecheck。不要从 v0.20 复制 `node_modules` 或 `Cargo.lock`。

4. 从 SkillHub Core 开始，不先做品牌替换。

## 16. 当前状态（2026-07-22 续 2）

已完成并提交（分支 `migrate/tpa-v0.21`）：

- SkillHub Core/API/UI/i18n/docs
- 数据目录 `~/.tpa-cowork` + `~/.hope-agent` legacy 迁移
- agent-venv 打包/解压与 NSIS 资源接线
- browser native host `com.tpacowork.chrome`
- 自动更新默认关闭
- 桌面直进聊天 + 知识笔记重命名 + 企业支持入口
- pack-local / release-fast
- 品牌：TPA CoWork 身份、二进制名、扩展、技能文案
- 品牌残留补齐：Info.plist / office 文档元数据 / skill-creator 路径 / chrome 包名

验证：

- `pnpm typecheck` 通过
- `paths::tests` 10 passed
- `pack:local` compile：`tpa-cowork 0.21.0`（release-fast）
- `prepare:eval-sidecar` 完成（eval-sidecar fat LTO ~18m）
- `agent-venv.zip` 63.2 MB 已就绪

进行中：

- `pnpm pack:local:bundle`（NSIS）— release 主程序编译中，完成后产出安装包

可选/延后：

- SkillHub 服务地址 GUI 集中配置（env `TPA_SKILLHUB_URL` / `HOPE_SKILLHUB_URL` 已可用）
- PERF-001 仅在复现问题后移植
