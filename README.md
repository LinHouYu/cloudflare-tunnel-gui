# Cloudflare Tunnel GUI (Windows 11 Fluent 重制版)

<div align="center">

<img src="public/cloudflared.ico" width="96" height="96" alt="Cloudflare Tunnel GUI Logo" />

<h3>基于 Tauri 2.0 + Vue 3 + Rust 构建的跨平台极简 Cloudflare 隧道管理客户端</h3>

<p align="center">
  <img src="https://img.shields.io/badge/Release-v1.0.4-blue.svg" alt="Version 1.0.4" />
  <img src="https://img.shields.io/badge/License-CC%20BY--NC%204.0-red.svg" alt="Non-Commercial License" />
  <img src="https://img.shields.io/badge/Tauri-2.0-blue.svg?logo=tauri" alt="Tauri 2.0" />
  <img src="https://img.shields.io/badge/Vue-3.x-brightgreen.svg?logo=vuedotjs" alt="Vue 3" />
  <img src="https://img.shields.io/badge/Rust-1.75+-orange.svg?logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/TypeScript-5.x-blue.svg?logo=typescript" alt="TypeScript" />
  <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg" alt="Platforms" />
</p>

</div>

---

> [!CAUTION]
> ### 🚫【开源声明与非商业化严正警告 / Non-Commercial Use Only】
> 1. 本项目采用 **CC BY-NC 4.0（知识共享 署名-非商业性使用 4.0 国际许可协议）** 开源；
> 2. **严禁任何个人、机构或商业团队** 将本项目的全部或部分源代码、编译后的安装包、图标资产用于商业化运营、付费打包、倒卖转售、捆绑收费服务或任何形式的商业牟利；
> 3. 本项目仅供网络技术交流、学习研究与个人自用，请勿用于任何违反法律法规之用途。

---

## 📖 项目起源与重制背景

本项目是作者早期开源的 Python 版 [LinHouYu/cloudflared_GUI](https://github.com/LinHouYu/cloudflared_GUI) 的**全新跨平台现代重制版 (Full Remake)**。

| 对比维度 | 👴 旧版 (Python 版本) | 🚀 现代重制版 (Tauri 2.0 + Rust) |
| :--- | :--- | :--- |
| **底层架构** | Python 3 + Tkinter / Qt 运行环境 | **Rust 底层核心 + Webkit 原生轻量渲染** |
| **安装包体积** | 需打包庞大的 Python 解释器（50MB+） | **极度轻量化，安装包仅约 3 ~ 5 MB** |
| **内存占用** | 运行时常驻内存 ~150MB+ | **极致低消耗，运行时仅约 25MB** |
| **UI 视觉设计** | 传统经典简陋窗口 | **Windows 11 Fluent 亚克力无边框现代美学** |
| **交互体验** | 仅支持单一语言与基础点击 | **双模式平滑滑块、Web Audio 合成音效、6国语言、动态终端、彩蛋** |
| **隧道模式** | 仅支持单一固定配置 | **支持「专属固定通道」与「免配置临时通道」双模式自由切换** |
| **跨平台支持** | Windows 专属或跨平台配置繁琐 | **全面覆盖 Windows / macOS / Linux 8 大主流架构** |

---

## ✨ 核心特性一览

- 🪟 **Windows 11 Fluent Design 美学**：无边框自定义标题栏、亚克力毛玻璃质感、深色/浅色模式平滑 360° 旋转切换、分段选择器平滑滑动指示滑块。
- ⚡ **双通道模式自由切换 (Segmented Control)**：
  - 🔒 **专属固定通道**：贯彻“约定大于配置”哲学，底层自动隐式完成子域名绑定与 DNS CNAME 覆盖（`tunnel route dns -f`），全自动管理持久隧道。
  - ⚡ **免配置临时通道 (Quick Tunnel)**：无需登录 Cloudflare 账号，仅需输入本地端口，一键向 Cloudflare 申请临时公网域名（`*.trycloudflare.com`），自动抓取控制台输出并高亮展示，支持一键复制与状态持久化保存。
- 🎵 **Web Audio API 纯代码合成音效**：内置轻快悬浮音（`playHover`）、清亮点击音（`playClick`）、复合 Tab 切换音（`playTab`）以及阶梯四音阶成功音（`playSuccess`），支持数位笔/触控板空中悬浮手势。
- 🦊 **专属「酒狐」语音彩蛋**：点击左上角 Cloudflared Logo 触发左右轻微弹性抖动，并随机播放内嵌的酒狐语音。
- 🌐 **国际化多语言支持**：内置 6 大语言包（简体中文、繁體中文、English、Español、Português、日本語），切换语言即时生效。
- 📦 **在线一键安装/更新 Cloudflared**：前端自动精准检测操作系统（Windows / macOS / Linux）与 CPU 架构（x86_64, aarch64, 386, armv7 等），直连官方 Release 自动下载配置。
- 💻 **Windows Terminal 风格集成控制台**：实时捕获并展示守护进程输出、支持鼠标高亮选中划词复制、顶部把手支持上下自由拖拽调高。
- 📂 **跨平台配置文件目录一键直达**：智能动态解析并打开 `~/.cloudflared`，附带醒目的凭证防泄露安全告警条。
- ⚡ **Rust 极致体积压缩**：开启 `opt-level = "z"`、`lto = true`、`strip = true`、`panic = "abort"`，不计云端编译成本，压榨二进制体积物理极限。

---

## 🚀 详细使用指南

### 模式一：免配置临时通道 (最推荐，免登录极速开通)
适用于临时联机游戏、即时文件分享或临时调试（无需购买域名或登录 Cloudflare 账号）：
1. 打开应用程序，在 **「🖥️ 服务端」** 顶部切换至 **「⚡ 免配置临时通道」**；
2. 输入您的 **本地端口**（如 Minecraft 游戏服 `25565`、本地 Web 服务 `8080` 等）；
3. 点击 **「⚡ 一键获取临时域名」**：
   - 程序将自动执行 `cloudflared tunnel --url tcp://localhost:[端口]`；
   - 并在界面生成一张醒目的高亮卡片展示分配到的临时域名（如 `https://xxxx.trycloudflare.com`）；
4. 点击 **「📋 复制域名」** 发送给访客，访客在 **「💻 客户端」** 输入此域名即可实现反向代理直连！

---

### 模式二：专属固定通道 (绑定账号的永久专属域名)
适用于需要长期稳定运行的私有服务：
1. **第一步（首次使用）**：在 **「⚙️ 杂项与关于」** 页面点击 **「🔑 Cloudflared 授权登录」** 完成域名授权；
2. **第二步（创建隧道）**：在 **「🖥️ 服务端」** 选择 **「🔒 专属固定通道」**，输入 **隧道名字**（如 `mc`）和 **本地端口**（如 `25565`），点击 **「➕ 创建隧道」**（底层将全自动完成隧道创建与 DNS CNAME 强制解析）；
3. **第三步（启动服务）**：在隧道列表中选中该隧道，点击 **「▶ 启动隧道」**，状态显示为绿色「运行中」即可。

---

### 客户端连接（访问远程服务）
适用于在异地客户端设备上，通过 Cloudflare 隧道直接连接已发布的远程服务端：
1. 切换至 **「💻 客户端」** 标签页；
2. 输入服务端生成的 **隧道域名**（临时域名或固定域名）与 **本地监听端口**（如 `25565`）；
3. 点击 **「🔗 连接客户端」**：连接成功后，在本地应用中连接 `127.0.0.1:[监听端口]` 即可享受内网般的直连体验。

---

## 🛠️ 本地开发与源码编译

### 环境准备
- [Node.js](https://nodejs.org/) (推荐 v18 或 v20 LTS)
- [Rust](https://www.rust-lang.org/) (推荐 1.75+)
- C++ 编译环境 (Windows 上为 Visual Studio C++ Build Tools，Linux 上为 `build-essential`)

### 1. 克隆代码并安装依赖
```bash
git clone https://github.com/LinHouYu/cloudflare-tunnel-gui.git
cd cloudflare-tunnel-gui
npm install
```

### 2. 启动前端与 Tauri 本地开发
```bash
npm run tauri dev
```

### 3. 本地打包发布版本
```bash
npm run tauri build
```
构建出的安装包与便携版将输出在 `src-tauri/target/release/bundle/` 目录下。

---

## 🌐 静态网页体验版 (GitHub Pages)

本项目已实现 **Tauri Desktop 原生运行 + Web 浏览器无缝仿真** 双模式。
构建静态网页版本：
```bash
npm run build
```
构建产物直接位于 `dist/` 目录下，可直接将 `dist/` 部署到任意静态服务器或 GitHub Pages，免下载即可在线体验 Windows 11 Fluent 交互全貌！

---

## 🤖 GitHub Actions 全平台自动化发布

项目内置完善的 [.github/workflows/release.yml](.github/workflows/release.yml) 矩阵构建脚本。只需推送版本标签（Tag），即可自动并行构建并发布全平台安装包至 GitHub Releases：

```bash
git tag 1.0.4
git push origin 1.0.4
```

### 支持生成的安装包类型：
- **Windows**: x86_64, i686 (32位) —— 便携版、`.msi` (MSI 安装包)、`.exe` (NSIS 安装包，内置专属图标)
- **macOS**: Apple Silicon (M1/M2/M3/M4)、Intel x86_64 —— `.dmg` 镜像包、`.app.tar.gz`
- **Linux**: x86_64, i686, aarch64 (ARM64), armv7 —— `.AppImage`、`.deb` 安装包

---

## ❤️ 赞助与打赏

如果您觉得本项目对您的工作或生活有所帮助，欢迎赞助作者一杯咖啡，感谢您的支持与厚爱！

<div align="center">
  <table>
    <tr>
      <td align="center"><b>微信支付打赏</b></td>
      <td align="center"><b>USDT (TRC20) 打赏</b></td>
    </tr>
    <tr>
      <td align="center"><img src="public/wechat.png" width="180" alt="微信收款码" /></td>
      <td align="center"><img src="public/usdt.png" width="180" alt="USDT 收款码" /></td>
    </tr>
  </table>
  <p><b>USDT (TRC20) 钱包地址:</b></p>
  <code>TQhtmLB9A7xjjPzXJZv95g7XzRo5QhXFVq</code>
</div>

---

## 👤 作者与联系方式

- **开发者**：LinHouYu
- **GitHub 主页**：[@LinHouYu](https://github.com/LinHouYu)
- **Bilibili 个人空间**：[LinHouYu 的 Bilibili 空间](https://space.bilibili.com/1563740453)
- **旧版仓库归档**：[LinHouYu/cloudflared_GUI (Python 原始版)](https://github.com/LinHouYu/cloudflared_GUI)

---

## 📄 开源许可证

本项目源码基于 **[CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/deed.zh)** 协议开源。
- 允许：学习研究、代码分享、修改派生（非商业用途）。
- 禁止：商业化售卖、付费封装、商业牟利。