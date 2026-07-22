# TPA CoWork 个性化需求规范

> 文档类型：长期产品需求真相源  
> 适用范围：基于 Hope Agent 上游版本维护的 TPA CoWork 定制版  
> 原则：本文记录“必须保留什么”，不绑定某个上游版本的文件、函数或提交。

## 1. 维护目标

在尽量减少长期 fork 补丁的前提下，持续保留 TPA CoWork 的品牌、SkillHub、数据兼容、运行环境和企业分发能力。上游升级时优先复用上游实现或配置能力；只有需求缺失时才移植或重写本地代码。

## 2. 优先级定义

- **P0 必须**：缺失会导致产品身份、用户数据、核心业务或安装运行不可用。
- **P1 重要**：影响企业使用体验、运维策略或主要交互，但可短期降级。
- **P2 优化**：性能、稳定性和视觉体验；若上游已解决则不保留本地补丁。

## 3. P0 核心需求

### BRAND-001：TPA CoWork 产品身份

- 产品显示名统一为 `TPA CoWork`。
- Windows/macOS 安装信息、窗口、菜单、托盘、CLI、Server banner 和浏览器扩展保持一致。
- 图标使用 TPA CoWork 品牌资源。
- 旧品牌只允许出现在数据迁移、历史兼容或必要的第三方说明中。
- **验收**：安装包、应用、CLI、扩展中不存在面向用户的错误品牌残留。

### DATA-001：独立数据根目录与升级兼容

- 主数据目录为 `~/.tpa-cowork/`。
- 能从历史 `~/.hope-agent/` 安全迁移数据。
- 迁移必须幂等、可中断重试，并且不得覆盖已存在的新目录数据。
- 会话、配置、凭据、技能、知识空间、设计空间、附件和日志必须保持可访问。
- **验收**：全新安装、仅旧目录、双目录并存三种场景均符合预期。

### SKILLHUB-001：AgentWork 兼容 SkillHub

- 支持云会话登录、刷新和退出。
- 支持公开技能搜索、详情查看和下载。
- 支持“我的技能”、云端刷新和提交审核。
- Tauri 与 HTTP/Server 模式能力一致。
- SkillHub 服务地址应集中配置，避免散落硬编码。
- **验收**：桌面与 HTTP 模式均可完成完整 SkillHub 流程，服务异常不会导致应用崩溃。

### RUNTIME-001：内置 agent-venv

- Windows 安装包包含可用的 agent Python 环境。
- `agent-venv.zip` 是打包资源真相源，运行时按明确规则定位和解压。
- 首次解压、版本刷新、损坏恢复和路径含非 ASCII 字符时均可用。
- 安装器与运行时不得产生重复解压竞态。
- **验收**：未预装 Python 的目标机器也能正常使用依赖该环境的能力。

### BROWSER-001：TPA 浏览器扩展与 Native Host

- Native Host 标识统一为 `com.tpacowork.chrome`。
- 扩展、manifest、Rust 常量、安装路径和诊断逻辑保持一致。
- Windows AppData 和安全上下文处理正确。
- **验收**：扩展能够发现、连接和诊断 TPA native host。

### RUNTIME-002：Windows 主线程 16MB 栈预留配置

- 在 `.cargo/config.toml` 中明确配置 Windows MSVC 目标 (`x86_64-pc-windows-msvc` / `aarch64-pc-windows-msvc` / `i686-pc-windows-msvc`) 的 `rustflags = ["-C", "link-args=/STACK:16777216"]`。
- 防止由于 Tauri 注册 400+ 个接口命令在 Windows 默认 1MB 栈分配下引发 `thread 'main' has overflowed its stack` 及 `CrashSender.exe` 崩溃。
- **验收**：Windows 平台上桌面应用启动及高频 Command 调度下主线程稳定运行，无 Stack Overflow 崩溃。

## 4. P1 重要需求

### UPDATE-001：桌面更新策略

- 桌面自动更新入口默认关闭，不允许无提示自动安装或重启。
- 是否保留手动检查、下载或安装，由每个目标版本明确决定。
- 优先使用上游配置控制，不删除底层更新能力。
- **验收**：启动和设置页不会触发非预期更新，同时保留明确的人工升级路径。

### ONBOARD-001：精简首次启动

- 默认尽快进入主聊天界面，避免冗长 onboarding。
- 不得跳过 Provider、Agent、数据迁移或安全策略所需的初始化。
- 必须保留设置入口和错误恢复路径。
- **验收**：新用户可直接使用；缺少必要配置时能得到明确引导。

### UI-001：TPA 导航和 SkillHub 入口

- 侧边栏包含 SkillHub、我的技能及必要的设置入口。
- 云登录入口与 SkillHub 会话状态一致。
- UI 改动遵守新版公共组件、焦点、表单和 i18n 契约。
- **验收**：导航清晰，主要入口可达，不破坏新版工作区功能。

### UI-002：按钮视觉增强与微交互动效规范

- **视觉与微动效规范**：
  - 基础动效：通用按钮统一增加 `active:scale-[0.98]` 按压微缩放反馈、`transition-all duration-200` 悬停过渡和平滑阴影。
  - 色彩与质感扩展：在基础组件库中扩展高质感变体，包括品牌渐变（`variant: "gradient"`）、柔和发光（`variant: "glow"`）与精致边框（`variant: "soft"`）。
- **低迁移改造成本约束（红线）**：
  - 动效与色彩样式**必须严格集中收敛**在公共 UI 组件 `src/components/ui/button.tsx` 的 CVA (Class Variance Authority) 变体与 `src/index.css` 的全局设计 Token 中。
  - **严禁**在散落的业务页面组件（如 `ChatWindow.tsx`, `SettingsView.tsx`）中手写分散的硬编码动效类名。
- **验收**：全站所有调用 `<Button>` 的组件均能无缝自动获得动效与色彩质感；后续同步上游升级代码时，业务页面零冲突，迁移改造成本接近于零。

### SUPPORT-001：企业支持入口

- About 页支持企业内网链接。
- 未配置 URL 时不显示空链接按钮。
- **验收**：配置后可访问，未配置时界面无无效入口。

### PACKAGE-001：本地双平台打包与架构适配

- 明确支持 **Windows 64位 (`x86_64-pc-windows-msvc`)** 与 **麒麟 ARM 桌面 (`aarch64-unknown-linux-gnu`)** 两类平台目标的独立打包。
- 打包辅助脚本 `scripts/pack-local.mjs` 支持解析 `--target <target-triple>` 参数并自动将平台变量传播至 `ha-browser-host` 与 `hope-agent-eval` Sidecar。
- 根据目标平台自适应生成安装包格式：Windows 目标输出 NSIS `.exe`；Linux/麒麟目标输出 `.deb` 与 `.AppImage`。
- **验收**：在对应编译环境下通过 `pnpm pack:local:bundle -- --target <triple>` 能准确打出完整可安装的软件包。

### DEV-001：Dev 开发模式无锁就绪配置

- `src-tauri/tauri.conf.json` 的 `beforeDevCommand` 配置保持为 `"pnpm dev"`。
- 避免在前置任务中串行链式执行 `cargo build`（如 Sidecar）导致与 Tauri 主进程 `cargo run` 并发争抢 `target/.cargo-lock` 构建锁而发生死锁。
- **验收**：执行 `pnpm tauri dev` 时，Vite 开发服务器在 `http://localhost:1420` 秒级启动就绪，开发过程顺畅无阻。

## 5. P2 可选择优化

### PERF-001：启动性能与前端稳定性

- 仅在新版仍存在对应问题时保留启动渲染、分包、预加载和空值安全补丁。
- 性能改动必须有实际启动或长会话场景验证。
- **验收**：无启动白屏或关键空值崩溃，且不退化新版性能。

### KNOWLEDGE-001：知识空间笔记重命名体验

- 支持安全、确定性的笔记重命名与路径更新。
- 正确处理扩展名、嵌套路径、重名和非法输入。
- **验收**：重命名成功后文件和 UI 状态一致，失败时不产生半成品。

## 6. 长期架构约束

- 业务逻辑优先放在 `ha-core`，桌面与 Server 只做薄适配。
- 新增前端命令必须同时具备 Tauri 和 HTTP 实现。
- 用户可配置项优先配置化，并同步 GUI 与设置技能能力。
- 数据目录和文件路径必须走集中路径入口，不在业务代码散落品牌路径字面量。
- 品牌替换应使用精确白名单，不全仓库无差别替换。
- 不用旧版 `App.tsx`、`IconSidebar.tsx`、`paths.rs` 整文件覆盖新版。
- 上游已有等价功能时删除本地补丁，避免永久维护重复实现。
- 每个主题应可独立提交、验证、回滚和迁移。

## 7. 每次升级的决策规则

对每条需求依次判断：

1. **上游已实现**：直接采用上游实现，仅验证需求是否满足。
2. **新版可配置**：改用配置，不再维护源码补丁。
3. **上游部分实现**：只补缺口，不复制完整旧模块。
4. **上游未实现**：根据新版架构重新移植。
5. **需求不再需要**：明确标记废弃，不默认继承历史行为。

## 8. 发布完成标准

- P0 需求全部达到“已验收”。
- Tauri 与 HTTP 模式的 SkillHub 能力一致。
- 新旧数据目录场景均经过烟测。
- agent-venv 在干净 Windows 环境可用。
- 浏览器扩展能连接正确的 Native Host。
- 安装包、CLI、窗口和扩展品牌一致。
- 不存在以旧版高风险文件整体覆盖新版的情况。
- 迁移提交按主题拆分，并记录暂缓和废弃项。

## 9. 关联资料

- v0.20 实现盘点：`hope-agent-v0.20-local-migration-checklist.md`
- 每次升级模板：`upstream-version-migration-template.md`
