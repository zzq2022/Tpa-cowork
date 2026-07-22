# TPA CoWork 工作区资料

本目录是 **长期维护项目** `tpa-cowork` 的本地笔记与迁移资产（默认不进上游发布叙事）。

| 路径 | 说明 |
|---|---|
| [tpa-satellite-layer/](./tpa-satellite-layer/) | 卫星层架构设计（方案 1） |
| [tpa-cowork-customization-spec.md](./tpa-cowork-customization-spec.md) | 长期需求真相源 |
| [upstream-version-migration-template.md](./upstream-version-migration-template.md) | 每次升级空白模板 |
| [migration/](./migration/) | 各版本迁移实例 |
| [BOOTSTRAP.md](./BOOTSTRAP.md) | **从本工作区开始的操作手册** |

## 仓库身份

| 项 | 值 |
|---|---|
| 目录 | `D:\Pyprojects\tpa-cowork` |
| 分支 | `migrate/tpa-v0.22` |
| 上游基线 | Hope Agent `v0.22.0` (`212be00a`) |
| baseline 标签 | `baseline/v0.22.0` |
| 官方远程 | `upstream` → https://github.com/shiwenwen/hope-agent |
| 供体（TPA v0.21 实现） | `D:\Pyprojects\hope-agent-021-migration` @ `migrate/tpa-v0.21` |
| 旧 v0.20 产品树 | `D:\Pyprojects\hope-agent-020`（仅历史参考） |

**维护模式：** Hope 宿主尽量贴 upstream；TPA 能力以卫星模块接入（见 tpa-satellite-layer）。
