# 开启 GitHub Pages（在线演示必做）

README 里的 `reed-soul.github.io` 链接 **只有在你开启 Pages 之后** 才能访问。  
CI 已经会把演示站发布到 `gh-pages` 分支，但 GitHub **默认不会**自动对外提供网站。

## 为什么 jsDelivr 链接不能用？

`https://cdn.jsdelivr.net/gh/.../examples/index.html` 会把 HTML 当成 **纯文本**（`Content-Type: text/plain`），浏览器只会显示源码，**WASM 不会运行**。  
jsDelivr 只适合托管 `.js` / `.wasm` 资源，**不能**当作演示站首页。

## 操作步骤（约 1 分钟）

1. 用仓库 **Owner** 账号打开：  
   **https://github.com/reed-soul/wasm-spatial-core/settings/pages**

2. **Build and deployment** — 任选一种（不要混用）：

   **方式 A（推荐）**  
   - **Source**：**Deploy from a branch**  
   - Branch：**`gh-pages`**，文件夹：**`/ (root)`**

   **方式 B**  
   - **Source**：**GitHub Actions**  
   - 保存后，每次 push `master` 由 workflow **Deploy via GitHub Actions** 发布

3. 点 **Save**

5. 等 1～5 分钟，访问：  
   **https://reed-soul.github.io/wasm-spatial-core/examples/index.html**

6. 页面应出现深色界面；点 **Run Quick Start Demo**，应显示 `WASM loaded successfully`。

## 若仍是 404

- 确认 **Actions** 里最近一次 **Deploy Demo to GitHub Pages** 已成功  
- 确认 **Code** 页有 **`gh-pages`** 分支  
- 仓库 **Settings → General → Visibility** 需为 **Public**（私有仓 Pages 需 GitHub Pro）  
- 组织仓库需组织管理员允许 Pages

## 暂时无法开 Pages 时

本地体验（与线上一致）：

```bash
git clone https://github.com/reed-soul/wasm-spatial-core.git
cd wasm-spatial-core
npm run demo
```

手机与电脑同一 WiFi 下，用电脑 IP 访问 `http://<你的电脑IP>:8080/examples/index.html`。
