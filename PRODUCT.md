# Product

<!-- impeccable:product-schema 1 -->

## Platform

native desktop (Windows first; Linux and macOS supported)

## Users

专业摄影师。典型场景：一次拍摄或多天拍摄结束后，把存储卡插到电脑上，将卡上成百上千的照片与视频批量迁移到工作盘/素材库，并按拍摄日期分好目录。过程中必须保留调色软件（Darktable / Lightroom）生成的 `.xmp` 侧边栏数据，不能丢文件、不能覆盖同名文件。

## Product Purpose

把「插卡 → 分日期入库 → 保留调色数据」这条重复、易错的手工流程，收敛成一次点击即可完成的操作。成功意味着：迁移快速且可核对、源卡上的文件被正确移走（或按选择保留）、xmp 与照片始终在一起、同名不覆盖。

## Positioning

三个能力同时成立，缺一不可：

1. 按 RAW 特征签名自动识别存储卡，不依赖固定盘符，并排除电脑内置硬盘；
2. 迁移照片时同步并补迁 `.xmp` 伴生文件；
3. 纯 Rust 编译成单个可执行文件、零运行时依赖，附带图形界面。

## Operating Context

- 用户通常在拍摄后集中处理，面对大量文件，关注速度与「不丢、不错、可回退核对」。
- 素材来源是相机存储卡（Windows 盘符不固定；Linux 挂载在 `/media`、`/mnt`、`/run/media`；macOS 在 `/Volumes`）。
- 目标目录默认是运行目录下的 `RAW`，归档结构为 `年/月_日`（如 `2026/09_24`）。
- 常见伴生文件来自 Darktable / Lightroom。

## Capabilities and Constraints

- 归档目录：`年/月_日`。
- 迁移方式二选一：移动（删除源文件）或复制（保留源文件）。
- 时间筛选：按年/月/日勾选要迁移的范围，预览实时同步。
- 重名不覆盖：目标已有同名文件时追加 `_1`、`_2`。
- 跨盘移动：`rename` 遇 CrossesDevices 时回退为复制+删除；只读文件先清除只读属性。
- 处理格式：`.jpg` `.jpeg` `.nef` `.cr3` `.arw` `.dng` `.mp4` `.mov`；探测用 RAW 签名另含 `.raf` `.orf` `.rw2` `.pef` `.srw` `.nrw`。
- 界面为中文；需要系统中文字体（不内嵌）。
- 技术约束：Rust 1.94+ stable；eframe/egui（glow/OpenGL 后端）；release 开启体积优化。

## Brand Commitments

- 名称：CamDrop。
- 界面语言：中文。
- 用户明确的视觉约束：浅色背景，湖蓝色强调色；不要深色背景。（仅记录，不展开设计。）
- 应用图标：`assets/icon.svg`（湖蓝圆角底 + 白色向下箭头落入托盘），成品 `assets/icon.png` / `.ico` 由 `tools/gen_icon.py` 一次性生成。

## Evidence on Hand

- `README.md`、`README_EN.md`。
- `assets/` 图标源与成品。
- `src/`、`tests/`（7 个集成测试）。
- 没有真实用户数据、案例、评测或客户背书；后续工作不得虚构。

## Product Principles

1. 任务优先：界面服务于「快速、可靠地迁移」，克制表达，工具隐入流程。
2. 不丢数据：先预览再执行，同名不覆盖，xmp 必须与照片同行。
3. 零依赖、可离线：单文件可执行，不依赖 Python 或其它运行时。
4. Windows 体验一等公民，同时保证 Linux/macOS 可用。

## Accessibility & Inclusion

暂无已确认的特定无障碍要求；界面为中文，需系统中文字体。
