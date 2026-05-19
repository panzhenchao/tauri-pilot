# tauri-pilot 学习笔记

> 面向"整体认知"的入门读物：读完能跟别人讲清楚 tauri-pilot 是什么、解决什么问题、架构怎么落地、有哪些值得借鉴的设计取舍。所有论点都附文件路径:行号，可直接跳到源码核对。

## 目录

- [一、项目定位与能力边界](#一项目定位与能力边界)
- [二、架构与模块](#二架构与模块)
- [三、安装与使用](#三安装与使用)
- [四、技术亮点与设计取舍](#四技术亮点与设计取舍)
- [五、一句话总结](#五一句话总结)

---

## 一、项目定位与能力边界

### 1.1 解决了什么问题

Tauri v2 应用的 UI 自动化测试存在工具空白，根因是 Tauri **不使用独立浏览器进程**，而是嵌入操作系统原生 WebView：Linux 上是 WebKitGTK，macOS 上是 WebKit，Windows 上是 WebView2（`README.md:41`、`README.md:230`、`docs/src/content/docs/guides/ai-agents.md:10`）。这导致主流方案全部失效：

- **Playwright** 驱动 Chromium / Firefox / 自带的 WebKit 构建，需要"附着到一个浏览器进程"，而 Tauri 没有这样的进程可挂载（`docs/src/content/docs/guides/ai-agents.md:10`）。
- WebDriver / Selenium 同样依赖浏览器 driver 协议——README 直接指出 "There's no tool for AI agents to interact with Tauri app UIs"（`README.md:41`）。

tauri-pilot 的解法：用一个 Rust **插件**（`tauri-plugin-pilot`）嵌入到 app 内（仅 debug 构建），起一个 Unix Socket / Windows Named Pipe 服务端，注入一段 JS Bridge（`window.__PILOT__`）到 WebView 内，由独立 **CLI**（`tauri-pilot-cli`）通过 JSON-RPC 2.0 newline-delimited 协议驱动（`README.md:43-60`、`docs/src/content/docs/guides/architecture.md:23-37`）。输出形态是为 LLM 优化的**文本化 accessibility tree + 短引用 `@e1/@e2`**（`README.md:23-37`、`docs/src/content/docs/guides/ai-agents.md:11-12`）。

### 1.2 能力总览（按功能聚类）

基于 `README.md:141-179` 与 `SKILL.md:38-187` 的命令表重新聚类：

- **连通性 / 窗口**：`ping`、`windows`、`state`、`url`、`title`（多窗口支持，可通过 `--window <label>` 或环境变量 `TAURI_PILOT_WINDOW` 选择，`docs/src/content/docs/guides/architecture.md:83-87`）。
- **结构探查**：`snapshot`（支持 `-i` 仅交互元素、`-s` CSS 范围、`-d` 深度、`--save`）、`diff`（增量变更，省 token）、`text/html/value/attrs/forms`。
- **交互**：`click`、`fill`、`type`、`press`、`select`、`check`、`scroll`、`drag`、`drop`、`navigate`。
- **断言**：`assert` 一步式校验（text / visible / hidden / value / count / checked / contains / url），退出码 0/1 直接对接 shell 链与 CI（`README.md:171`、`SKILL.md:80-91`）。
- **等待 / 观察**：`wait`（appear/disappear/selector/超时）、`watch`（MutationObserver，支持 `--require-mutation`）。
- **运行时调试**：`eval`（支持 stdin heredoc、自动包裹顶层 `await`，`CHANGELOG.md:97`）、`ipc` 直接调 Tauri 命令、`screenshot`、`logs`（500 条 ring buffer）、`network`（fetch + XHR，200 条 ring buffer，`CHANGELOG.md:219-223`）、`storage`（local/session）。
- **录制 / 回放 / 场景**：`record start|stop|status`、`replay`（含 `--export sh` 导出 shell）、`run <scenario.toml>`（声明式，支持 `fail_fast`、`--junit` CI 输出、自动失败截图到 `./tauri-pilot-failures/`，`CHANGELOG.md:75`）。
- **MCP 集成**：`tauri-pilot mcp` 作为 stdio MCP server，把同一命令面暴露成结构化工具（`README.md:196-226`、`docs/src/content/docs/guides/ai-agents.md:149-182`）。

底层共 42 个 JSON-RPC method（`docs/src/content/docs/guides/architecture.md:65`），CLI 与 MCP 是同一协议的两层封装。

### 1.3 不能做什么 / 边界

- **仅 debug 构建可用**：插件代码整体在 `#[cfg(debug_assertions)]` 下编译，不会进入 release（`README.md:77-83`、`SECURITY.md:5`）。
- **仅 Tauri v2**，v1 不支持；MSRV Rust 1.95.0+，edition 2024（`README.md:228-232`）。
- **Socket 仅本机、仅当前用户**：`/tmp` 或 `$XDG_RUNTIME_DIR`，`0o600` + UID 校验；Windows 用 user-only DACL Named Pipe（`SECURITY.md:5`、`CHANGELOG.md:165-170`、`CHANGELOG.md:74`）。
- **X11 上 `press` 触发不了 `tauri-plugin-global-shortcut` 注册的全局快捷键**：`enigo` 的 `XTestFakeKeyEvent` 与 `XGrabKey` passive grab 的 modifier mask 匹配会失同步；DOM 监听器和 Tauri accelerator 仍能收到 `isTrusted=true` 事件（`README.md:236-240`、`CHANGELOG.md:96`）。Workaround：把回调封装成 `#[tauri::command]`，用 `ipc` 触发。
- **JSON-RPC eval 调用有超时**：默认 10s，需要更长时间须显式 `--timeout`（`CHANGELOG.md:31-45`）。
- **WebView 输出视为不可信数据**：`eval / html / text / attrs / value / logs / network / screenshot / storage / forms / ipc` 返回的内容可能含间接 prompt injection，需要 agent 把它当数据而非指令（`SKILL.md:144-161`、`CHANGELOG.md:48-58`）。
- **未确认**：是否支持 Tauri 多 WebView（iframe 内跨 realm）、Wayland 上 `press` 全功能（CHANGELOG 提到已启用 wayland backend，但未对 global shortcut 做与 X11 同等说明）。

### 1.4 典型场景

1. **AI agent E2E**：Claude Code 直接 shell 调用 `snapshot -i → click/fill → diff → assert`，或通过 `mcp` 子命令以结构化工具消费（`README.md:181-194`、`docs/src/content/docs/guides/ai-agents.md:125-147`）。
2. **CI 集成**：`tauri-pilot run scenario.toml --junit out.xml`，配合自动失败截图，输出 JUnit XML 给 Jenkins / GitHub Actions / Allure 直接消费（`CHANGELOG.md:75-79`，示例 `docs/examples/login-flow.toml`）。
3. **人工调试**：开发者用 `eval -`（heredoc）、`logs --level error -f`、`network --failed -f`、`screenshot` 像浏览器 DevTools 一样观察 app（`README.md:129-139`、`SKILL.md:124-141`）。
4. **录制即测试**：开发者手动操作一次，`record start/stop`，再 `replay` 或 `replay --export sh` 导出可执行脚本作为回归用例（`SKILL.md:170-176`）。
5. **多窗口 / 设置面板分别驱动**：`--window settings` 或 `TAURI_PILOT_WINDOW=settings`（`docs/src/content/docs/guides/ai-agents.md:104-123`）。

### 1.5 目标用户

- **AI agent（Claude Code 及任意 MCP-aware agent）**：首选消费者；输出本就是 a11y tree + ref，无需训练；可通过 `npx skills add` 一键安装能力（`README.md:103-107`），或直接挂 `mcp` server。
- **Tauri 应用开发者**：在 `cargo install tauri-pilot-cli` + 两行插件改动后获得交互式 REPL 风格调试（`README.md:62-95`），替代裸 DevTools 流。
- **QA / CI 工程师**：使用 `run` 子命令把 TOML 声明式场景接入流水线，无需编写 Rust 测试代码即可获得带断言、超时、JUnit、失败截图的 E2E 套件（`CHANGELOG.md:75`）。

---

## 二、架构与模块

### 2.1 Workspace 结构

`Cargo.toml` 顶层声明只有两个成员（`Cargo.toml:3`，`resolver = "2"`）：

- `crates/tauri-pilot-cli` — 二进制 crate，bin 名为 `tauri-pilot`（`crates/tauri-pilot-cli/Cargo.toml:11-13`）。依赖 `clap` 解析参数、`tokio` 异步 IO、`rmcp` 提供 MCP server transport、`anyhow`/`tracing-subscriber` 处理诊断。模块切分见 `crates/tauri-pilot-cli/src/main.rs:1-7`：`cli`、`client`、`mcp`、`output`、`protocol`、`scenario`、`style`。
- `crates/tauri-plugin-pilot` — Tauri 插件 lib crate，注册名 `tauri-plugin-pilot`（`crates/tauri-plugin-pilot/Cargo.toml:14`）。依赖 `tauri`（`unstable` feature）、`tokio` IO、可选 `enigo`（feature `press`，`Cargo.toml:21-26`）。模块拆为 `diff`、`eval`、`handler`、`key`、`protocol`、`recorder`、`server`（`crates/tauri-plugin-pilot/src/lib.rs:1-12`）。

整条链路只有这两个 crate；CLI 与插件各自维护一份 `protocol.rs`（请求/响应类型），通过 JSON-RPC over 字节流耦合，而不是共享 crate（`crates/tauri-pilot-cli/src/protocol.rs:15-43` 对比 `crates/tauri-plugin-pilot/src/protocol.rs:15-43`）。

### 2.2 进程与通信模型

插件只在 debug build + (unix|windows) 下激活；release build 退化为空插件（`crates/tauri-plugin-pilot/src/lib.rs:42-46`）。激活路径在 `init()` 内做四件事（`lib.rs:48-83`）：

1. `js_init_script(BRIDGE_JS)` 把 `html-to-image.iife.js` + `bridge.js` 拼接后挂为每页 init script（`lib.rs:27-32`）。
2. 构造 `EvalEngine` 并 `app.manage` 注入 Tauri state（`lib.rs:53-54`）。
3. 计算 socket 路径（Unix 端为 `$XDG_RUNTIME_DIR/tauri-pilot-{identifier}.sock`，回落 `/tmp`，见 `server/unix.rs:62-86`；Windows 端为 `\\.\pipe\tauri-pilot-{identifier}`，见 `server/windows.rs:27-29`）。
4. `server::bind` 同步绑定（避开 tokio runtime 限制），随后 `async_runtime::spawn(server::run)` 异步驱动 accept loop。

`tauri-pilot click @e3` 端到端调用链：

```
shell  $ tauri-pilot click @e3
  │
  ▼  main.rs:62  resolve_socket()           — 扫 XDG/instances 找最新 socket
  │
  ▼  cli.rs:297  parse_target("@e3") → Target::Ref("e3")
  ▼  main.rs:930 target_params → {"ref":"e3"}
  ▼  client/mod.rs:34 Client::call("click", {ref:"e3"})
  │   ──写 {"jsonrpc":"2.0","id":N,"method":"click","params":{"ref":"e3"}}\n──▶
  │
  ▼  server/{unix,windows}.rs accept_loop → handle_connection
  ▼  server/mod.rs:71 解析 Request → dispatch_request → handler::dispatch
  ▼  handler.rs:136-140 命中 "click" 分支 → handle_eval_method
  ▼  handler.rs:488 build_bridge_call("click", …) → "window.__PILOT__.click({\"ref\":\"e3\"})"
  ▼  EvalEngine::register() 拿 id, oneshot::Receiver
  ▼  EvalEngine::wrap_script (eval.rs:100) 包成 try/catch + invoke('plugin:pilot|__callback',…)
  ▼  eval_fn(window=None, wrapped)  — webview.eval (lib.rs:113-131)
  │
  ▼  bridge.js:459 click(params) → resolveTarget → 派发 pointerdown/mousedown/…/click
  ▼  bridge.js 末尾 → __TAURI_INTERNALS__.invoke('plugin:pilot|__callback',{id, result})
  ▼  handler.rs:546 __callback (#[tauri::command]) → handle_callback → engine.resolve(id, …)
  ▼  oneshot 唤醒 → handler 返回 Response::success → 写回 socket
  │   ◀─{"jsonrpc":"2.0","id":N,"result":{"ok":true}}\n─
  ▼  client/mod.rs:54-75 读 line → 反序列化 Response → 返回 Value
  ▼  main.rs output::format_* 打印 → 进程退出码
```

### 2.3 关键模块剖析

- **CLI 命令解析**：`cli.rs` 用 `clap::Subcommand` 定义 30+ 子命令（`cli.rs:23-120`）。全局 flag `--socket`/`--window`/`--json` 通过 `global=true` 渗透到所有子命令（`cli.rs:8-17`）。`Target` 三态枚举 + `parse_target`（`cli.rs:288-309`）实现 `@ref` / `"x,y"` / CSS selector 的统一寻址语法。
- **CLI Client**：`client/mod.rs` 用 `#[cfg(unix)]` / `#[cfg(windows)]` 选择 `UnixStream` 或 `NamedPipeClient`（`client/mod.rs:9-17`），`call()` 同步写 NDJSON、读单行响应，并校验 `id` 匹配（`client/mod.rs:34-76`）。`#[cfg(unix)]` 下 `resolve_socket` 扫 `$XDG_RUNTIME_DIR` 与 `/tmp` 拿最新 `.sock`；Windows 下读 `%LOCALAPPDATA%\tauri-pilot\instances\*.json`，按 `created_at` 选最新且 `pid` 仍存活的实例（`main.rs:1356-1469`）。
- **Plugin server**：`server/mod.rs:22-88` 是平台中立的 `handle_connection`：BufReader 逐行读、`take(MAX_LINE_LENGTH+1)` 防 OOM、违例时回 `-32700`。Unix 侧 `server/unix.rs:94-150` 用 std listener bind（umask `0o177` → 文件 mode `0o600`），`SocketGuard` 记录 inode、Drop 时仅在 inode 一致时 `unlink`；accept 后 `peer_cred().uid()` 校验同用户（`server/unix.rs:194-212`）。Windows 侧 `server/windows.rs:27-80` 创建 Named Pipe 时显式拼 DACL 限制为当前用户，并把 `pid + pipe + created_at` 写到 `instances/{identifier}.json` 供 CLI 发现。
- **Dispatch / Eval**：`handler.rs:86-243` 是大 `match`，把 method 字符串映射到 `handle_eval_method`（绝大多数）/`handle_diff`/`handle_press` 等。`handle_eval_method`（`handler.rs:447-484`）走 ADR-001 模式：`build_bridge_call` 生成 `window.__PILOT__.<m>(<json>)` 字符串（`handler.rs:488-512`，`ipc` 是特例直调 `__TAURI_INTERNALS__.invoke`），`EvalEngine::wrap_script` 包 try/await/`plugin:pilot|__callback`（`eval.rs:100-108`），通过 `webview.eval` 注入；`EvalEngine` 维护 `HashMap<u64, oneshot::Sender>` 与 `tokio::time::timeout`，回调命中即 resolve（`eval.rs:62-139`），`__callback` 是 `#[tauri::command]`（`handler.rs:541-553`）。
- **JS Bridge**（`crates/tauri-plugin-pilot/js/bridge.js`，IIFE 自执行）：暴露 `window.__PILOT__` 一张对象表（`bridge.js:1178-1213`），涵盖 `snapshot/click/fill/type/select/scroll/drag/drop/text/html/value/attrs/eval/wait/screenshot/consoleLogs/networkRequests/storage*/formDump/watch`。`snapshot` 按 ARIA role 遍历 DOM，自增 `e1/e2…` ref 并把 DOM 节点存入 `idMap`（`bridge.js:342-408`）；`resolveTarget` 支持 `{ref}` / `{selector}` / `{x,y}` 三种寻址（`bridge.js:416-429`）；`click` 显式按 pointerdown → mousedown → pointerup → mouseup → click 序列派发（`bridge.js:459-501`，配套 `lib.rs:209-251` 测试守护顺序）。`fill/typeText/select` 用 `nativeValueSetter` 抓取**元素自身原型**上的 `value` setter 以绕过 React 等框架的实例覆盖（`bridge.js:508-543`）。屏幕截图依赖前置 vendored 的 `html-to-image.iife.js`，因此插件强制其在 `bridge.js` 之前注入。
- **MCP server**：`tauri-pilot mcp` 走 `main.rs:42-44` → `mcp::run_mcp_server`，用 `rmcp::transport::stdio` 起 server（`mcp.rs:33-43`）。`PilotMcpServer::call_tool_by_name` 对每个工具名做 schema 校验后转调 `call_app_tool` → `call_app` → `Client::connect`+`Client::call`，即每次工具调用都现连一次 socket/pipe（`mcp.rs:84-119`、`mcp.rs:122-200`）。复用了 CLI 的 `target_params`、`with_window`、`build_wait_params`、`resolve_socket` 等帮助函数。
- **Recorder**：`recorder.rs` 提供 `record.start/stop/status/add` 四个 RPC，状态用 `Arc<Mutex<RecorderState>>` 持有（`recorder.rs:15-26`）；`handler::dispatch` 在调用成功后自动 `recorder.record(method, original_params)`（`handler.rs:237-240`），用于 scenario replay。

### 2.4 协议设计要点

- **JSON-RPC 2.0 形态**：纯 NDJSON（每行一个对象，`\n` 分隔），无 batch、无 notification —— `Request.id` 是 `u64`（`crates/tauri-plugin-pilot/src/protocol.rs:17-23`），响应 `id` 是 `Value`（兼容服务器对解析失败时返回 `null`，`protocol.rs:54-70`）。`params` 用自定义 `deserialize_params` 把显式 `null` 归一为 `None`（`protocol.rs:4-13`）。错误码沿用规范：`-32700` 解析错误、`-32600` 版本错、`-32601` 未知方法、`-32602` 参数无效、`-32603` 内部错（散见 `server/mod.rs:53-78`、`handler.rs:111-117`、`handler.rs:230-234`）。
- **Refs 体系**：`@eN` 风格的引用号完全产生于 JS 端 —— `snapshot()` 在遍历时自增 `refCounter` 并写入模块级 `idMap: Map<string, Element>`（`bridge.js:347-381`），所有后续交互通过 `resolveTarget({ref})` 反查（`bridge.js:416-417`）。CLI 侧 `Target::Ref` 仅做字符串透传（`cli.rs:289-308`、`main.rs:930-936`）。**含义**：refs 是会话相关、依附于最近一次 `snapshot` 的句柄；任何 `snapshot()` 调用都会重置 `refCounter` 与 `idMap`。`idMap` 是 bridge IIFE 内的单例，所以同一 webview 内全局共享。
- **跨平台 socket 抽象**：在 `server/mod.rs:119-127` 与 `client/mod.rs:9-31` 各自用 `#[cfg(unix)]` / `#[cfg(windows)]` 分流到 `unix.rs` / `windows.rs`；二者对外都提供 `socket_path(identifier) → PathBuf`、`bind(&Path) -> (Listener, Guard)`、`run(...)` 三个相同签名的函数。差异：Unix 用 socket 文件 + `peer_cred().uid()` 鉴权，Windows 用 Named Pipe + DACL + `instances/*.json` 注册表（pipe 名空间是平坦的，需要外置 registry 配合 PID 存活检测来淘汰 stale 实例）。
- **Eval/回调通道**：响应不直接来自 `webview.eval` 的同步返回（Tauri eval 是 fire-and-forget），而是由注入脚本通过 `__TAURI_INTERNALS__.invoke('plugin:pilot|__callback', {id, result|error})` 反向投递到 Tauri IPC 命令 `__callback`，再由 `EvalEngine.resolve` 唤醒等待的 oneshot —— 注释里反复提到的 "ADR-001 pattern"（`eval.rs:21-25`、`handler.rs:514-553`）。Rust 侧 `wait` 包了一层 `tokio::time::timeout`，默认 10s，`screenshot` 30s，`wait`/`watch` 还会把 JS 端 `options.timeout` 加 2s buffer（`handler.rs:28-57`）。

---

## 三、安装与使用

### 3.1 环境要求

- **Rust 1.95.0+**（edition 2024，整个 workspace 在 `Cargo.toml:8` 声明 `rust-version = "1.95.0"`）
- **Tauri v2**（v1 不支持）
- **平台**：Linux（WebKitGTK）、macOS（WebKit）、Windows（WebView2）
- 仓库未固定 `rust-toolchain.toml`，本地 toolchain 满足 1.95+ 即可

### 3.2 把插件集成到你的 Tauri app

在 `src-tauri/Cargo.toml` 加依赖（见 `README.md:67`、`docs/src/content/docs/guides/plugin-setup.md:12`）：

```toml
[dependencies]
tauri-plugin-pilot = { git = "https://github.com/mpiton/tauri-pilot" }
```

在 `src-tauri/src/main.rs` 注册插件，必须用 `#[cfg(debug_assertions)]` 包裹：

```rust
fn main() {
    let mut builder = tauri::Builder::default();

    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(tauri_plugin_pilot::init());
    }

    builder.run(tauri::generate_context!()).expect("error running app");
}
```

**为什么 debug-only**：插件只在 debug 构建里编译进二进制，`cargo build --release` 完全不包含它，生产零开销、不需要发版前剥离（`docs/src/content/docs/guides/plugin-setup.md:38-43`）。

**capability 必加**：`pilot:default` 在 `crates/tauri-plugin-pilot/permissions/default.toml` 定义，只授权一个内部 IPC 命令 `__callback`（eval 引擎用它把 WebView 里的执行结果回传给 Rust）。在 `src-tauri/capabilities/default.json` 加上：

```json
{ "permissions": ["core:default", "pilot:default"] }
```

漏了这条，`eval` 会以 `eval timed out after 10s` 失败（`README.md:95`）。

插件启动后会在 `/tmp/tauri-pilot-{identifier}.sock` 监听（`identifier` 来自 `tauri.conf.json`），CLI 默认按 mtime 选最新的一个 sock。

### 3.3 安装 CLI

发布到 crates.io 的包名就是 `tauri-pilot-cli`，但二进制名是 `tauri-pilot`（见 `crates/tauri-pilot-cli/Cargo.toml:15-17` 的 `[[bin]] name = "tauri-pilot"`）：

```bash
cargo install tauri-pilot-cli
```

本地开发可以 `cargo install --path crates/tauri-pilot-cli`。

### 3.4 常用工作流

**单步交互 + 断言**（snapshot → click → assert，refs 每次 snapshot 重置）：

```bash
tauri-pilot ping
tauri-pilot snapshot -i
tauri-pilot fill @e2 "workspace"
tauri-pilot click @e3
tauri-pilot wait --selector ".success-message"
tauri-pilot assert text @e1 "Dashboard"
tauri-pilot assert url "/dashboard"
```

`assert` 退出码 0=通过、1=失败，省掉 `text @ref` + 解析 + 比对的三步往返（`SKILL.md:91`）。

**录制回放**（`docs/.../reference/cli.md:1203-1247`）：

```bash
tauri-pilot record start
tauri-pilot click @e3
tauri-pilot fill @e2 "test"
tauri-pilot record stop --output test.json
tauri-pilot replay test.json
tauri-pilot replay test.json --export sh   # 导出为可独立运行的 shell 脚本
```

**声明式 TOML 场景**（完整样例见 `docs/examples/login-flow.toml`）：

```toml
[scenario]
name = "Login flow"
fail_fast = true
global_timeout_ms = 60000

[[step]]
name = "open login page"
action = "navigate"
url = "http://localhost:1420/login"

[[step]]
name = "fill email"
action = "fill"
target = "#email"
value = "user@example.com"

[[step]]
name = "assert dashboard url"
action = "assert-url"
expected = "/dashboard"
```

执行：`tauri-pilot run docs/examples/login-flow.toml`。失败截图自动落到 `./tauri-pilot-failures/`（`SKILL.md:186`）。

**MCP 模式**（给原生支持 MCP 的 agent 用，长连接复用、结构化返回）：

```bash
tauri-pilot mcp                                                    # stdio MCP server
tauri-pilot --socket /tmp/tauri-pilot-myapp.sock --window main mcp # 指定 app 和窗口
```

注意全局 flag 必须写在 `mcp` 子命令前（`docs/.../reference/cli.md:97-104`）。

**eval 多行脚本**（用单引号 heredoc 关掉 shell 展开，`$` 和反引号不用转义）：

```bash
tauri-pilot eval - <<'EOF'
const els = document.querySelectorAll('button');
return els.length;
EOF
```

多语句脚本必须用 `return` 显式回值，否则结果是 `null`（`docs/.../reference/cli.md:785-794`）。Top-level `await` 会被自动包成 async IIFE。

### 3.5 AI agent 集成

两条路径，按 agent 类型选：

- **Claude Code skill**：`npx skills add https://github.com/mpiton/tauri-pilot`（`README.md:106`）。注册仓库根的 `SKILL.md` 进 Claude Code，里面写好了 workflow 守则（"snapshot 前永远先 ping"、"refs 每次 snapshot 重置"、"`logs --level error` 收尾检查"）、targeting 三态（`@ref` / CSS / `x,y`）、记录凭据安全注意事项，以及对 WebView 返回内容当数据不当指令的反 prompt-injection 提示。适合 Claude Code 这种直接以 shell 调 CLI 的 agent。
- **MCP server**：`tauri-pilot mcp` 起 stdio MCP server，把 `snapshot`/`click`/`fill`/`eval`/`assert_*` 等暴露为结构化工具调用，返回 JSON 而不是要解析终端文本。适合原生 MCP 客户端（Cursor、Claude Desktop 等）。Server 启动时不需要 Tauri app 在跑，每次工具调用才懒连接 socket（`docs/.../reference/cli.md:113-115`）。

### 3.6 CI 与调试

**CI**：`tauri-pilot run scenario.toml --junit results.xml` 输出 JUnit XML（`SKILL.md:184`、`crates/tauri-pilot-cli/Cargo.toml:35` 依赖 `quick-xml`）。退出码全局约定：0=通过，1=任意失败；`--no-fail-fast` 让单步失败后继续跑完。`assert` 子命令同样遵循 0/1 退出码，可以直接串在 `&&` 后面。

**调试**：

- 日志：CLI 用 `tracing-subscriber` 的 `EnvFilter`，默认 `warn`，通过 `RUST_LOG` 调整（`crates/tauri-pilot-cli/src/main.rs:31-32`），例 `RUST_LOG=debug tauri-pilot ping`。
- WebView 侧 console：`tauri-pilot logs --level error`、`tauri-pilot logs -f --json` NDJSON 流给 jq。
- 网络：`tauri-pilot network --failed`、`--filter graphql`、`-f` follow。两者都是 ring buffer（logs 500 条、network 200 条），`--clear` 清空。
- 直连 socket 调试：`echo '{"jsonrpc":"2.0","id":1,"method":"ping","params":{}}' | socat - UNIX-CONNECT:/tmp/tauri-pilot-com.myapp.sock`（`docs/.../reference/cli.md:1293`）。

---

## 四、技术亮点与设计取舍

### 4.1 为 LLM 优化的协议（snapshot / diff / assert / refs）

tauri-pilot 没有把完整 DOM 序列化给 agent，而是发明了一套围绕"可达性树 + 短 ref"的协议，三处证据共同体现 token 经济性这一设计动机：

- **a11y 树代替 DOM**：`snapshot()` 用一张 `ROLE_MAP`（`crates/tauri-plugin-pilot/js/bridge.js:15-40`）把 `<button>`/`<a>`/`<h1>` 等隐式角色映射成 ARIA 角色，并通过 `walk()` 递归只输出有 `role` 的节点；交互模式（`interactive: true`，`bridge.js:343, 369-376`）进一步剪枝到 `INTERACTIVE_ROLES` 集合（`bridge.js:42-51`）。可推测的动机是：a11y 子集既保留了语义又把节点数压到一两位数量级。
- **稳定短 ref + Map 持有真节点**：`refCounter` 从 1 递增生成 `e1/e2/…`，引用的 DOM 节点存进 JS 端的 `idMap`（`bridge.js:4-5, 347-381, 410-414`），返回给 CLI 的只是字符串。Rust 端的格式化（`crates/tauri-pilot-cli/src/output.rs:58-106`）输出 `- button "Refresh" [ref=e3]` 这样紧凑的一行。文档明确说明 "Refs are reset on each new snapshot"（`docs/src/content/docs/guides/architecture.md:71-73`），把生命周期收窄到"一次快照内"，避免了维护跨调用 ID 池的复杂度——代价是 agent 必须先 snapshot 再操作。
- **`diff` 只回传变化**：`compute_diff`（`crates/tauri-plugin-pilot/src/diff.rs:58-138`）用 `(role, name, depth)` 作匹配键（`diff.rs:36-51`），输出 `added / removed / changed` 三段；`changed` 只列出实际变动字段（`value` / `checked` / `disabled`，`diff.rs:87-95`）。配合 `EvalEngine::store_snapshot`（`crates/tauri-plugin-pilot/src/eval.rs:46-59`），diff 是相对服务端缓存计算的，CLI 不需要再传一次完整快照。
- **`assert` 一步式断言**：`AssertKind`（`crates/tauri-pilot-cli/src/cli.rs:227-244`）把 `text / visible / hidden / value / count / checked / contains / url` 包成 CLI 子命令，由 `run_assert_command`（`crates/tauri-pilot-cli/src/main.rs:656-748`）在本地比较后用 `assert_fail` 决定 exit code。`README.md:192` 明确写道这是为了 "replace the manual `text @ref` + parse + compare pattern, reducing round-trips and token usage" —— 属于明确写在文档里的设计意图。

### 4.2 跨平台 WebView 与 IPC 抽象

WebView 三套引擎（WebKitGTK / WebKit / WebView2）通过 Tauri 统一抽象，再通过 `js_init_script()` 把同一份 `bridge.js` 注入到所有 WebView（`crates/tauri-plugin-pilot/src/lib.rs:28-32, 51`，使用 `include_str!` 把 JS 编进 Rust 二进制）。IPC 自己实现了一层薄抽象：

- `crates/tauri-plugin-pilot/src/server/mod.rs:119-127` 用 `#[cfg(unix)] / #[cfg(windows)]` 切换 `unix.rs` 的 `UnixListener`（基于 `tokio::net::UnixListener`，`unix.rs:9-10`）与 `windows.rs` 的 `NamedPipeServer`（`tokio::net::windows::named_pipe`，`windows.rs:15`）。
- 共用的 `handle_connection`（`server/mod.rs:22-88`）是泛型的 `S: AsyncRead + AsyncWrite + Unpin`，所以协议层（行分隔 JSON-RPC、1 MiB 限长、错误码 -32700 / -32600 / -32601）只写一次。这是看到的实现，没有依赖 `interprocess` 这类跨平台库——可推测的取舍是：少一个依赖、对每个平台的安全模型保持完全控制。

### 4.3 安全与隔离（debug-only + capability + 同用户校验）

- **仅 debug 构建启用**：插件主体被 `#[cfg(all(any(unix, windows), debug_assertions))]` 包裹（`crates/tauri-plugin-pilot/src/lib.rs:27, 43-48`），release 构建返回空 `Builder::new("pilot").build()` —— 把"打开远程注入接口"这件事从源头消除。`README.md:77-79` 也要求用户自己用 `#[cfg(debug_assertions)]` 包裹 `.plugin(...)`，形成双重防线。
- **`pilot:default` capability**：通过 `permissions/default.toml` 仅放行 `__callback` 命令（`crates/tauri-plugin-pilot/permissions/default.toml`），`README.md:86-95` 解释了不加这条权限会出现 "eval timed out after 10s" —— eval 路径完全依赖 Tauri 的能力系统授权。
- **socket 路径与权限**：Unix 端优先 `$XDG_RUNTIME_DIR`，并强制验证目录归属当前用户且无 group/world 位（`server/unix.rs:48-58, 79-81`）；socket 文件用 `umask(0o177)` 创建并显式 `chmod 0o600`（`unix.rs:97-101, 135`）；`accept_loop` 还用 `peer_cred()` 拒绝异 UID 连接（`unix.rs:195-212`）；`SocketGuard` 在 drop 时通过 inode 比对（`unix.rs:14-31`）只 unlink 自己的 socket。Windows 端构造 user-only DACL + `ImpersonateNamedPipeClient` 校验 client SID（`server/windows.rs:236-291, 296-389`），且明确"宁可断连接也不降级到默认 DACL"（`windows.rs:393-407`）。

### 4.4 稳健性（超时、等待、observer）

- **双层超时 + 缓冲**：Rust 端 `DEFAULT_TIMEOUT = 10s`、`SCREENSHOT_TIMEOUT = 30s`，`wait`/`watch` 用 `bridge_eval_timeout()`（`crates/tauri-plugin-pilot/src/handler.rs:28-56`）把 JS 端 `options.timeout` 加 2 秒 `BRIDGE_TIMEOUT_BUFFER_MS` 再传给 oneshot channel —— 确保 JS 侧自己的超时消息能先回流，而不是被 Rust 通道粗暴截断。`CHANGELOG.md:29-45` 记录了 #91 修复就是因为 `wait` 没走这条路径，造成"用户 timeout > 10s 反被 Rust 通道截成 `eval timed out`"。
- **`waitFor` / `watch` 用 MutationObserver**：`bridge.js:867-918`（waitFor）和 `bridge.js:937-1000`（watch）都通过 `MutationObserver` 监听 `childList + subtree + attributes`，避免 polling；`watch` 还支持 `stable` 静默时间（`bridge.js:940, 965-968`），等 DOM 不再变动后再 settle —— 录制/回放的稳定性主要靠它。
- **eval callback 模式（ADR-001）**：`webview.eval()` 是 fire-and-forget，作者用 `EvalEngine` 维护 `HashMap<u64, oneshot::Sender>`（`eval.rs:19-30, 62-69`），脚本被 `wrap_script()` 包成 try/catch + `__TAURI_INTERNALS__.invoke('plugin:pilot|__callback', {id, result})`（`eval.rs:99-108`），超时路径会清理 pending map（`eval.rs:130-137`）防内存泄漏。
- **三段式 eval 兼容 top-level await**：bridge.js 中的 `evalScript` 依次尝试"表达式 → async 表达式 → async-IIFE → 间接 eval"四档（`crates/tauri-plugin-pilot/src/lib.rs:340-367` 的测试固化了顺序），并在 `hasTopLevelAwait` 失败时给出明确错误（`lib.rs:330-333`）。

### 4.5 工程文化亮点

- **诚实文档化限制**：`README.md:234-240` 用整段篇幅解释 X11 + `tauri-plugin-global-shortcut` 为什么不可靠，给出 `XGrabKey` passive grab 与 `XTestFakeKeyEvent` 调用顺序的细节，并在源码里再复述一遍（`crates/tauri-plugin-pilot/src/key.rs:1-27`、`handler.rs:347-349`）—— 同时附上 workaround（用 `tauri-pilot ipc <command>` 代替）。这种"先告诉你什么不能做、再给变通路径"的姿态在开源项目里并不常见。
- **对外友好接口**：除了 CLI，同一组能力还以 MCP stdio server 形式暴露（`crates/tauri-pilot-cli/src/mcp.rs`、`README.md:196-225`），让原生支持 MCP 的 agent 不用 fork 进程；TOML 场景 + `--junit` 输出（`crates/tauri-pilot-cli/src/scenario.rs:531-611`、`main.rs:1303-1346`）则把测试结果直接对接 Jenkins/GitHub Actions/Allure。
- **测试固化 bug 故事**：`lib.rs` 里大量"断言 bridge.js 字面文本"的 unit test（`lib.rs:208-461`）实际上是 #79、#85 等历史 bug 的回归挡板 —— 选用断言子串而非 AST，可推测是为了让 `rustfmt`/`prettier` 重排不会破坏检查、同时让 reviewer 一眼看出"这里改了会回归哪个 issue"。

### 4.6 可借鉴实践 5 条

1. **协议为消费方设计**：a11y 树 + 短 ref + 一步 assert + 增量 diff 都是围绕"被 LLM 消费"的具体推导，普适规则是"先想清楚下游谁在读、再决定上游传什么"。
2. **调试能力收进 debug-only 编译开关**：把强力但危险的内省能力用 `cfg(debug_assertions)` + 用户侧 capability + OS 级权限做三层把关，比"加个环境变量开关"鲁棒得多。
3. **统一抽象只到值得抽象的层**：跨平台 IPC 用 `cfg` 切两份实现 + 协议泛型化，没引入 `interprocess` 这类通用库 —— 平台特定的安全语义（peer cred / DACL / impersonation）反而能完整保留。
4. **双层超时 + 缓冲带**：当你有 outer（Rust 通道）和 inner（JS 业务）两个超时，inner 必须比 outer 短，留出缓冲让"业务自身的错误信息"有机会先回流。
5. **把 bug 写进测试**：用注释和断言把 issue 编号、错误信息、stage 顺序"焊死"进 unit test，未来重构者读 test 就能复盘原始事故。

---

## 五、一句话总结

> **tauri-pilot = 给 Tauri v2 应用补上的"AI 友好版 Playwright"。** 它把"独立浏览器进程缺失"这一根本约束转成"插件 + Unix Socket/Named Pipe + 注入式 JS Bridge"的三件套，再用 a11y 树 + 短 ref + 一步式 assert + 增量 diff 把协议本身设计成对 LLM token 友好的形态，最后用 `cfg(debug_assertions)` + capability + 同用户 socket 把"远程注入"这个危险能力锁在 debug 构建里。值得抄的不是某一行代码，而是它整套"先想清楚谁来用，再决定怎么传"的工程姿态。
