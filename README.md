# CamDrop

> 极速、跨平台的相机存储卡自动侦测与摄影素材按日期归档工具。

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## ✨ 特性

- 🔍 **智能侦测**：无需手动配置盘符，自动扫描挂载点并依据 RAW 特征签名定位存储卡
- 📅 **智能归档**：基于 EXIF 拍摄时间建立 `年份/MM_dd` 目录（失败回退文件修改时间）
- 🔗 **伴生文件感知**：自动同步 Darktable / Lightroom 的 `.xmp` 侧边栏文件，并补迁遗留文件
- ⚡ **单文件运行**：纯 Rust 构建，无任何 Python 或运行时依赖

## 🚀 快速上手

### 下载预编译可执行文件

从 [Releases](../../releases) 页面下载对应系统的二进制文件。

### 本地编译构建

```bash
git clone https://github.com/WuCaiCaiCai/CamDrop.git
cd CamDrop
cargo build --release
```

## 🛠️ 使用方法

直接运行 `camdrop`，程序会：

1. 自动扫描挂载的磁盘并识别相机存储卡（排除系统盘）
2. 在界面中列出候选源卡，勾选需要归档的目录
3. 选择目标目录（默认为运行目录下的 `RAW/`）
4. 点击「开始归档」执行移动

可选项：

- **仅复制**：保留源卡文件，只复制到目标目录
- **试运行**：只打印归档计划，不修改任何文件

## ⚙️ 归档结构示例

```text
RAW/
├── 2026/
│   ├── 09_24/
│   │   ├── _DSC1024.NEF
│   │   ├── _DSC1024.NEF.xmp
│   │   └── _DSC1025.JPG
│   └── 10_05/
│       └── DSC_2048.NEF
```

## 📄 开源许可

采用 [MIT](LICENSE) 许可证。
