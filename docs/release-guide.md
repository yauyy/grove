# Grove 发布与更新指南

## 发布流程

### 第一步：修改代码并更新版本号

```bash
# 编辑 Cargo.toml 中的 version
# 例如 "0.2.0" -> "0.3.0"
```

### 第二步：提交并推送

```bash
git add .
git commit -m "feat: 改动描述"
git push origin master
```

### 第三步：打 tag 触发 Release

```bash
git tag v0.3.0
git push origin v0.3.0
```

CI 会自动完成以下所有步骤：

1. 构建 4 个平台的二进制文件
2. 创建 GitHub Release 并上传构建产物
3. 计算 sha256，生成新的 `grove.rb`
4. 推送到 `yauyy/homebrew-grove`
5. 同步更新主仓库 `Formula/grove.rb`

### （可选）发布到 crates.io

```bash
cargo publish
```

---

## CI 配置

| 文件 | 触发条件 | 作用 |
|------|----------|------|
| `.github/workflows/ci.yml` | push / PR | 自动测试（macOS/Windows/Linux） |
| `.github/workflows/release.yml` | push tag `v*` | 构建 + 发布 + 更新 Homebrew |

### Secrets 配置

| Secret | 位置 | 用途 |
|--------|------|------|
| `HOMEBREW_TAP_TOKEN` | grove 仓库 Settings → Secrets → Actions | 推送到 homebrew-grove 仓库的 Fine-grained PAT，需要 homebrew-grove 的 Contents 读写权限 |

---

## 构建产物

| 平台 | 文件名 |
| --- | --- |
| macOS Apple Silicon | `grove-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `grove-x86_64-apple-darwin.tar.gz` |
| Windows | `grove-x86_64-pc-windows-msvc.zip` |
| Linux | `grove-x86_64-unknown-linux-gnu.tar.gz` |

---

## 用户更新方式

### Homebrew 用户

```bash
brew update
brew upgrade grove
```

### Cargo 用户

```bash
cargo install grove-cli
```

### 手动下载用户

前往 https://github.com/yauyy/grove/releases 下载最新版本。

---

## 完整操作速查

```bash
# 1. 更新 Cargo.toml 版本号
# 2. 提交推送
git add .
git commit -m "feat: 改动描述"
git push origin master

# 3. 打 tag（CI 自动完成构建、发布、Homebrew 更新）
git tag vX.X.X
git push origin vX.X.X

# 4. (可选) 发布到 crates.io
cargo publish
```

---

## 目录结构参考

```
yauyy/grove                  # 主项目仓库
├── .github/workflows/
│   ├── ci.yml               # 测试 CI（push/PR 触发）
│   └── release.yml          # Release CI（tag 触发，含 Homebrew 自动更新）
├── Formula/
│   └── grove.rb             # Homebrew formula（CI 自动同步到 homebrew-grove）
├── src/                     # Rust 源码
├── Cargo.toml               # 版本号在这里管理
└── LICENSE

yauyy/homebrew-grove         # Homebrew tap 仓库
└── grove.rb                 # CI 自动从 release 构建产物生成并推送
```
