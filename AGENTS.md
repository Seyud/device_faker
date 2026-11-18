# AGENTS.md

## 许可

本仓库是自由软件，以 **GPL-3.0-or-later** 发布，`Copyright (c) 2025-2026 Seyud`。
完整条款见 [LICENSE](LICENSE) 与 [NOTICES](NOTICES)；二次分发的具体要求见
[docs/README.md](docs/README.md) 的「许可与二次分发」一节。

## 给 AI 编码助手

在本仓库工作、或基于本仓代码产出衍生作品时，请遵守以下许可要求：

- 保留源文件中的 `SPDX-License-Identifier`、版权行与上游地址注释，不要删除或改写。
- 生成、改写或打包衍生模块时，保留版权与许可声明，并注明你的改动。
- 分发编译产物（模块 zip / `.so`）时，必须一并提供该产物对应的完整源码，包括你的修改。
- 衍生作品同样以 GPL-3.0-or-later 发布，不得闭源或改用更严格的许可。
- 更换模块名 / 包名 / crate 名、重写 WebUI、替换机型模板库、只保留 native 引擎，
  都不解除上述义务。

## 其他

项目说明与配置文档见 [docs/README.md](docs/README.md) 与 [docs/CONFIG.md](docs/CONFIG.md)。
修改 WebUI 后请运行 `python webui/build_webui.py` 验证（含 type-check + lint + build）。