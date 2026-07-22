# TPA 卫星层（方案 1）资料夹

本目录存放 **TPA CoWork 编译期卫星模块** 设计与后续落地笔记，与零散迁移草稿隔离。

| 文件 | 说明 |
|---|---|
| [design.md](./design.md) | 完整设计：架构 / 目录边界 / 挂载点 / 升级工作流 / 落地路线图 |
| [mount-points.md](./mount-points.md) | 宿主挂载点清单（升级时优先打开） |

长期需求真相源仍在：

- [../tpa-cowork-customization-spec.md](../tpa-cowork-customization-spec.md)
- [../upstream-version-migration-template.md](../upstream-version-migration-template.md)
- 当前版本实例：[../tpa-migration-v0.21.0.md](../tpa-migration-v0.21.0.md)

**一句话：** Hope = 上游宿主；TPA = 独立卫星模块集合；升级 = 接模块 + 重放挂载点，而不是全仓重植。

## 当前工程

- 工作区：`D:\Pyprojects\tpa-cowork`（`migrate/tpa-v0.22` @ 官方 `v0.22.0`）
- 启动手册：[../BOOTSTRAP.md](../BOOTSTRAP.md)
- 本版迁移单：[../migration/tpa-migration-v0.22.0.md](../migration/tpa-migration-v0.22.0.md)
- 供体：`D:\Pyprojects\hope-agent-021-migration`（TPA on v0.21，只读）
